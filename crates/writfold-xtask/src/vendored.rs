//! Narrow source-export admission, never a general exemption for local dependencies.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, BufReader, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

const COMMIT: &str = "b14c67a7aca9f416c4facdf189ef340d5666351f";
const ARTIFACT: &str = "b4c3574a1874f8441b871b1b901eb5ef51f8829a760237adfd1b2df11a1827e0";
const ENCODING: &str = "SHA256 of UTF-8 JSON files array, sorted object keys, separators comma/colon, no trailing newline; entries in git ls-tree order";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Export {
    schema: String,
    commit: String,
    source_artifact_sha256: String,
    digest_encoding: String,
    files: Vec<SourceFile>,
}

// Declaration order is the frozen canonical JSON key order. Paths/identities
// in this export are ASCII, so serde and the source producer escape identically.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SourceFile {
    git_blob: String,
    git_mode: String,
    path: String,
    sha256: String,
}

/// Every Splicery package admitted from the export, wherever it resolves.
pub fn directory(name: &str) -> Option<&'static Path> {
    admitted(name).map(|package| Path::new(package.directory))
}

struct Admitted {
    directory: &'static str,
    /// `writfold-sources` may declare it; otherwise it is reachable only
    /// through another admitted Splicery package.
    direct: bool,
    /// Exact declared dependency names (all kinds). Justification:
    /// domain hashes captures (sha2); syntax parses into domain types;
    /// compile lowers syntax into domain plans (regex selectors); splice is
    /// the dependency-free byte-splice kernel proved over its Aeneas
    /// translation upstream; kernel is the checked domain facade over splice.
    dependencies: &'static [&'static str],
}

fn admitted(name: &str) -> Option<Admitted> {
    let (directory, direct, dependencies): (_, _, &'static [&'static str]) = match name {
        "splicery-domain" => ("crates/splicery-domain", true, &["sha2"]),
        "splicery-syntax" => ("crates/splicery-syntax", true, &["splicery-domain"]),
        "splicery-compile" => (
            "crates/splicery-compile",
            true,
            &["regex", "splicery-domain", "splicery-syntax"],
        ),
        "splicery-splice" => ("crates/splicery-splice", false, &[]),
        "splicery-kernel" => (
            "crates/splicery-kernel",
            true,
            &["splicery-domain", "splicery-splice"],
        ),
        _ => return None,
    };
    Some(Admitted {
        directory,
        direct,
        dependencies,
    })
}

fn declared_dependencies_match(package: &Value, expected: &[&str]) -> bool {
    package["dependencies"].as_array().is_some_and(|declared| {
        let mut names: Vec<_> = declared.iter().map(|d| d["name"].as_str()).collect();
        names.sort_unstable();
        names.dedup();
        names.len() == expected.len()
            && names
                .iter()
                .zip(expected)
                .all(|(name, wanted)| *name == Some(*wanted))
    })
}

pub struct SpliceryExport {
    root: PathBuf,
}

impl SpliceryExport {
    pub fn admit(root: &Path) -> Result<Self, String> {
        let vendor = root.join("vendor");
        let export_root = vendor.join("splicery");
        for path in [&vendor, &export_root] {
            if !fs::symlink_metadata(path)
                .map_err(|e| e.to_string())?
                .is_dir()
            {
                return Err("source export directory is not a regular directory".into());
            }
        }
        let manifest = export_root.join("provenance.json");
        let info = fs::symlink_metadata(&manifest).map_err(|e| e.to_string())?;
        if !info.is_file() || info.len() > 1_048_576 || info.mode() & 0o7777 != 0o644 {
            return Err("source export manifest kind, size or mode changed".into());
        }
        let export: Export = serde_json::from_reader(BufReader::new(
            File::open(&manifest).map_err(|e| e.to_string())?,
        ))
        .map_err(|e| e.to_string())?;
        if export.schema != "writfold-splicery-source-export/1"
            || export.commit != COMMIT
            || export.source_artifact_sha256 != ARTIFACT
            || export.digest_encoding != ENCODING
            || export.files.iter().any(|f| !f.path.is_ascii())
            || export
                .files
                .windows(2)
                .any(|pair| pair[0].path >= pair[1].path)
        {
            return Err("source export does not name the admitted immutable profile".into());
        }
        let mut hash = Sha256::new();
        serde_json::to_writer(HashWriter(&mut hash), &export.files).map_err(|e| e.to_string())?;
        if !matches_digest(hash, ARTIFACT) {
            return Err("source export file inventory differs from the admitted artifact".into());
        }
        if inspect(&export_root, Path::new(""), &export.files)? != export.files.len() {
            return Err("source export is missing declared files".into());
        }
        Ok(Self { root: export_root })
    }

