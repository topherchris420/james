//! Demonstration A: a person gives James's team one task; four assistants
//! collaborate through bounded actions; the file they produce waits for the
//! person's approval; the whole run is a verifiable journal.
//!
//!   cargo run -p james-agents --example team_demo                       # scripted team, offline
//!   cargo run -p james-agents --example team_demo -- --llama-cpp 8080 qwen2.5-1.5b-instruct
//!   cargo run -p james-agents --example team_demo -- --ollama 11434 gemma3:4b
//!   cargo run -p james-agents --example team_demo -- --approve           # non-interactive
//!
//! Other flags: `--lm-studio PORT MODEL`, `--deny`, `--data DIR`, `--workspace DIR`.
//! The scripted team is a rule-based policy; every reply it gives is recorded
//! as `scripted`, never as a model's.

use james_agents::clock::now_ms;
use james_agents::model::{ModelPort, ModelRequest};
use james_agents::protocol::Observation;
use james_agents::runtime::{TaskState, Turn};
use james_agents::{LocalModel, Runtime, ScriptedModel, StepOutcome, one_line};
use james_models::LocalProvider;
use std::io::{BufRead, Write};
use std::path::PathBuf;

const OBJECTIVE: &str = "Read the project notes in docs/ and produce notes/brief.md: a short brief of what must happen before release, \
with every point cited as [path:line]. Have the reviewer check the citations before the writer proposes the file.";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().position(|a| a == name);
    let value = |name: &str, offset: usize| flag(name).and_then(|i| args.get(i + offset)).cloned();
    let decision = if flag("--approve").is_some() {
        Some(true)
    } else if flag("--deny").is_some() {
        Some(false)
    } else {
        None
    };
    let scratch = std::env::temp_dir().join(format!("james-team-demo-{}", std::process::id()));
    let data = value("--data", 1)
        .map(PathBuf::from)
        .unwrap_or_else(|| scratch.join("team"));
    let workspace = value("--workspace", 1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let ws = scratch.join("workspace");
            seed_workspace(&ws);
            ws
        });

    let mut model: Box<dyn ModelPort> = match ["--llama-cpp", "--ollama", "--lm-studio"]
        .iter()
        .find(|f| flag(f).is_some())
    {
        Some(f) => {
            let provider = match *f {
                "--llama-cpp" => LocalProvider::LlamaCpp,
                "--ollama" => LocalProvider::Ollama,
                _ => LocalProvider::LmStudio,
            };
            let port: u16 = value(f, 1)
                .and_then(|p| p.parse().ok())
                .unwrap_or(provider.default_port());
            let name = value(f, 2).unwrap_or_default();
            Box::new(LocalModel::new(provider, port, name).expect("model settings"))
        }
        None => Box::new(scripted_team()),
    };

    println!("James team demonstration");
    println!("  model:     {}", model.label());
    println!("  workspace: {}", workspace.display());
    println!("  journal:   {}", data.join("journal.jsonl").display());
    let (mut rt, recovery) =
        Runtime::open(&data, Some(&workspace), now_ms()).expect("open the team");
    if !recovery.interrupted.is_empty() {
        println!(
            "  recovered: interrupted tasks {:?} wait for you",
            recovery.interrupted
        );
    }
    let root = rt
        .submit(OBJECTIVE, "coordinator", now_ms())
        .expect("submit");
    println!(
        "\nYou → coordinator ({root}): {}\n",
        one_line(OBJECTIVE, 200)
    );

    let mut shown = 0;
    loop {
        let outcome = rt.step(model.as_mut(), now_ms()).expect("step");
        shown = print_new_events(&rt, shown);
        if outcome != StepOutcome::Idle {
            continue;
        }
        let pending: Vec<_> = rt
            .state()
            .approvals
            .values()
            .filter(|a| a.is_pending())
            .cloned()
            .collect();
        if pending.is_empty() {
            break;
        }
        for a in pending {
            let content = rt.blob(&a.content_sha256).unwrap_or_default();
            println!(
                "\n┌─ Approval needed ({}) ─ {} wants to write {}",
                a.id, a.requested_by, a.path
            );
            println!("│  exact action sha256 {}", a.action_sha256);
            println!(
                "│  previous file: {}",
                a.previous_sha256.as_deref().unwrap_or("(none; a new file)")
            );
            for line in content.lines() {
                println!("│  {line}");
            }
            let approve = decision.unwrap_or_else(|| {
                print!("└─ Write this file? [y/N] ");
                let _ = std::io::stdout().flush();
                let mut answer = String::new();
                let _ = std::io::stdin().lock().read_line(&mut answer);
                matches!(answer.trim(), "y" | "Y" | "yes")
            });
            if decision.is_some() {
                println!(
                    "└─ {} by command-line flag",
                    if approve { "Approved" } else { "Denied" }
                );
            }
            let outcome = rt
                .decide(&a.id, approve, &a.action_sha256, now_ms())
                .expect("decide");
            println!("   → {outcome}\n");
        }
    }
    shown = print_new_events(&rt, shown);
    let _ = shown;

    let task = &rt.state().tasks[&root];
    println!(
        "\nResult ({}, words by {}):",
        task.state.as_str(),
        task.result_generation
            .map(|g| g.as_str())
            .unwrap_or("nobody")
    );
    println!("{}", task.result.as_deref().unwrap_or("(none)"));
    println!("\nMemory kept, with provenance:");
    for m in rt.state().memory.values() {
        let sources: Vec<String> = m
            .sources
            .iter()
            .map(|s| {
                format!(
                    "{}@{}",
                    s.reference,
                    s.sha256.as_deref().map(|h| &h[..10]).unwrap_or("-")
                )
            })
            .collect();
        println!(
            "  {} [{}] by {} ({}) from {:?}: {}",
            m.id,
            m.scope.plain(),
            m.author,
            m.generation.as_str(),
            sources,
            one_line(&rt.memory_text(&m.id).unwrap_or_default(), 100)
        );
    }
    let brief = workspace.join("notes/brief.md");
    println!(
        "\nnotes/brief.md on disk: {}",
        if brief.exists() {
            "written"
        } else {
            "not written"
        }
    );
    let tasks = rt.state().tasks.values().filter(|t| t.root == root).count();
    let calls = rt
        .events()
        .iter()
        .filter(|e| {
            e.kind == "model.called"
                && e.task_id
                    .as_deref()
                    .is_some_and(|t| rt.state().tasks[t].root == root)
        })
        .count();
    let refused = rt
        .events()
        .iter()
        .filter(|e| e.kind == "agent.output_invalid")
        .count();
    println!(
        "Tasks in the tree: {tasks}; model calls: {calls}; unusable replies refused: {refused}"
    );
    match rt.verify() {
        Ok(()) => println!(
            "Journal chain verified: {} records, head {}",
            rt.events().len(),
            &rt.events().last().unwrap().sha256[..16]
        ),
        Err(e) => println!("Journal verification FAILED: {e}"),
    }
    if task.state != TaskState::Succeeded {
        std::process::exit(2);
    }
}

