//! The bound tree of a `prose.vale` profile, `.writfold/vale/<name>/`
//! (#132): the user's own `.vale.ini` and styles, unchanged. Every regular
//! file is inventoried as (path, sha256); a symlink, an unreadable entry,
//! more than [`MAX_FILES`] files or more than [`MAX_BYTES`] bytes refuses
//! the profile, so a partial inventory is never bound. The config's
//! top-level `StylesPath` must be relative, without `..`, and name a
//! directory inside the tree. Tested, not proved.
use sha2::{Digest, Sha256};
use std::path::Path;

/// Where profile trees live, under the KB root.
pub const DIR: &str = ".writfold/vale";
/// The config file every tree has.
pub const CONFIG: &str = ".vale.ini";
pub const MAX_FILES: usize = 2000;
pub const MAX_BYTES: u64 = 8 << 20;

/// Every file of profile `name`'s tree, sorted, after checking the
/// config's `StylesPath`.
pub fn inventory(root: &Path, name: &str) -> Result<Vec<(String, String)>, String> {
    let tree = format!("{DIR}/{name}");
    let base = root.join(&tree);
    if !base.is_dir() {
        return Err(format!(
            "no Vale tree: copy your .vale.ini and styles to {tree}/"
        ));
    }
    let mut files = Vec::new();
    let mut bytes = 0u64;
    walk(&base, "", &mut files, &mut bytes).map_err(|why| format!("{tree}: {why}"))?;
    files.sort();
    let config = files.iter().find(|(p, _)| p == CONFIG);
    if config.is_none() {
        return Err(format!("{tree} has no {CONFIG}"));
    }
    let text =
        std::fs::read_to_string(base.join(CONFIG)).map_err(|e| format!("{tree}/{CONFIG}: {e}"))?;
    styles(&base, &text).map_err(|why| format!("{tree}/{CONFIG}: {why}"))?;
    Ok(files)
}

fn walk(
    dir: &Path,
    rel: &str,
    files: &mut Vec<(String, String)>,
    bytes: &mut u64,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{rel}: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{rel}: {e}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = match rel {
            "" => name,
            _ => format!("{rel}/{name}"),
        };
        let kind = entry.file_type().map_err(|e| format!("{path}: {e}"))?;
        if kind.is_symlink() {
            return Err(format!("{path} is a symbolic link"));
        }
        if kind.is_dir() {
            walk(&entry.path(), &path, files, bytes)?;
            continue;
        }
        if !kind.is_file() {
            return Err(format!("{path} is not a regular file"));
        }
        let data = std::fs::read(entry.path()).map_err(|e| format!("{path}: {e}"))?;
        *bytes += data.len() as u64;
        if files.len() >= MAX_FILES || *bytes > MAX_BYTES {
            return Err(format!("more than {MAX_FILES} files or {MAX_BYTES} bytes"));
        }
        files.push((path, crate::index::hex(&Sha256::digest(&data))));
    }
    Ok(())
}

/// The top-level `StylesPath` of `.vale.ini`, checked to stay in the tree.
fn styles(base: &Path, ini: &str) -> Result<String, String> {
    let value = ini
        .lines()
        .map(str::trim)
        .take_while(|l| !l.starts_with('['))
        .find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == "StylesPath").then(|| v.trim().trim_matches('"').to_owned())
        })
        .ok_or("has no top-level StylesPath")?;
    let path = Path::new(&value);
    let escapes = path.components().any(|c| {
        !matches!(
            c,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    });
    if value.is_empty() || escapes {
        return Err(format!(
            "StylesPath {value:?} must be a relative path inside the profile tree"
        ));
    }
    if !base.join(path).is_dir() {
        return Err(format!(
            "StylesPath {value:?} is not a directory in the profile tree"
        ));
    }
    Ok(value)
}
