//! Behaviour of the team runtime: identity and lifecycle, delegation bounds,
//! memory isolation and continuity, tool authorization, approvals, routines,
//! restart and cancellation, failures and timeouts, offline operation.

use james_agents::identity::{AgentProfile, Capability};
use james_agents::journal::{Draft, Journal};
use james_agents::memory::Scope;
use james_agents::model::{CancelSignal, ModelPort, ModelReply, ModelRequest};
use james_agents::protocol::Observation;
use james_agents::runtime::{Activity, Limits, TaskState, Turn};
use james_agents::{Generation, LocalModel, Runtime, ScriptedModel, StepOutcome};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

const T0: i64 = 1_794_830_400_000; // 2026-11-16T12:00:00Z

fn tempdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("james-agents-rt-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn workspace(tag: &str) -> PathBuf {
    let ws = tempdir(&format!("{tag}-ws"));
    fs::create_dir_all(ws.join("docs")).unwrap();
    fs::write(
        ws.join("docs/setup.md"),
        "# Setup\nInstall Rust 1.88.\nRun cargo test.\n",
    )
    .unwrap();
    ws
}

fn run_until_idle(rt: &mut Runtime, model: &mut dyn ModelPort, now: i64, max: usize) -> usize {
    for i in 0..max {
        if rt.step(model, now).unwrap() == StepOutcome::Idle {
            return i;
        }
    }
    panic!("did not settle in {max} steps");
}

fn observations(rt: &Runtime, task: &str) -> Vec<Observation> {
    rt.state().tasks[task]
        .transcript
        .iter()
        .filter_map(|t| match t {
            Turn::Observation(o) => Some(o.clone()),
            _ => None,
        })
        .collect()
}

fn refusals(rt: &Runtime, task: &str) -> Vec<String> {
    observations(rt, task)
        .into_iter()
        .filter_map(|o| match o {
            Observation::Refused { reason } => Some(reason),
            _ => None,
        })
        .collect()
}

#[test]
fn identities_persist_across_reopen_without_duplication() {
    let dir = tempdir("identity");
    let (rt, _) = Runtime::open(&dir, None, T0).unwrap();
    let ids: Vec<String> = rt.state().agents.keys().cloned().collect();
    assert_eq!(ids, ["coordinator", "researcher", "reviewer", "writer"]);
    let registered = rt
        .events()
        .iter()
        .filter(|e| e.kind == "agent.registered")
        .count();
    drop(rt);
    let (rt, recovery) = Runtime::open(&dir, None, T0 + 1000).unwrap();
    assert_eq!(rt.state().agents.keys().cloned().collect::<Vec<_>>(), ids);
    assert_eq!(
        rt.events()
            .iter()
            .filter(|e| e.kind == "agent.registered")
            .count(),
        registered
    );
    assert!(recovery.journal.records > 0);
    rt.verify().unwrap();
}

#[test]
fn a_revision_is_a_new_record_never_an_edit() {
    let mut rt = Runtime::in_memory(None, T0).unwrap();
    let mut writer = rt.state().agents["writer"].clone();
    writer.role = "Drafts release notes".into();
    rt.register(writer, T0 + 1).unwrap();
    assert_eq!(rt.state().agents["writer"].revision, 2);
    let records = rt
        .events()
        .iter()
        .filter(|e| e.kind == "agent.registered" && e.data["identity"]["agent_id"] == "writer")
        .count();
    assert_eq!(records, 2);
}

