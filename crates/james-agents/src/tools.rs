//! Bounded operations on one workspace folder the person chose.
//!
//! Paths are relative, with no `..`, root or drive components, and are
//! resolved through the file system so a symbolic link or junction cannot
//! lead outside the folder. Reads and listings are size-bounded. Writes are
//! never made here on an agent's say-so: [`Workspace::apply_write`] is called
//! only by the runtime after the person approved the exact content, and it
//! refuses if the file changed since the proposal.
//!
//! This is host-code enforcement, not an OS sandbox: another program can
//! still change the folder between a check and a use.

use crate::canonical::sha256_hex;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

pub const MAX_READ_BYTES: usize = 64 * 1024;
pub const MAX_WRITE_BYTES: usize = 64 * 1024;
pub const MAX_LIST: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolError(pub String);

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ToolError {}

fn err(message: impl Into<String>) -> ToolError {
    ToolError(message.into())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRead {
    pub path: String,
    pub text: String,
    pub truncated: bool,
    /// SHA-256 of the whole file's bytes, not only the part shown.
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    pub path: String,
    pub line: usize,
    pub quote: Option<String>,
    /// The line exists in the file.
    pub exists: bool,
    /// The quoted words appear on that line, when a quote was given.
    pub quote_matches: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
}

/// Validates a relative path's shape before touching the file system.
pub fn clean_relative(path: &str) -> Result<PathBuf, ToolError> {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "." {
        return Ok(PathBuf::new());
    }
    if trimmed.len() > 400 || trimmed.contains('\0') {
        return Err(err("path is too long or not text"));
    }
    let candidate = Path::new(trimmed);
    let mut out = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            _ => {
                return Err(err(
                    "use a path inside the workspace folder, without '..' or a drive",
                ));
            }
        }
    }
    Ok(out)
}