fn print_new_events(rt: &Runtime, from: usize) -> usize {
    let events = rt.events();
    for e in &events[from..] {
        let who = e.actor.as_str();
        let task = e.task_id.as_deref().unwrap_or("");
        let line = match e.kind.as_str() {
            "task.created" => format!(
                "{task} created for {} by {}",
                e.data["task"]["assignee"].as_str().unwrap_or(""),
                who
            ),
            "agent.output" => format!(
                "{who} [{}] {}",
                e.generation.as_str(),
                e.data["summary"].as_str().unwrap_or("")
            ),
            "agent.output_invalid" => format!(
                "host refused a reply: {}",
                e.data["reason"].as_str().unwrap_or("")
            ),
            "model.failed" => format!("model error: {}", e.data["error"].as_str().unwrap_or("")),
            "task.observed" => match Observation::from_value(&e.data["observation"]) {
                Some(o @ Observation::Tool { .. }) | Some(o @ Observation::Refused { .. }) => {
                    let to = rt
                        .state()
                        .tasks
                        .get(task)
                        .map(|t| t.assignee.as_str())
                        .unwrap_or(who);
                    format!("  host → {to}: {}", o.summary())
                }
                _ => continue,
            },
            "approval.requested" => format!("{task} is waiting for your approval"),
            "approval.closed" => format!(
                "approval {}: {}",
                e.data["outcome"].as_str().unwrap_or(""),
                e.data["detail"].as_str().unwrap_or("")
            ),
            "task.finished" => format!("{task} {}", e.data["state"].as_str().unwrap_or("")),
            "memory.written" => format!(
                "memory {} written",
                e.data["memory"]["memory_id"].as_str().unwrap_or("")
            ),
            _ => continue,
        };
        println!("  #{:<3} {}", e.seq, one_line(&line, 160));
    }
    events.len()
}

fn seed_workspace(ws: &std::path::Path) {
    let _ = std::fs::remove_dir_all(ws);
    std::fs::create_dir_all(ws.join("docs")).unwrap();
    std::fs::write(
        ws.join("docs/release.md"),
        "# Release checklist\n\nThe installer must be signed before release.\nScreen-reader testing is still open.\nThe changelog needs a dated entry.\n",
    )
    .unwrap();
    std::fs::write(
        ws.join("docs/known-issues.md"),
        "# Known issues\n\nOn macOS the first launch must be allowed in Privacy settings.\nLM Studio validation is incomplete and must be finished before release.\n",
    )
    .unwrap();
    std::fs::write(ws.join("docs/style.md"), "# Style\n\nUse plain words.\n").unwrap();
}

