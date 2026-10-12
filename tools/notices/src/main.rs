//! Writes the third-party notices that ship with every James package.
//!
//! For each crate compiled into the desktop app on the given targets, the
//! file lists the license its authors declare and reproduces the license
//! texts the crate itself publishes. Identical texts are printed once. A
//! crate that publishes no license file gets the standard text of each
//! license it declares, from `installer/licenses`; a license without a
//! stored text is an error, so a new dependency can't slip through silently.
//!
//! ```sh
//! cargo run -p james-notices --locked -- --target x86_64-unknown-linux-gnu --output THIRD-PARTY-NOTICES.txt
//! ```

use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const APP: &str = "james-desktop";
/// Which release of the SPDX License List the stored standard texts come from.
const SPDX_RELEASE: &str = "3.29.0";
/// Larger files are not license texts.
const MAX_TEXT: u64 = 256 * 1024;

fn main() {
    if let Err(e) = run() {
        eprintln!("james-notices: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut targets = Vec::new();
    let mut output = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--target" => targets.push(args.next().ok_or("--target needs a target triple")?),
            "--output" => output = Some(PathBuf::from(args.next().ok_or("--output needs a path")?)),
            _ => {
                return Err(format!(
                    "unexpected argument {arg}; usage: --target <triple>... --output <file>"
                ));
            }
        }
    }
    let output = output.ok_or("--output is required")?;
    if targets.is_empty() {
        return Err("give at least one --target".into());
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut packages = BTreeMap::new();
    let mut version = String::new();
    for target in &targets {
        let metadata = cargo_metadata(&root, target)?;
        version = app_version(&metadata)?;
        for package in shipped_packages(&metadata)? {
            packages.insert((package.name.clone(), package.version.clone()), package);
        }
    }
    let standard = root.join("installer/licenses");
    let text = render(
        &version,
        &targets,
        packages.into_values().collect(),
        &standard,
    )?;
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    fs::write(&output, text).map_err(|e| format!("{}: {e}", output.display()))?;
    Ok(())
}

fn cargo_metadata(root: &Path, target: &str) -> Result<Value, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .current_dir(root)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--filter-platform",
            target,
        ])
        .output()
        .map_err(|e| format!("could not run cargo metadata: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo metadata failed for {target}: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("cargo metadata output: {e}"))
}

fn app_version(metadata: &Value) -> Result<String, String> {
    metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|p| p["name"] == APP)
        .and_then(|p| p["version"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("{APP} is not in the workspace"))
}

#[derive(Debug, Clone, PartialEq)]
struct Package {
    name: String,
    version: String,
    license: String,
    repository: Option<String>,
    authors: Vec<String>,
    dir: PathBuf,
    license_file: Option<String>,
}

/// Every non-workspace package reachable from the app through normal
/// (not build or dev) dependencies on this target.
fn shipped_packages(metadata: &Value) -> Result<Vec<Package>, String> {
    let all: BTreeMap<&str, &Value> = metadata["packages"]
        .as_array()
        .ok_or("no packages in cargo metadata")?
        .iter()
        .filter_map(|p| Some((p["id"].as_str()?, p)))
        .collect();
    let nodes: BTreeMap<&str, &Value> = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or("no dependency graph in cargo metadata")?
        .iter()
        .filter_map(|n| Some((n["id"].as_str()?, n)))
        .collect();
    let workspace: BTreeSet<&str> = metadata["workspace_members"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let app = all
        .iter()
        .find(|(id, p)| workspace.contains(*id) && p["name"] == APP)
        .map(|(id, _)| *id)
        .ok_or_else(|| format!("{APP} is not in the workspace"))?;
    let mut seen = BTreeSet::new();
    let mut stack = vec![app];
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        let Some(node) = nodes.get(id) else { continue };
        for dep in node["deps"].as_array().into_iter().flatten() {
            let normal = dep["dep_kinds"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|kind| kind["kind"].is_null());
            if let (true, Some(pkg)) = (normal, dep["pkg"].as_str()) {
                stack.push(pkg);
            }
        }
    }
    seen.into_iter()
        .filter(|id| !workspace.contains(id))
        .map(|id| {
            let p = all.get(id).ok_or_else(|| format!("unknown package {id}"))?;
            let text = |key: &str| p[key].as_str().map(str::to_owned);
            Ok(Package {
                name: text("name").unwrap_or_default(),
                version: text("version").unwrap_or_default(),
                license: text("license").unwrap_or_default(),
                repository: text("repository"),
                authors: p["authors"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
                dir: Path::new(p["manifest_path"].as_str().unwrap_or_default())
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_default(),
                license_file: text("license_file"),
            })
        })
        .collect()
}

/// Whether a file name looks like a license or notice text.
fn is_license_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    // Source code that happens to have such a name (`copying.rs`) is not a text.
    let code = [".rs", ".c", ".h", ".cpp", ".py", ".js", ".toml", ".json"];
    if code.iter().any(|ext| lower.ends_with(ext)) {
        return false;
    }
    [
        "licen",
        "copying",
        "copyright",
        "notice",
        "unlicense",
        "ofl.",
        "ufl.",
    ]
    .iter()
    .any(|start| lower.starts_with(start))
        || lower.ends_with("-license.txt")
        || lower.ends_with("_license.txt")
}

/// The license texts a crate publishes: matching files at its root and in
/// subfolders (fonts keep theirs beside the font files), plus a declared
/// `license-file`. Paths are relative to the crate and sorted.
fn license_files(package: &Package) -> Vec<PathBuf> {
    let mut found = BTreeSet::new();
    let mut folders = vec![(package.dir.clone(), 0)];
    while let Some((dir, depth)) = folders.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        let has_fonts = entries.iter().any(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("ttf") || e.eq_ignore_ascii_case("otf"))
        });
        for path in entries {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if path.is_dir() {
                let skip = ["target", "tests", "benches", "examples", ".git"];
                if depth < 2 && !skip.contains(&name) {
                    folders.push((path, depth + 1));
                }
            } else if is_license_name(name)
                || (has_fonts && name.to_ascii_lowercase().ends_with(".txt"))
            {
                found.insert(path);
            }
        }
    }
    if let Some(file) = &package.license_file {
        let path = package.dir.join(file);
        if path.is_file() {
            found.insert(path);
        }
    }
    found
        .into_iter()
        .filter(|p| fs::metadata(p).is_ok_and(|m| m.len() <= MAX_TEXT))
        .filter_map(|p| p.strip_prefix(&package.dir).ok().map(Path::to_path_buf))
        .collect()
}

