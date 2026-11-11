//! Create-only model proposals, for one explicitly selected directory.
//!
//! Existing paths are never overwritten. Path checks are not a sandbox against
//! another process concurrently replacing the selected directory.
use james_models::ChatMessage;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_NAME_BYTES: usize = 128;
const MAX_INSTRUCTION_BYTES: usize = 4096;
const MAX_CONTENT_BYTES: usize = 24 * 1024;
const MAX_RESPONSE_BYTES: usize = 32768;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

const SYSTEM_PROMPT: &str = "Create a UTF-8 text file according to the user's request. Reply with exactly one JSON object with one field named content containing the entire file text. Do not use Markdown fences or add fields. Do not request tools, file paths, or shell commands to execute. The user will review the exact text before the file is created.";

#[derive(Debug)]
pub struct CreateWorkspace {
    root: PathBuf,
}

/// The user-selected destination and instruction are frozen before generation.
#[derive(Debug)]
pub struct CreateRequest {
    root: PathBuf,
    name: String,
    instruction: String,
}

/// Only constructed from a valid response to a specific request. Callers can
/// display its fields, but cannot mutate the reviewed path or content.
#[derive(Debug, Clone)]
pub struct CreateProposal {
    root: PathBuf,
    name: String,
    content: String,
}

#[derive(Serialize)]
struct Prompt<'a> {
    filename: &'a str,
    instruction: &'a str,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    content: String,
}

impl CreateWorkspace {
    pub fn open(root: &Path) -> Result<Self, String> {
        let root = fs::canonicalize(root).map_err(|e| format!("Cannot open directory: {e}"))?;
        let root_text = root
            .to_str()
            .ok_or("Choose a directory with a reviewable UTF-8 path.")?;
        if root_text.chars().any(james_platform::is_unreviewable) {
            return Err(
                "Choose a directory without hidden or control characters in its path.".into(),
            );
        }
        let workspace = Self { root };
        workspace.check_root()?;
        Ok(workspace)
    }

    pub fn request(&self, name: &str, instruction: &str) -> Result<CreateRequest, String> {
        validate_name(name)?;
        if instruction.trim().is_empty()
            || instruction.len() > MAX_INSTRUCTION_BYTES
            || instruction.contains('\0')
        {
            return Err(
                "Enter an instruction between 1 and 4096 bytes, without NUL characters.".into(),
            );
        }
        self.check_root()?;
        if fs::symlink_metadata(self.root.join(name)).is_ok() {
            return Err("A file or directory with that name already exists.".into());
        }
        Ok(CreateRequest {
            root: self.root.clone(),
            name: name.to_owned(),
            instruction: instruction.to_owned(),
        })
    }

    /// Creates a new file without replacing an existing directory entry. The
    /// name and text must be taken from the same reviewed proposal.
    pub fn create(&self, proposal: CreateProposal) -> Result<PathBuf, String> {
        if proposal.root != self.root {
            return Err("The proposal belongs to another directory.".into());
        }
        validate_name(&proposal.name)?;
        validate_content(&proposal.content)?;
        self.check_root()?;
        let target = self.root.join(&proposal.name);
        let (temp, mut file) = self.create_temp()?;
        let result = (|| {
            file.write_all(proposal.content.as_bytes())
                .map_err(|e| format!("Could not write file: {e}"))?;
            file.sync_all()
                .map_err(|e| format!("Could not save file: {e}"))?;
            drop(file);
            self.check_root()?;
            // Link creation fails atomically if target exists. It never replaces
            // an existing entry, including an entry created during generation.
            fs::hard_link(&temp, &target).map_err(|e| format!("Could not create file: {e}"))?;
            Ok(target)
        })();
        let _ = fs::remove_file(temp);
        result
    }

    fn check_root(&self) -> Result<(), String> {
        let meta = fs::symlink_metadata(&self.root)
            .map_err(|e| format!("Cannot access selected directory: {e}"))?;
        if !meta.is_dir() || meta.file_type().is_symlink() || is_reparse_point(&meta) {
            return Err(
                "The selected directory is no longer available as a normal directory.".into(),
            );
        }
        if fs::canonicalize(&self.root).map_err(|e| e.to_string())? != self.root {
            return Err("The selected directory changed; select it again.".into());
        }
        Ok(())
    }

    fn create_temp(&self) -> Result<(PathBuf, fs::File), String> {
        for _ in 0..32 {
            let n = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = self
                .root
                .join(format!(".james-create-{}-{n}.tmp", std::process::id()));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&path) {
                Ok(file) => return Ok((path, file)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(format!("Could not prepare the new file: {e}")),
            }
        }
        Err("Could not reserve a temporary file in the selected directory.".into())
    }
}

impl CreateRequest {
    pub fn messages(&self) -> Result<Vec<ChatMessage>, String> {
        let payload = serde_json::to_string(&Prompt {
            filename: &self.name,
            instruction: &self.instruction,
        })
        .map_err(|e| format!("Could not prepare request: {e}"))?;
        Ok(vec![
            ChatMessage::new("system", SYSTEM_PROMPT),
            ChatMessage::new("user", payload),
        ])
    }

    /// Reject code fences, non-object replies, unexpected fields and oversized text.
    pub fn parse_response(self, response: &str) -> Result<CreateProposal, String> {
        if response.len() > MAX_RESPONSE_BYTES {
            return Err("Model response is too large.".into());
        }
        let response: Response = serde_json::from_str(response)
            .map_err(|_| "Model did not return the requested JSON object.")?;
        validate_content(&response.content)?;
        Ok(CreateProposal {
            root: self.root,
            name: self.name,
            content: response.content,
        })
    }
}