/// The rule-based team. It reads only what the host shows it.
fn scripted_team() -> ScriptedModel {
    ScriptedModel::new("demo team", |req: &ModelRequest| {
        let tools: Vec<(&str, &str)> = req
            .observations
            .iter()
            .filter_map(|o| match o {
                Observation::Tool {
                    tool,
                    ok: true,
                    text,
                } => Some((tool.as_str(), text.as_str())),
                _ => None,
            })
            .collect();
        let children: Vec<(&str, &str)> = req
            .observations
            .iter()
            .filter_map(|o| match o {
                Observation::Child { agent, result, .. } => Some((agent.as_str(), result.as_str())),
                _ => None,
            })
            .collect();
        let json = |v: serde_json::Value| v.to_string();
        match req.agent_id.as_str() {
            "coordinator" => match children.as_slice() {
                [] => json(
                    serde_json::json!({"action":"delegate","to":"researcher","max_steps":12,
                    "objective":"Read every file under docs/ and report what must happen before release. Quote each point and cite it as [path:line]. Remember the key finding for the team."}),
                ),
                [(_, findings)] => json(
                    serde_json::json!({"action":"delegate","to":"writer","max_steps":4,
                    "objective":format!("Draft a release brief from these findings. Keep every citation. Finish with the draft text; do not propose a file yet.\n{findings}")}),
                ),
                [_, (_, draft)] => json(
                    serde_json::json!({"action":"delegate","to":"reviewer","max_steps":4,
                    "objective":format!("Check every citation in this draft against the files.\n{draft}")}),
                ),
                [_, (_, draft), (_, review)] if review.contains("0 fail") => json(
                    serde_json::json!({"action":"delegate","to":"writer","max_steps":4,
                    "objective":format!("The reviewer confirmed the citations. Propose notes/brief.md with exactly this content:\n{draft}")}),
                ),
                [_, _, (_, review)] => json(
                    serde_json::json!({"action":"finish","result":format!("Stopped before proposing a file: the reviewer found problems.\n{review}")}),
                ),
                [.., (_, outcome)] => json(serde_json::json!({"action":"finish","result":format!(
                    "The researcher read the docs, the writer drafted a cited brief, the reviewer's host check passed, and the writer's file proposal went to you. Outcome: {outcome}")})),
            },
            "researcher" => {
                let listed = tools
                    .iter()
                    .find(|(t, _)| *t == "list_files")
                    .map(|(_, text)| *text);
                let reads: Vec<&str> = tools
                    .iter()
                    .filter(|(t, _)| *t == "read_file")
                    .map(|(_, text)| *text)
                    .collect();
                let Some(listing) = listed else {
                    return json(serde_json::json!({"action":"list_files","path":"docs"}));
                };
                let files: Vec<String> = listing
                    .lines()
                    .filter_map(|l| l.split(' ').next())
                    .filter(|n| n.ends_with(".md"))
                    .map(|n| format!("docs/{n}"))
                    .collect();
                if let Some(next) = files.get(reads.len()) {
                    return json(serde_json::json!({"action":"read_file","path":next}));
                }
                let mut points = Vec::new();
                for read in &reads {
                    let path = read
                        .lines()
                        .next()
                        .and_then(|l| l.split(' ').next())
                        .unwrap_or("");
                    for line in read.lines().skip(1) {
                        let Some((number, text)) = line.split_once("| ") else {
                            continue;
                        };
                        if text.contains("must")
                            || text.contains("still open")
                            || text.contains("needs")
                        {
                            points.push(format!(
                                "- \"{}\" [{path}:{}]",
                                text.trim(),
                                number.trim()
                            ));
                        }
                    }
                }
                let remembered = req
                    .previous_actions
                    .iter()
                    .any(|a| a["action"] == "remember");
                if !remembered {
                    return json(serde_json::json!({"action":"remember","scope":"team",
                        "content":format!("Release blockers found in docs/: {} items (see the researcher's report).", points.len())}));
                }
                json(serde_json::json!({"action":"finish","result":points.join("\n")}))
            }
            "writer" => {
                if let Some(content) = req
                    .objective
                    .split_once("exactly this content:\n")
                    .map(|(_, c)| c)
                {
                    let decided = req.observations.iter().find_map(|o| match o {
                        Observation::Approval {
                            outcome, detail, ..
                        } => Some(format!("{outcome}: {detail}")),
                        _ => None,
                    });
                    return match decided {
                        None => json(
                            serde_json::json!({"action":"propose_write","path":"notes/brief.md","content":content}),
                        ),
                        Some(result) => {
                            json(serde_json::json!({"action":"finish","result":result}))
                        }
                    };
                }
                let points: Vec<&str> = req
                    .objective
                    .lines()
                    .filter(|l| l.starts_with("- "))
                    .collect();
                let draft = format!("# Before release\n\n{}\n", points.join("\n"));
                json(serde_json::json!({"action":"finish","result":draft}))
            }
            "reviewer" => {
                let checked = tools
                    .iter()
                    .find(|(t, _)| *t == "verify_citations")
                    .map(|(_, text)| *text);
                match checked {
                    None => {
                        let draft = req.objective.split_once('\n').map(|(_, d)| d).unwrap_or("");
                        json(serde_json::json!({"action":"verify_citations","text":draft}))
                    }
                    Some(report) => json(serde_json::json!({"action":"finish","result":report})),
                }
            }
            _ => json(serde_json::json!({"action":"fail","reason":"not part of the demo"})),
        }
    })
}

#[allow(dead_code)]
fn turns(rt: &Runtime, task: &str) -> usize {
    rt.state().tasks[task]
        .transcript
        .iter()
        .filter(|t| matches!(t, Turn::Action { .. }))
        .count()
}