/// The SPDX identifiers in a license expression, including the old
/// `MIT/Apache-2.0` form and `WITH` exceptions.
fn license_ids(expression: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for token in expression.split(|c: char| c.is_whitespace() || "()/".contains(c)) {
        if token.is_empty() || ["OR", "AND", "WITH"].contains(&token) {
            continue;
        }
        let id = token.trim_end_matches('+').to_owned();
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

fn read_text(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let text = String::from_utf8_lossy(&bytes).replace("\r\n", "\n");
    Ok(text.trim_end().to_owned())
}

fn render(
    version: &str,
    targets: &[String],
    packages: Vec<Package>,
    standard: &Path,
) -> Result<String, String> {
    const RULE: &str =
        "================================================================================";
    // Each distinct text once, in order of first use, with the crates using it.
    let mut texts: Vec<(String, String, Vec<String>)> = Vec::new();
    let mut needed_standard = BTreeSet::new();
    let mut entries = String::new();
    for package in &packages {
        let label = format!("{} {}", package.name, package.version);
        entries.push_str(&format!("\n{label}\n  License: {}\n", package.license));
        if let Some(repository) = &package.repository {
            entries.push_str(&format!("  Source: {repository}\n"));
        }
        if !package.authors.is_empty() {
            entries.push_str(&format!("  Authors: {}\n", package.authors.join(", ")));
        }
        let files = license_files(package);
        if files.is_empty() {
            let ids = license_ids(&package.license);
            if ids.is_empty() {
                return Err(format!(
                    "{label} declares no license and publishes no license file"
                ));
            }
            entries.push_str(&format!(
                "  License texts: none published in the crate; see the standard text of {} below\n",
                ids.join(", ")
            ));
            needed_standard.extend(ids);
            continue;
        }
        let mut refs = Vec::new();
        for file in files {
            let text = read_text(&package.dir.join(&file))?;
            let shown = file.to_string_lossy().replace('\\', "/");
            let index = match texts.iter().position(|(t, _, _)| *t == text) {
                Some(i) => {
                    texts[i].2.push(label.clone());
                    i
                }
                None => {
                    texts.push((text, format!("{label}: {shown}"), Vec::new()));
                    texts.len() - 1
                }
            };
            refs.push(format!("[{}] {shown}", index + 1));
        }
        entries.push_str(&format!("  License texts: {}\n", refs.join(", ")));
    }
    let mut out = format!(
        "THIRD-PARTY SOFTWARE NOTICES FOR JAMES {version}\n\n\
         James is free software under the MIT license (see LICENSE). The James\n\
         program built for {} also contains the {} open-source Rust crates\n\
         listed below, each under the license its authors chose. Each entry names\n\
         the crate's declared license and the license texts the crate publishes;\n\
         identical texts are printed once under LICENSE TEXTS. Where a crate\n\
         publishes no license file, the standard text of each license it declares\n\
         follows under STANDARD LICENSE TEXTS (SPDX License List {SPDX_RELEASE}).\n\
         This file is generated from Cargo.lock by tools/notices.\n\n{RULE}\nCRATES\n{RULE}\n",
        targets.join(", "),
        packages.len()
    );
    out.push_str(&entries);
    out.push_str(&format!("\n{RULE}\nLICENSE TEXTS\n{RULE}\n"));
    for (index, (text, origin, others)) in texts.iter().enumerate() {
        out.push_str(&format!("\n---- [{}] {origin}\n", index + 1));
        if !others.is_empty() {
            out.push_str(&format!(
                "     Same text published by: {}\n",
                others.join(", ")
            ));
        }
        out.push_str(&format!("\n{text}\n"));
    }
    if !needed_standard.is_empty() {
        out.push_str(&format!("\n{RULE}\nSTANDARD LICENSE TEXTS\n{RULE}\n"));
        for id in &needed_standard {
            let path = standard.join(format!("{id}.txt"));
            if !path.is_file() {
                return Err(format!(
                    "no stored text for {id}: add {} from the SPDX License List",
                    path.display()
                ));
            }
            out.push_str(&format!("\n---- {id}\n\n{}\n", read_text(&path)?));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("james-notices-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn package(name: &str, license: &str, dir: &Path) -> Package {
        Package {
            name: name.into(),
            version: "1.0.0".into(),
            license: license.into(),
            repository: Some(format!("https://example.invalid/{name}")),
            authors: vec!["A. Author".into()],
            dir: dir.to_path_buf(),
            license_file: None,
        }
    }

    #[test]
    fn reads_every_spdx_form_cargo_accepts() {
        assert_eq!(license_ids("MIT OR Apache-2.0"), ["MIT", "Apache-2.0"]);
        assert_eq!(license_ids("MIT/Apache-2.0"), ["MIT", "Apache-2.0"]);
        assert_eq!(
            license_ids("(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0"),
            ["MIT", "Apache-2.0", "OFL-1.1", "Ubuntu-font-1.0"]
        );
        assert_eq!(
            license_ids("Apache-2.0 WITH LLVM-exception OR MIT"),
            ["Apache-2.0", "LLVM-exception", "MIT"]
        );
        assert_eq!(license_ids("GPL-2.0+"), ["GPL-2.0"]);
    }

    #[test]
    fn finds_license_files_including_font_licenses_in_subfolders() {
        let dir = scratch("find");
        for file in [
            "LICENSE-MIT",
            "LICENSE-APACHE",
            "COPYING",
            "README.md",
            "src/lib.rs",
            "src/copying.rs",
        ] {
            let path = dir.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "text").unwrap();
        }
        fs::create_dir_all(dir.join("fonts")).unwrap();
        for file in [
            "Font.ttf",
            "OFL.txt",
            "Hack-Regular.txt",
            "emoji-mit-license.txt",
        ] {
            fs::write(dir.join("fonts").join(file), "text").unwrap();
        }
        fs::create_dir_all(dir.join("tests")).unwrap();
        fs::write(dir.join("tests/LICENSE"), "fixture").unwrap();
        let found: Vec<String> = license_files(&package("fonts", "MIT", &dir))
            .iter()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();
        assert_eq!(
            found,
            [
                "COPYING",
                "LICENSE-APACHE",
                "LICENSE-MIT",
                "fonts/Hack-Regular.txt",
                "fonts/OFL.txt",
                "fonts/emoji-mit-license.txt"
            ]
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn prints_shared_texts_once_and_standard_texts_for_crates_without_files() {
        let root = scratch("render");
        let (a, b, c, standard) = (
            root.join("a"),
            root.join("b"),
            root.join("c"),
            root.join("std"),
        );
        for dir in [&a, &b, &c, &standard] {
            fs::create_dir_all(dir).unwrap();
        }
        fs::write(a.join("LICENSE-APACHE"), "Apache text\r\n").unwrap();
        fs::write(b.join("LICENSE"), "Apache text\n").unwrap();
        fs::write(standard.join("MIT.txt"), "Standard MIT").unwrap();
        fs::write(standard.join("Zlib.txt"), "Standard Zlib").unwrap();
        let packages = vec![
            package("a", "Apache-2.0", &a),
            package("b", "Apache-2.0", &b),
            package("c", "MIT OR Zlib", &c),
        ];
        let out = render(
            "0.4.0",
            &["x86_64-unknown-linux-gnu".into()],
            packages,
            &standard,
        )
        .unwrap();
        assert!(out.contains("contains the 3 open-source Rust crates"));
        assert_eq!(out.matches("Apache text").count(), 1, "{out}");
        assert!(
            out.contains("---- [1] a 1.0.0: LICENSE-APACHE\n     Same text published by: b 1.0.0")
        );
        assert!(out.contains("b 1.0.0\n  License: Apache-2.0\n  Source: https://example.invalid/b\n  Authors: A. Author\n  License texts: [1] LICENSE"));
        assert!(
            out.contains("none published in the crate; see the standard text of MIT, Zlib below")
        );
        assert!(
            out.contains("---- MIT\n\nStandard MIT") && out.contains("---- Zlib\n\nStandard Zlib")
        );
        assert!(!out.contains('\r'));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_license_without_a_stored_text_is_an_error() {
        let root = scratch("missing");
        let crate_dir = root.join("x");
        fs::create_dir_all(&crate_dir).unwrap();
        let error = render(
            "0.4.0",
            &["t".into()],
            vec![package("x", "BSD-4-Clause", &crate_dir)],
            &root,
        )
        .unwrap_err();
        assert!(error.contains("no stored text for BSD-4-Clause"), "{error}");
        fs::remove_dir_all(root).unwrap();
    }
}