impl CreateProposal {
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn filename(&self) -> &str {
        &self.name
    }
    pub fn content(&self) -> &str {
        &self.content
    }
}

fn validate_content(content: &str) -> Result<(), String> {
    if content.len() > MAX_CONTENT_BYTES {
        return Err("Generated file exceeds 24 KiB.".into());
    }
    let mut chars = content.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            if chars.next() != Some('\n') {
                return Err("Generated text contains a carriage return outside CRLF.".into());
            }
        } else if !matches!(c, '\n' | '\t') && james_platform::is_unreviewable(c) {
            return Err(
                "Generated text contains hidden or control characters that cannot be reviewed."
                    .into(),
            );
        }
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > MAX_NAME_BYTES
        || name.ends_with([' ', '.'])
        || name.chars().any(|c| {
            james_platform::is_unreviewable(c)
                || matches!(c, '/' | '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*')
        })
        || !matches!(
            (
                Path::new(name).components().next(),
                Path::new(name).components().nth(1)
            ),
            (Some(Component::Normal(_)), None)
        )
    {
        return Err(
            "Choose a single valid filename without a directory or special characters.".into(),
        );
    }
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if matches!(
        stem.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    ) {
        return Err("That filename is reserved by Windows.".into());
    }
    Ok(())
}
#[cfg(windows)]
fn is_reparse_point(meta: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    meta.file_attributes() & 0x400 != 0
}
#[cfg(not(windows))]
fn is_reparse_point(_: &fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let n = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("james-create-test-{}-{n}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn review_is_bound_and_creates_exact_text() {
        let f = Fixture::new();
        let w = CreateWorkspace::open(&f.0).unwrap();
        let req = w.request("hello.txt", "Write a greeting").unwrap();
        let messages = req.messages().unwrap();
        assert_eq!(messages.len(), 2);
        let p = req.parse_response(r#"{"content":"hello\nworld"}"#).unwrap();
        assert_eq!(p.filename(), "hello.txt");
        assert_eq!(p.root(), w.root.as_path());
        assert_eq!(p.content(), "hello\nworld");
        assert!(!f.0.join("hello.txt").exists());
        let path = w.create(p).unwrap();
        assert_eq!(fs::read_to_string(path).unwrap(), "hello\nworld");
        assert_eq!(fs::read_dir(&f.0).unwrap().count(), 1);
    }
    #[test]
    fn existing_or_racing_target_is_never_overwritten() {
        let f = Fixture::new();
        let w = CreateWorkspace::open(&f.0).unwrap();
        let p = w
            .request("file", "write")
            .unwrap()
            .parse_response(r#"{"content":"new"}"#)
            .unwrap();
        fs::write(f.0.join("file"), "original").unwrap();
        assert!(w.create(p).is_err());
        assert_eq!(fs::read_to_string(f.0.join("file")).unwrap(), "original");
        assert!(w.request("file", "write").is_err());
        assert_eq!(fs::read_dir(&f.0).unwrap().count(), 1);
    }
    #[test]
    fn forbids_invalid_paths_and_bad_model_responses() {
        let f = Fixture::new();
        let w = CreateWorkspace::open(&f.0).unwrap();
        for name in [
            "../outside",
            "/root",
            "a/b",
            "a\\b",
            "bad:stream",
            "NUL.txt",
            "..",
            "file.",
        ] {
            assert!(w.request(name, "hi").is_err(), "{name}");
        }
        for reply in [
            r#"{"content":"ok","path":"escape"}"#,
            r#"{"text":"hello"}"#,
            "```json\n{}\n```",
            r#"{"content":"\u0000"}"#,
        ] {
            assert!(
                w.request("file", "hi")
                    .unwrap()
                    .parse_response(reply)
                    .is_err()
            );
        }
        let huge = serde_json::json!({"content": "x".repeat(MAX_CONTENT_BYTES + 1)}).to_string();
        assert!(
            w.request("file", "hi")
                .unwrap()
                .parse_response(&huge)
                .is_err()
        );
    }
    #[test]
    fn proposal_cannot_move_to_another_root() {
        let a = Fixture::new();
        let b = Fixture::new();
        let wa = CreateWorkspace::open(&a.0).unwrap();
        let wb = CreateWorkspace::open(&b.0).unwrap();
        let p = wa
            .request("file", "write")
            .unwrap()
            .parse_response(r#"{"content":"x"}"#)
            .unwrap();
        assert!(wb.create(p).is_err());
        assert!(!b.0.join("file").exists());
    }

    #[test]
    fn rejects_text_that_cannot_be_reviewed() {
        let f = Fixture::new();
        let w = CreateWorkspace::open(&f.0).unwrap();
        let hidden_root = f.0.join("hidden\u{202e}dir");
        fs::create_dir(&hidden_root).unwrap();
        assert!(CreateWorkspace::open(&hidden_root).is_err());
        for bad in ["x\u{202e}y", "x\u{200b}y", "x\ry", "x\u{7}y"] {
            let reply = serde_json::json!({"content": bad}).to_string();
            assert!(
                w.request("safe.txt", "write")
                    .unwrap()
                    .parse_response(&reply)
                    .is_err()
            );
        }
        assert!(w.request("bad\u{202e}txt", "write").is_err());
        assert!(
            w.request("safe.txt", "write")
                .unwrap()
                .parse_response(r#"{"content":"a\r\nb\nc\tend"}"#)
                .is_ok()
        );
    }
}
