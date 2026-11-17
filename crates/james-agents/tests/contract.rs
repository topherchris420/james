//! `agent-interop/v1` conformance: the vendored specification is the one the
//! R.A.I.N. Lab pins, this crate reproduces the Lab's canonical JSON and
//! digests byte for byte, refuses the negative fixtures, and the records it
//! exports satisfy the shared schemas.

use james_agents::canonical::{self, sha256_hex};
use james_agents::memory::Scope;
use james_agents::{AGENT_INTEROP_V1_MANIFEST_SHA256, Runtime, ScriptedModel};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn spec() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/interop/agent-interop-v1")
}

fn read(rel: &str) -> Value {
    serde_json::from_slice(&fs::read(spec().join(rel)).unwrap()).unwrap()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            walk(&path, out)
        } else {
            out.push(path)
        }
    }
}

#[test]
fn vendored_specification_matches_the_pin_shared_with_the_lab() {
    let root = spec();
    let manifest = fs::read_to_string(root.join("MANIFEST.sha256")).unwrap();
    let mut files = Vec::new();
    walk(&root, &mut files);
    let mut on_disk: Vec<String> = files
        .iter()
        .map(|p| {
            p.strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .filter(|p| p != "MANIFEST.sha256")
        .collect();
    on_disk.sort();
    let listed: Vec<(&str, &str)> = manifest.lines().map(|l| (&l[66..], &l[..64])).collect();
    assert_eq!(
        listed
            .iter()
            .map(|(p, _)| p.to_string())
            .collect::<Vec<_>>(),
        on_disk
    );
    for (path, sha) in listed {
        assert_eq!(
            sha256_hex(&fs::read(root.join(path)).unwrap()),
            sha,
            "{path}"
        );
    }
    assert_eq!(
        sha256_hex(manifest.as_bytes()),
        AGENT_INTEROP_V1_MANIFEST_SHA256
    );
}

#[test]
fn reproduces_the_labs_canonical_text_and_digests() {
    let fixtures = read("fixtures/canonical.json");
    let cases = fixtures["cases"].as_array().unwrap();
    assert!(cases.len() > 5);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        assert_eq!(
            canonical::canonical(&case["value"]).unwrap(),
            case["canonical"].as_str().unwrap(),
            "{name}"
        );
        assert_eq!(
            canonical::digest(&case["value"]).unwrap(),
            case["sha256"].as_str().unwrap(),
            "{name}"
        );
    }
}

#[test]
fn every_sealed_fixture_verifies_and_every_negative_fixture_fails() {
    for file in [
        "agent-identity.json",
        "agent-task.json",
        "agent-memory.json",
        "agent-approval.json",
        "rain-interop-evidence.json",
        "rain-interop-proposal.json",
    ] {
        let value = read(&format!("fixtures/{file}"));
        assert!(canonical::verify_sealed(&value), "{file}");
        let schema = read(&format!(
            "schemas/{}",
            file.replace(".json", ".schema.json")
        ));
        assert_eq!(
            schema_errors(&value, &schema),
            Vec::<String>::new(),
            "{file}"
        );
    }
    let chain = read("fixtures/agent-event-chain.json");
    assert!(chain_errors(chain["events"].as_array().unwrap()).is_empty());
    let approval = read("fixtures/agent-approval.json");
    assert_eq!(
        canonical::digest(&approval["action"]).unwrap(),
        approval["action_sha256"].as_str().unwrap()
    );

    let negative = read("fixtures/negative.json");
    for case in negative["cases"].as_array().unwrap() {
        let value = &case["value"];
        let refused = match case["kind"].as_str().unwrap() {
            "agent-event-chain" => !chain_errors(value.as_array().unwrap()).is_empty(),
            "agent-approval/v1" => {
                // Resealed, so the record digest holds; the action digest does not.
                canonical::digest(&value["action"]).unwrap()
                    != value["action_sha256"].as_str().unwrap()
            }
            _ => !canonical::verify_sealed(value),
        };
        assert!(refused, "{}", case["name"]);
    }
}

fn chain_errors(events: &[Value]) -> Vec<String> {
    let mut prev = Value::Null;
    let mut errors = Vec::new();
    for (i, e) in events.iter().enumerate() {
        if !canonical::verify_sealed(e) {
            errors.push(format!("{i}: digest"));
        }
        if e["prev"] != prev || e["seq"].as_u64() != Some(i as u64 + 1) {
            errors.push(format!("{i}: link"));
        }
        prev = e["sha256"].clone();
    }
    errors
}

#[test]
fn records_james_exports_satisfy_the_shared_schemas() {
    let dir = tempdir("export");
    fs::write(dir.join("notes.md"), "line one\nline two\n").unwrap();
    let now = 1_794_830_400_000;
    let mut rt = Runtime::in_memory(Some(&dir), now).unwrap();
    rt.remember(Scope::Team, "The person likes short answers.", now)
        .unwrap();
    let root = rt
        .submit("Summarise notes.md into summary.md.", "coordinator", now)
        .unwrap();
    let mut model = ScriptedModel::queue(vec![
        (
            "coordinator",
            r#"{"action":"delegate","to":"writer","objective":"Write summary.md saying line one."}"#,
        ),
        (
            "writer",
            r#"{"action":"propose_write","path":"summary.md","content":"line one [notes.md:1]\n"}"#,
        ),
    ]);
    for _ in 0..4 {
        rt.step(&mut model, now).unwrap();
    }
    assert!(rt.state().tasks.contains_key(&root));
    let checks: [(&str, Vec<Value>); 5] = [
        ("agent-identity", rt.identity_records()),
        ("agent-task", rt.task_records()),
        ("agent-memory", rt.memory_records()),
        ("agent-approval", rt.approval_records()),
        ("agent-event", rt.event_records()),
    ];
    for (name, records) in checks {
        assert!(!records.is_empty(), "{name}");
        let schema = read(&format!("schemas/{name}.schema.json"));
        for r in records {
            assert!(canonical::verify_sealed(&r), "{name}");
            assert_eq!(
                schema_errors(&r, &schema),
                Vec::<String>::new(),
                "{name}: {r}"
            );
        }
    }
    assert!(chain_errors(&rt.event_records()).is_empty());
    let _ = fs::remove_dir_all(dir);
}

fn tempdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("james-agents-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The structural subset of JSON Schema the contract uses: `type`, `const`,
/// `enum`, `required`, `properties`, `additionalProperties`, `items`,
/// `minimum`/`maximum`, `minLength`/`maxLength`, `maxItems`, `oneOf`.
/// (`pattern` needs a regex engine; the Lab's suite checks patterns.)
fn schema_errors(value: &Value, schema: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    check(value, schema, "$", &mut errors);
    errors
}

fn type_ok(v: &Value, t: &str) -> bool {
    match t {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "number" => v.is_number(),
        "integer" => {
            v.as_i64().is_some()
                || v.as_u64().is_some()
                || v.as_f64().is_some_and(|f| f.fract() == 0.0)
        }
        "boolean" => v.is_boolean(),
        "null" => v.is_null(),
        _ => false,
    }
}

fn check(v: &Value, s: &Value, path: &str, errors: &mut Vec<String>) {
    if let Some(t) = s.get("type") {
        let types: Vec<&str> = match t {
            Value::String(t) => vec![t.as_str()],
            Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
            _ => vec![],
        };
        if !types.iter().any(|t| type_ok(v, t)) {
            errors.push(format!("{path}: not of type {types:?}"));
            return;
        }
    }
    if let Some(c) = s.get("const") {
        if v != c {
            errors.push(format!("{path}: expected {c}"));
        }
    }
    if let Some(Value::Array(options)) = s.get("enum") {
        if !options.contains(v) {
            errors.push(format!("{path}: {v} not in enum"));
        }
    }
    if let (Some(text), Some(max)) = (v.as_str(), s.get("maxLength").and_then(Value::as_u64)) {
        if text.chars().count() as u64 > max {
            errors.push(format!("{path}: too long"));
        }
    }
    if let (Some(text), Some(min)) = (v.as_str(), s.get("minLength").and_then(Value::as_u64)) {
        if (text.chars().count() as u64) < min {
            errors.push(format!("{path}: too short"));
        }
    }
    if let Some(n) = v.as_f64() {
        if s.get("minimum")
            .and_then(Value::as_f64)
            .is_some_and(|m| n < m)
        {
            errors.push(format!("{path}: below minimum"));
        }
        if s.get("maximum")
            .and_then(Value::as_f64)
            .is_some_and(|m| n > m)
        {
            errors.push(format!("{path}: above maximum"));
        }
    }
    if let Value::Array(items) = v {
        if s.get("maxItems")
            .and_then(Value::as_u64)
            .is_some_and(|m| items.len() as u64 > m)
        {
            errors.push(format!("{path}: too many items"));
        }
        if let Some(item_schema) = s.get("items") {
            for (i, item) in items.iter().enumerate() {
                check(item, item_schema, &format!("{path}[{i}]"), errors);
            }
        }
    }
    if let Value::Object(map) = v {
        let props = s.get("properties").and_then(Value::as_object);
        for key in s
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !map.contains_key(key) {
                errors.push(format!("{path}: missing {key}"));
            }
        }
        for (key, child) in map {
            match props.and_then(|p| p.get(key)) {
                Some(sub) => check(child, sub, &format!("{path}.{key}"), errors),
                None => match s.get("additionalProperties") {
                    Some(Value::Bool(false)) => errors.push(format!("{path}: unexpected {key}")),
                    Some(sub @ Value::Object(_)) => {
                        check(child, sub, &format!("{path}.{key}"), errors)
                    }
                    _ => {}
                },
            }
        }
    }
    if let Some(Value::Array(options)) = s.get("oneOf") {
        let valid = options
            .iter()
            .filter(|o| schema_errors(v, o).is_empty())
            .count();
        if valid != 1 {
            errors.push(format!("{path}: matches {valid} of oneOf"));
        }
    }
}