impl Workspace {
    pub fn new(root: &Path) -> Result<Self, ToolError> {
        let root = fs::canonicalize(root).map_err(|e| err(format!("workspace folder: {e}")))?;
        if !root.is_dir() {
            return Err(err("the workspace is not a folder"));
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The canonical path of an existing entry, if it is inside the root.
    fn existing(&self, relative: &Path) -> Result<PathBuf, ToolError> {
        let full = fs::canonicalize(self.root.join(relative))
            .map_err(|_| err("no such file or folder"))?;
        if !full.starts_with(&self.root) {
            return Err(err("that path leads outside the workspace folder"));
        }
        Ok(full)
    }

    pub fn list(&self, path: &str) -> Result<Vec<Entry>, ToolError> {
        let dir = self.existing(&clean_relative(path)?)?;
        let mut out = Vec::new();
        for entry in fs::read_dir(&dir).map_err(|e| err(format!("cannot list: {e}")))? {
            let entry = entry.map_err(|e| err(format!("cannot list: {e}")))?;
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue; // hidden files and VCS folders stay out of view
            }
            out.push(Entry {
                name,
                is_dir: meta.is_dir(),
                bytes: meta.len(),
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out.truncate(MAX_LIST);
        Ok(out)
    }

    pub fn read(&self, path: &str) -> Result<FileRead, ToolError> {
        let relative = clean_relative(path)?;
        let full = self.existing(&relative)?;
        if !full.is_file() {
            return Err(err("that is a folder, not a file"));
        }
        let mut bytes = Vec::new();
        fs::File::open(&full)
            .and_then(|f| f.take(4 * 1024 * 1024).read_to_end(&mut bytes))
            .map_err(|e| err(format!("cannot read: {e}")))?;
        if bytes.contains(&0) {
            return Err(err("binary file; only text files can be read"));
        }
        let sha256 = sha256_hex(&bytes);
        let truncated = bytes.len() > MAX_READ_BYTES;
        let shown = &bytes[..bytes.len().min(MAX_READ_BYTES)];
        Ok(FileRead {
            path: relative.to_string_lossy().replace('\\', "/"),
            text: String::from_utf8_lossy(shown).into_owned(),
            truncated,
            sha256,
        })
    }

    /// SHA-256 of a file's current bytes, or None when it does not exist.
    pub fn digest_of(&self, path: &str) -> Result<Option<String>, ToolError> {
        let relative = clean_relative(path)?;
        let target = self.write_target(&relative)?;
        match fs::read(&target) {
            Ok(bytes) => Ok(Some(sha256_hex(&bytes))),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(err(format!("cannot read: {e}"))),
        }
    }

    /// Where a write would land, checked against the root through the
    /// deepest existing ancestor (the file itself may not exist yet).
    fn write_target(&self, relative: &Path) -> Result<PathBuf, ToolError> {
        if relative.as_os_str().is_empty() {
            return Err(err("name a file"));
        }
        let mut existing = self.root.join(relative);
        let mut rest = Vec::new();
        while !existing.exists() {
            let name = existing
                .file_name()
                .ok_or_else(|| err("bad path"))?
                .to_owned();
            rest.push(name);
            if !existing.pop() {
                return Err(err("bad path"));
            }
        }
        let mut full = fs::canonicalize(&existing).map_err(|e| err(format!("bad path: {e}")))?;
        if !full.starts_with(&self.root) {
            return Err(err("that path leads outside the workspace folder"));
        }
        for part in rest.into_iter().rev() {
            full.push(part);
        }
        if full.is_dir() {
            return Err(err("that is a folder, not a file"));
        }
        Ok(full)
    }

    /// Writes approved content, only if the file still has `expected_previous`
    /// (None: it must still not exist). Returns the new content's digest.
    pub fn apply_write(
        &self,
        path: &str,
        content: &str,
        expected_previous: Option<&str>,
    ) -> Result<String, ToolError> {
        if content.len() > MAX_WRITE_BYTES {
            return Err(err("the content is larger than James writes"));
        }
        let relative = clean_relative(path)?;
        let target = self.write_target(&relative)?;
        let current = match fs::read(&target) {
            Ok(bytes) => Some(sha256_hex(&bytes)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(err(format!("cannot read: {e}"))),
        };
        if current.as_deref() != expected_previous {
            return Err(err(
                "the file changed after the proposal; nothing was written",
            ));
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| err(format!("cannot create folder: {e}")))?;
            // Re-check after creating folders: a link could have appeared.
            let parent = fs::canonicalize(parent).map_err(|e| err(format!("bad folder: {e}")))?;
            if !parent.starts_with(&self.root) {
                return Err(err("that path leads outside the workspace folder"));
            }
        }
        let tmp = target.with_extension("james-tmp");
        fs::write(&tmp, content).map_err(|e| err(format!("cannot write: {e}")))?;
        fs::rename(&tmp, &target).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            err(format!("cannot write: {e}"))
        })?;
        Ok(sha256_hex(content.as_bytes()))
    }

    /// Checks `[path:line]` citations (optionally preceded by a quote in
    /// double quotes) against the files. At most 40 are checked.
    pub fn verify_citations(&self, text: &str) -> Vec<Citation> {
        parse_citations(text)
            .into_iter()
            .take(40)
            .map(|(path, line, quote)| {
                let lines: Option<Vec<String>> = self
                    .read(&path)
                    .ok()
                    .map(|r| r.text.lines().map(str::to_owned).collect());
                let found = lines
                    .as_ref()
                    .and_then(|l| line.checked_sub(1).and_then(|i| l.get(i)));
                let quote_matches = quote
                    .as_ref()
                    .map(|q| found.is_some_and(|l| normalize(l).contains(&normalize(q))));
                Citation {
                    path,
                    line,
                    quote,
                    exists: found.is_some(),
                    quote_matches,
                }
            })
            .collect()
    }
}

fn normalize(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// `"quoted words" [docs/a.md:12]` or `[docs/a.md:12]`.
pub fn parse_citations(text: &str) -> Vec<(String, usize, Option<String>)> {
    let mut out = Vec::new();
    let mut rest = text;
    let mut consumed = 0;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else { break };
        let inner = &after[..close];
        let parsed = inner
            .rsplit_once(':')
            .and_then(|(path, line)| line.trim().parse::<usize>().ok().map(|n| (path.trim(), n)))
            .filter(|(path, line)| {
                !path.is_empty() && !path.contains(char::is_whitespace) && *line > 0
            });
        if let Some((path, line)) = parsed {
            let before = &text[..consumed + open];
            let quote = before.trim_end().strip_suffix('"').and_then(|b| {
                b.rfind('"')
                    .map(|q| b[q + 1..].to_owned())
                    .filter(|q| !q.trim().is_empty() && q.len() < 400)
            });
            out.push((path.to_owned(), line, quote));
        }
        consumed += open + 1 + close + 1;
        rest = &after[close + 1..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_escaping_shapes() {
        for bad in ["../x", "/etc/passwd", "a/../../b"] {
            assert!(clean_relative(bad).is_err(), "{bad}");
        }
        #[cfg(windows)]
        assert!(clean_relative("C:\\Windows").is_err());
        assert_eq!(
            clean_relative("docs/./a.md").unwrap(),
            PathBuf::from("docs/a.md")
        );
    }

    #[test]
    fn finds_citations_and_quotes() {
        let c = parse_citations("It says \"install Rust\" [docs/setup.md:3] and [README.md:10].");
        assert_eq!(c.len(), 2);
        assert_eq!(
            c[0],
            ("docs/setup.md".into(), 3, Some("install Rust".into()))
        );
        assert_eq!(c[1], ("README.md".into(), 10, None));
        assert!(parse_citations("[not a citation] [x:0]").is_empty());
    }
}