    pub fn permits(&self, owner: &str, dependency: &Value) -> bool {
        owner == "writfold-sources"
            && dependency["name"]
                .as_str()
                .and_then(admitted)
                .is_some_and(|expected| {
                    expected.direct
                        && dependency["source"].is_null()
                        && dependency["path"].as_str().is_some_and(|path| {
                            Path::new(path).strip_prefix(&self.root).ok()
                                == Some(Path::new(expected.directory))
                        })
                })
    }

    pub fn matches_package(&self, package: &Value) -> bool {
        package["name"]
            .as_str()
            .and_then(admitted)
            .is_some_and(|expected| {
                package["source"].is_null()
                    && declared_dependencies_match(package, expected.dependencies)
                    && package["manifest_path"].as_str().is_some_and(|path| {
                        let path = Path::new(path);
                        path.file_name().is_some_and(|name| name == "Cargo.toml")
                            && path.parent().and_then(|p| p.strip_prefix(&self.root).ok())
                                == Some(Path::new(expected.directory))
                    })
            })
    }
}

fn inspect(root: &Path, relative: &Path, files: &[SourceFile]) -> Result<usize, String> {
    let mut count = 0;
    for entry in fs::read_dir(root.join(relative)).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let relative = relative.join(entry.file_name());
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            if !files
                .iter()
                .any(|f| Path::new(&f.path).starts_with(&relative))
            {
                return Err(format!(
                    "unlisted source export directory: {}",
                    relative.display()
                ));
            }
            count += inspect(root, &relative, files)?;
        } else if kind.is_file() {
            if relative == Path::new("provenance.json") {
                continue;
            }
            let name = relative.to_str().ok_or("non-UTF8 source export path")?;
            let index = files
                .binary_search_by(|f| f.path.as_str().cmp(name))
                .map_err(|_| format!("unlisted source export file: {name}"))?;
            inspect_file(&entry.path(), &files[index])?;
            count += 1;
        } else {
            return Err(format!(
                "source export contains a link or special file: {}",
                relative.display()
            ));
        }
    }
    Ok(count)
}

fn inspect_file(path: &Path, expected: &SourceFile) -> Result<(), String> {
    let mode = match expected.git_mode.as_str() {
        "100644" => 0o644,
        "100755" => 0o755,
        _ => return Err("source export contains an unsupported Git mode".into()),
    };
    let info = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !info.is_file() || info.mode() & 0o7777 != mode {
        return Err(format!(
            "source export kind/mode changed: {}",
            path.display()
        ));
    }
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    io::copy(&mut file, &mut HashWriter(&mut hash)).map_err(|e| e.to_string())?;
    if !matches_digest(hash, &expected.sha256) {
        return Err(format!("source export bytes changed: {}", path.display()));
    }
    Ok(())
}

struct HashWriter<'a>(&'a mut Sha256);
impl Write for HashWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn matches_digest(hash: Sha256, expected: &str) -> bool {
    const HEX: &[u8] = b"0123456789abcdef";
    let digest = hash.finalize();
    expected.len() == digest.len() * 2
        && expected
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .zip(digest)
            .all(|(pair, byte)| {
                pair[0] == HEX[usize::from(byte >> 4)] && pair[1] == HEX[usize::from(byte & 15)]
            })
}

#[cfg(test)]
#[path = "vendored_tests.rs"]
mod tests;