#[test]
fn delegation_is_bounded_by_capability_cycles_depth_and_budget() {
    let mut rt = Runtime::in_memory(None, T0).unwrap();
    // A second delegator so a cycle can be attempted.
    let mut planner = rt.state().agents["coordinator"].clone();
    planner.id = "planner".into();
    planner.display_name = "Planner".into();
    rt.register(planner, T0).unwrap();
    let root = rt.submit("Plan something.", "coordinator", T0).unwrap();
    let mut model = ScriptedModel::queue(vec![
        (
            "coordinator",
            r#"{"action":"delegate","to":"coordinator","objective":"me"}"#,
        ),
        (
            "coordinator",
            r#"{"action":"delegate","to":"nobody","objective":"x"}"#,
        ),
        (
            "coordinator",
            r#"{"action":"delegate","to":"planner","objective":"Plan part one.","max_steps":500}"#,
        ),
        (
            "coordinator",
            r#"{"action":"delegate","to":"planner","objective":"Plan part one.","max_steps":10}"#,
        ),
        (
            "planner",
            r#"{"action":"delegate","to":"coordinator","objective":"Back to you."}"#,
        ),
        (
            "planner",
            r#"{"action":"delegate","to":"researcher","objective":"Read."}"#,
        ),
        (
            "researcher",
            r#"{"action":"delegate","to":"writer","objective":"Write."}"#,
        ),
        ("researcher", r#"{"action":"finish","result":"read it"}"#),
        ("planner", r#"{"action":"finish","result":"planned"}"#),
        ("coordinator", r#"{"action":"finish","result":"done"}"#),
    ]);
    run_until_idle(&mut rt, &mut model, T0, 40);
    let r = refusals(&rt, &root);
    assert!(r[0].contains("yourself"), "{r:?}");
    assert!(r[1].contains("no assistant"), "{r:?}");
    assert!(r[2].contains("steps left"), "{r:?}");
    let child = rt.state().tasks[&root].children[0].clone();
    let planner_refusals = refusals(&rt, &child);
    assert!(
        planner_refusals[0].contains("cycle"),
        "{planner_refusals:?}"
    );
    let grandchild = rt.state().tasks[&child].children[0].clone();
    // The researcher has no delegate capability; depth is also exhausted there.
    assert!(refusals(&rt, &grandchild)[0].contains("cannot hand work"));
    assert_eq!(rt.state().tasks[&grandchild].budget.max_depth, 0);
    let root_task = &rt.state().tasks[&root];
    assert_eq!(root_task.state, TaskState::Succeeded);
    assert_eq!(root_task.reserved, 0, "a finished child's share returns");
    assert!(root_task.steps_used <= root_task.budget.max_steps);
    assert_eq!(rt.state().tasks[&child].budget.max_steps, 10);
}

#[test]
fn memory_is_scoped_and_carries_host_derived_provenance() {
    let ws = workspace("memory");
    let mut rt = Runtime::in_memory(Some(&ws), T0).unwrap();
    rt.remember(
        Scope::Agent("writer".into()),
        "Writer-only preference: British spelling.",
        T0,
    )
    .unwrap();
    rt.remember(Scope::Team, "Team fact: the project is James.", T0)
        .unwrap();
    let root = rt.submit("Learn the setup.", "researcher", T0).unwrap();
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let log = seen.clone();
    let mut model = ScriptedModel::new("memory probe", move |req: &ModelRequest| {
        log.lock()
            .unwrap()
            .push(format!("{}|{}", req.agent_id, req.user));
        match req.previous_actions.len() {
            0 => r#"{"action":"read_file","path":"docs/setup.md"}"#.into(),
            1 => r#"{"action":"remember","scope":"task","content":"Setup needs Rust 1.88 [docs/setup.md:2]"}"#.into(),
            _ => r#"{"action":"finish","result":"ok"}"#.into(),
        }
    });
    run_until_idle(&mut rt, &mut model, T0, 10);
    let prompts = seen.lock().unwrap().clone();
    assert!(
        prompts.iter().all(|p| !p.contains("British spelling")),
        "writer memory leaked"
    );
    assert!(prompts.iter().all(|p| p.contains("Team fact")));
    let task_memory = rt
        .state()
        .memory
        .values()
        .find(|m| m.scope == Scope::Task(root.clone()))
        .unwrap()
        .clone();
    assert_eq!(task_memory.author, "researcher");
    assert_eq!(task_memory.generation, Generation::Scripted);
    assert_eq!(task_memory.sources[0].reference, "docs/setup.md");
    assert_eq!(
        task_memory.sources[0].sha256.as_deref(),
        Some(
            james_agents::canonical::sha256_hex(&fs::read(ws.join("docs/setup.md")).unwrap())
                .as_str()
        )
    );
    // Another task tree does not see task-scoped memory.
    let other = rt.submit("Something else.", "researcher", T0).unwrap();
    let log2 = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let l2 = log2.clone();
    let mut probe = ScriptedModel::new("probe", move |req: &ModelRequest| {
        *l2.lock().unwrap() = req.user.clone();
        r#"{"action":"finish","result":"x"}"#.into()
    });
    run_until_idle(&mut rt, &mut probe, T0, 5);
    assert!(rt.state().tasks[&other].state == TaskState::Succeeded);
    assert!(!log2.lock().unwrap().contains("Setup needs Rust"));
}

#[test]
fn forget_removes_the_words_and_keeps_a_tombstone_across_restart() {
    let dir = tempdir("forget");
    let (mut rt, _) = Runtime::open(&dir, None, T0).unwrap();
    let id = rt
        .remember(Scope::Team, "Secret-ish: my dentist is on Tuesday.", T0)
        .unwrap();
    assert!(rt.memory_text(&id).unwrap().contains("dentist"));
    rt.forget(&id, T0 + 1).unwrap();
    assert_eq!(rt.memory_text(&id), None);
    drop(rt);
    let (rt, _) = Runtime::open(&dir, None, T0 + 2).unwrap();
    let entry = &rt.state().memory[&id];
    assert!(entry.forgotten);
    assert_eq!(rt.memory_text(&id), None);
    let journal = fs::read_to_string(dir.join("journal.jsonl")).unwrap();
    assert!(
        !journal.contains("dentist"),
        "the journal stores digests, not memory words"
    );
    assert_eq!(fs::read_dir(dir.join("blobs")).unwrap().count(), 0);
}

#[test]
fn tools_are_authorized_by_capability_and_confined_to_the_workspace() {
    let ws = workspace("tools");
    let outside = tempdir("tools-outside");
    fs::write(outside.join("secret.txt"), "outside").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, ws.join("escape")).unwrap();
    let mut rt = Runtime::in_memory(Some(&ws), T0).unwrap();
    let writer_task = rt.submit("Read a file.", "writer", T0).unwrap();
    let researcher_task = rt.submit("Read files.", "researcher", T0).unwrap();
    let mut model = ScriptedModel::queue(vec![
        ("writer", r#"{"action":"read_file","path":"docs/setup.md"}"#),
        ("writer", r#"{"action":"finish","result":"x"}"#),
        (
            "researcher",
            r#"{"action":"read_file","path":"../tools-outside/secret.txt"}"#,
        ),
        (
            "researcher",
            r#"{"action":"read_file","path":"escape/secret.txt"}"#,
        ),
        (
            "researcher",
            r#"{"action":"propose_write","path":"x.md","content":"x"}"#,
        ),
        ("researcher", r#"{"action":"finish","result":"x"}"#),
    ]);
    run_until_idle(&mut rt, &mut model, T0, 20);
    assert!(refusals(&rt, &writer_task)[0].contains("cannot read files"));
    let obs = observations(&rt, &researcher_task);
    let tool_failures: Vec<String> = obs
        .iter()
        .filter_map(|o| match o {
            Observation::Tool {
                ok: false, text, ..
            } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(
        tool_failures[0].contains("inside the workspace"),
        "{tool_failures:?}"
    );
    #[cfg(unix)]
    assert!(
        tool_failures[1].contains("outside the workspace"),
        "{tool_failures:?}"
    );
    assert!(
        refusals(&rt, &researcher_task)
            .iter()
            .any(|r| r.contains("cannot propose"))
    );
    assert!(obs.iter().all(|o| !o.summary().contains("outside\n")));
}

fn write_flow(tag: &str) -> (Runtime, PathBuf, String, String) {
    let ws = workspace(tag);
    let mut rt = Runtime::in_memory(Some(&ws), T0).unwrap();
    let task = rt.submit("Write notes/summary.md.", "writer", T0).unwrap();
    let mut model = ScriptedModel::queue(vec![(
        "writer",
        r##"{"action":"propose_write","path":"notes/summary.md","content":"# Summary\n"}"##,
    )]);
    run_until_idle(&mut rt, &mut model, T0, 5);
    let approval = rt.state().approvals.values().next().unwrap().clone();
    assert_eq!(rt.state().tasks[&task].state, TaskState::AwaitingApproval);
    assert!(
        !ws.join("notes/summary.md").exists(),
        "nothing is written before a decision"
    );
    (rt, ws, task, approval.id)
}

#[test]
fn a_write_waits_for_the_person_and_binds_to_what_they_saw() {
    let (mut rt, ws, task, id) = write_flow("approve");
    let digest = rt.state().approvals[&id].action_sha256.clone();
    let wrong = "0".repeat(64);
    assert!(rt.decide(&id, true, &wrong, T0 + 1).is_err());
    assert!(!ws.join("notes/summary.md").exists());
    assert_eq!(rt.decide(&id, true, &digest, T0 + 2).unwrap(), "applied");
    assert_eq!(
        fs::read_to_string(ws.join("notes/summary.md")).unwrap(),
        "# Summary\n"
    );
    assert!(
        rt.decide(&id, true, &digest, T0 + 3).is_err(),
        "a decision is used once"
    );
    assert_eq!(rt.state().tasks[&task].state, TaskState::Running);
    let record = &rt.state().approvals[&id].record;
    assert_eq!(record["decision"]["verdict"], "approved");
    assert!(james_agents::canonical::verify_sealed(record));
}

#[test]
fn denial_expiry_and_a_changed_file_write_nothing() {
    let (mut rt, ws, _, id) = write_flow("deny");
    let digest = rt.state().approvals[&id].action_sha256.clone();
    assert_eq!(rt.decide(&id, false, &digest, T0 + 1).unwrap(), "denied");
    assert!(!ws.join("notes/summary.md").exists());

    let (mut rt, ws, _, id) = write_flow("changed");
    fs::create_dir_all(ws.join("notes")).unwrap();
    fs::write(ws.join("notes/summary.md"), "someone else wrote this").unwrap();
    let digest = rt.state().approvals[&id].action_sha256.clone();
    assert_eq!(
        rt.decide(&id, true, &digest, T0 + 1).unwrap(),
        "not_applied"
    );
    assert_eq!(
        fs::read_to_string(ws.join("notes/summary.md")).unwrap(),
        "someone else wrote this"
    );

    let (mut rt, ws, task, id) = write_flow("expired");
    let digest = rt.state().approvals[&id].action_sha256.clone();
    let late = T0 + Limits::default().approval_ttl_ms + 1;
    assert!(rt.decide(&id, true, &digest, late).is_err());
    assert_eq!(
        rt.state().approvals[&id].outcome.as_deref(),
        Some("expired")
    );
    assert!(!ws.join("notes/summary.md").exists());
    assert_eq!(rt.state().tasks[&task].state, TaskState::Running);
}

#[test]
fn cancel_cascades_voids_proposals_and_cuts_an_in_flight_call() {
    let ws = workspace("cancel");
    let mut rt = Runtime::in_memory(Some(&ws), T0).unwrap();
    let root = rt.submit("Do a lot.", "coordinator", T0).unwrap();
    let mut model = ScriptedModel::queue(vec![
        (
            "coordinator",
            r#"{"action":"delegate","to":"writer","objective":"Propose a file."}"#,
        ),
        (
            "writer",
            r#"{"action":"propose_write","path":"a.md","content":"a"}"#,
        ),
    ]);
    run_until_idle(&mut rt, &mut model, T0, 10);
    let child = rt.state().tasks[&root].children[0].clone();
    assert_eq!(rt.state().tasks[&child].state, TaskState::AwaitingApproval);
    rt.cancel(&root, T0 + 1).unwrap();
    assert_eq!(rt.state().tasks[&root].state, TaskState::Cancelled);
    assert_eq!(rt.state().tasks[&child].state, TaskState::Cancelled);
    assert_eq!(
        rt.state()
            .approvals
            .values()
            .next()
            .unwrap()
            .outcome
            .as_deref(),
        Some("voided")
    );

    // A cancellation that arrives while the model is answering.
    struct CancelsItself;
    impl ModelPort for CancelsItself {
        fn label(&self) -> String {
            "self-cancelling probe".into()
        }
        fn complete(
            &mut self,
            req: &ModelRequest,
            cancel: &CancelSignal,
        ) -> Result<ModelReply, String> {
            cancel.request([req.task_id.clone()]);
            Ok(ModelReply {
                text: r#"{"action":"finish","result":"too late"}"#.into(),
                model: self.label(),
                generation: Generation::Scripted,
            })
        }
    }
    let t = rt.submit("Answer slowly.", "researcher", T0 + 2).unwrap();
    assert_eq!(
        rt.step(&mut CancelsItself, T0 + 3).unwrap(),
        StepOutcome::Cancelled(t.clone())
    );
    assert_eq!(rt.state().tasks[&t].state, TaskState::Cancelled);
    assert!(
        rt.state().tasks[&t].result.is_none(),
        "the cut reply is not used"
    );
}

#[test]
fn restart_interrupts_mid_step_work_and_continues_queued_work() {
    let dir = tempdir("restart");
    let ws = workspace("restart");
    let (mut rt, _) = Runtime::open(&dir, Some(&ws), T0).unwrap();
    let busy = rt.submit("Read the setup.", "researcher", T0).unwrap();
    let mut model = ScriptedModel::queue(vec![(
        "researcher",
        r#"{"action":"read_file","path":"docs/setup.md"}"#,
    )]);
    rt.step(&mut model, T0).unwrap();
    let waiting = rt.submit("Queued task.", "reviewer", T0).unwrap();
    drop(rt); // James closes with `busy` mid-task.
    let (mut rt, recovery) = Runtime::open(&dir, None, T0 + 60_000).unwrap();
    assert_eq!(recovery.interrupted, vec![busy.clone()]);
    assert_eq!(rt.state().tasks[&busy].state, TaskState::Interrupted);
    assert_eq!(rt.state().tasks[&waiting].state, TaskState::Queued);
    assert!(rt.workspace().is_some(), "the chosen folder is remembered");
    let mut model = ScriptedModel::queue(vec![
        ("reviewer", r#"{"action":"finish","result":"reviewed"}"#),
        (
            "researcher",
            r#"{"action":"finish","result":"Rust 1.88 [docs/setup.md:2]"}"#,
        ),
    ]);
    run_until_idle(&mut rt, &mut model, T0 + 60_000, 5);
    assert_eq!(rt.state().tasks[&waiting].state, TaskState::Succeeded);
    assert_eq!(
        rt.state().tasks[&busy].state,
        TaskState::Interrupted,
        "nothing resumes without the person"
    );
    rt.resume(&busy, T0 + 61_000).unwrap();
    run_until_idle(&mut rt, &mut model, T0 + 61_000, 5);
    let task = &rt.state().tasks[&busy];
    assert_eq!(task.state, TaskState::Succeeded);
    assert!(task.transcript.iter().any(|t| matches!(t, Turn::Observation(Observation::Tool { tool, .. }) if tool == "read_file")), "history survived");
    rt.verify().unwrap();
}

#[test]
fn a_torn_final_line_is_set_aside_and_a_damaged_history_refuses_to_open() {
    let dir = tempdir("torn");
    let (mut rt, _) = Runtime::open(&dir, None, T0).unwrap();
    rt.submit("x", "writer", T0).unwrap();
    drop(rt);
    let path = dir.join("journal.jsonl");
    let mut bytes = fs::read(&path).unwrap();
    bytes.extend_from_slice(b"{\"schema\":\"agent-event/v1\",\"seq\":");
    fs::write(&path, &bytes).unwrap();
    let (rt, recovery) = Runtime::open(&dir, None, T0 + 1).unwrap();
    assert!(
        recovery
            .journal
            .torn_tail
            .as_ref()
            .is_some_and(|p| p.exists())
    );
    rt.verify().unwrap();
    drop(rt);
    let text = fs::read_to_string(&path).unwrap();
    let tampered = text.replacen("\"objective\":\"x\"", "\"objective\":\"y\"", 1);
    assert_ne!(text, tampered);
    fs::write(&path, tampered).unwrap();
    assert!(
        Runtime::open(&dir, None, T0 + 2).is_err(),
        "a changed record must not be continued"
    );
}

#[test]
fn an_approved_write_interrupted_mid_apply_is_reconciled_by_reading_the_file() {
    for (tag, file_state, expected) in [
        ("rec-applied", Some("# Summary\n"), "applied"),
        ("rec-not", None, "not_applied"),
        ("rec-unknown", Some("other"), "unknown"),
    ] {
        let dir = tempdir(tag);
        let ws = workspace(tag);
        let (mut rt, _) = Runtime::open(&dir, Some(&ws), T0).unwrap();
        rt.submit("Write notes/summary.md.", "writer", T0).unwrap();
        let mut model = ScriptedModel::queue(vec![(
            "writer",
            r##"{"action":"propose_write","path":"notes/summary.md","content":"# Summary\n"}"##,
        )]);
        run_until_idle(&mut rt, &mut model, T0, 5);
        let a = rt.state().approvals.values().next().unwrap().clone();
        drop(rt);
        // Simulate a crash between "decided/applying" and "closed".
        let (mut journal, _) = Journal::open(&dir.join("journal.jsonl")).unwrap();
        for (kind, data) in [
            (
                "approval.decided",
                json!({ "request_id": a.id, "verdict": "approved", "action_sha256": a.action_sha256 }),
            ),
            ("approval.applying", json!({ "request_id": a.id })),
        ] {
            journal
                .append(Draft {
                    at: T0 + 1,
                    actor: "person",
                    kind,
                    task_id: Some(&a.task_id),
                    generation: Generation::Person,
                    data,
                })
                .unwrap();
        }
        drop(journal);
        if let Some(content) = file_state {
            fs::create_dir_all(ws.join("notes")).unwrap();
            fs::write(ws.join("notes/summary.md"), content).unwrap();
        }
        let (rt, recovery) = Runtime::open(&dir, None, T0 + 2).unwrap();
        assert_eq!(
            recovery.reconciled,
            vec![(a.id.clone(), expected.to_owned())],
            "{tag}"
        );
        assert_eq!(
            rt.state().approvals[&a.id].outcome.as_deref(),
            Some(expected)
        );
    }
}

#[test]
fn routines_fire_once_per_minute_and_missed_runs_are_recorded_not_run() {
    let dir = tempdir("routines");
    let (mut rt, _) = Runtime::open(&dir, None, T0).unwrap();
    rt.save_routine(
        None,
        "Morning brief",
        "0 * * * *",
        0,
        "researcher",
        "List what changed.",
        true,
        T0,
    )
    .unwrap();
    let at_one = T0 + 3_600_000; // 13:00
    let created = rt.tick(at_one).unwrap();
    assert_eq!(created.len(), 1);
    assert!(
        rt.tick(at_one + 30_000).unwrap().is_empty(),
        "once per minute"
    );
    assert_eq!(rt.state().tasks[&created[0]].created_by, "host");
    drop(rt);
    // Closed for three hours: 14:00, 15:00 and 16:00 came due.
    let (rt, recovery) = Runtime::open(&dir, None, at_one + 3 * 3_600_000 + 30 * 60_000).unwrap();
    assert_eq!(recovery.missed_routines, vec![("R-0001".to_owned(), 3)]);
    assert_eq!(rt.state().tasks.len(), 1, "missed runs are not run late");
    assert!(
        Runtime::in_memory(None, T0)
            .unwrap()
            .save_routine(None, "bad", "61 * * * *", 0, "writer", "x", true, T0)
            .is_err()
    );
}

#[test]
fn failures_and_timeouts_end_tasks_honestly() {
    struct Down;
    impl ModelPort for Down {
        fn label(&self) -> String {
            "down".into()
        }
        fn complete(&mut self, _: &ModelRequest, _: &CancelSignal) -> Result<ModelReply, String> {
            Err("connection refused".into())
        }
    }
    let mut rt = Runtime::in_memory(None, T0).unwrap();
    let t = rt.submit("x", "writer", T0).unwrap();
    run_until_idle(&mut rt, &mut Down, T0, 10);
    let task = &rt.state().tasks[&t];
    assert_eq!(task.state, TaskState::Failed);
    assert!(
        task.result
            .as_deref()
            .unwrap()
            .contains("connection refused")
    );

    let t = rt.submit("x", "writer", T0).unwrap();
    let mut babble = ScriptedModel::new("babble", |_: &ModelRequest| {
        "I think the answer is yes.".into()
    });
    run_until_idle(&mut rt, &mut babble, T0, 10);
    assert!(
        rt.state().tasks[&t]
            .result
            .as_deref()
            .unwrap()
            .contains("unusable")
    );

    let t = rt.submit("x", "writer", T0).unwrap();
    let mut model = ScriptedModel::queue(vec![]);
    let late = T0 + Limits::default().deadline_ms + 1;
    run_until_idle(&mut rt, &mut model, late, 5);
    assert!(
        rt.state().tasks[&t]
            .result
            .as_deref()
            .unwrap()
            .contains("time limit")
    );

    let mut rt = Runtime::in_memory(None, T0).unwrap().with_limits(Limits {
        root_steps: 2,
        ..Limits::default()
    });
    let t = rt.submit("x", "writer", T0).unwrap();
    let mut chatty = ScriptedModel::new("chatty", |_: &ModelRequest| {
        r#"{"action":"remember","scope":"agent","content":"again"}"#.into()
    });
    run_until_idle(&mut rt, &mut chatty, T0, 10);
    assert!(
        rt.state().tasks[&t]
            .result
            .as_deref()
            .unwrap()
            .contains("budget")
    );
}

#[test]
fn works_offline_and_reports_a_missing_model_server() {
    // Nothing listens on this port: the local adapter fails fast and the task fails with the reason.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut local = LocalModel::new(james_models::LocalProvider::LlamaCpp, port, "none").unwrap();
    let mut rt = Runtime::in_memory(None, T0).unwrap();
    let t = rt.submit("Say hi.", "writer", T0).unwrap();
    run_until_idle(&mut rt, &mut local, T0, 10);
    assert_eq!(rt.state().tasks[&t].state, TaskState::Failed);
    let activity = rt.state().activity();
    assert!(activity.iter().all(|(_, a)| *a == Activity::Idle));
}

#[test]
fn a_local_model_server_drives_a_task_end_to_end() {
    // A minimal OpenAI-compatible server on loopback, as llama.cpp answers.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        use std::io::{Read, Write};
        let (mut socket, _) = listener.accept().unwrap();
        let mut buf = [0u8; 65536];
        let mut request = Vec::new();
        loop {
            let n = socket.read(&mut buf).unwrap();
            request.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&request);
            if let Some(head_end) = text.find("\r\n\r\n") {
                let length: usize = text[..head_end]
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse().unwrap())
                    })
                    .unwrap_or(0);
                if request.len() >= head_end + 4 + length {
                    break;
                }
            }
        }
        let content = r#"{\"action\":\"finish\",\"result\":\"Hello from a local model.\"}"#;
        let body = format!(
            "data: {{\"choices\":[{{\"delta\":{{\"content\":\"{content}\"}}}}]}}\n\ndata: [DONE]\n\n"
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).unwrap();
        String::from_utf8_lossy(&request).into_owned()
    });
    let mut local =
        LocalModel::new(james_models::LocalProvider::LlamaCpp, port, "stand-in").unwrap();
    let mut rt = Runtime::in_memory(None, T0).unwrap();
    let t = rt.submit("Say hello.", "writer", T0).unwrap();
    rt.step(&mut local, T0).unwrap();
    let request = server.join().unwrap();
    assert!(request.contains("POST /v1/chat/completions"));
    assert!(request.contains("Say hello."));
    let task = &rt.state().tasks[&t];
    assert_eq!(task.state, TaskState::Succeeded);
    assert_eq!(task.result_generation, Some(Generation::Model));
    let called = rt
        .events()
        .iter()
        .find(|e| e.kind == "model.called")
        .unwrap();
    assert_eq!(called.generation, Generation::Model);
    assert!(called.data["model"].as_str().unwrap().contains("llama.cpp"));
}

#[test]
fn capabilities_cannot_be_granted_by_an_agent() {
    // A model that asks for a capability it lacks gets a refusal, and the
    // profile it acts under is unchanged.
    let mut rt = Runtime::in_memory(None, T0).unwrap();
    let t = rt.submit("x", "reviewer", T0).unwrap();
    let mut model = ScriptedModel::queue(vec![
        (
            "reviewer",
            r#"{"action":"grant","capability":"propose_write"}"#,
        ),
        (
            "reviewer",
            r#"{"action":"propose_write","path":"a","content":"b"}"#,
        ),
        ("reviewer", r#"{"action":"finish","result":"x"}"#),
    ]);
    run_until_idle(&mut rt, &mut model, T0, 10);
    let caps = &rt.state().agents["reviewer"].capabilities;
    assert!(!caps.contains(&Capability::ProposeWrite));
    assert!(refusals(&rt, &t).len() >= 2);
    let _: Option<AgentProfile> = None;
    let _ = Path::new(".");
}

#[test]
fn a_repeated_memory_write_is_refused_not_stored_again() {
    // Found with a real 3B local model, which wrote the same fact seven times.
    let mut rt = Runtime::in_memory(None, T0).unwrap();
    let t = rt.submit("Remember things.", "researcher", T0).unwrap();
    let mut model = ScriptedModel::queue(vec![
        (
            "researcher",
            r#"{"action":"remember","scope":"team","content":"The installer must be signed."}"#,
        ),
        (
            "researcher",
            r#"{"action":"remember","scope":"team","content":"The installer must be signed."}"#,
        ),
        ("researcher", r#"{"action":"finish","result":"ok"}"#),
    ]);
    run_until_idle(&mut rt, &mut model, T0, 10);
    assert_eq!(rt.state().memory.len(), 1);
    assert!(
        refusals(&rt, &t)
            .iter()
            .any(|r| r.contains("already kept as M-000001"))
    );
}
