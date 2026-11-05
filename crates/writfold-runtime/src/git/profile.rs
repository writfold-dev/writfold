use super::GitError;
use rustix::fd::OwnedFd;
use rustix::fs::{Dir, FileType, Mode, OFlags, fstat, openat};
use writfold_domain::{ByteCount, GitObjectFormat};

/// Check every pack-directory entry without following symlinks. The managed
/// metadata stability assumption still applies after these descriptors close;
/// selected objects are independently hash-checked when Git returns their bytes.
pub(super) fn check_packs(
    directory: &OwnedFd,
    format: GitObjectFormat,
    limit: ByteCount,
) -> Result<(), GitError> {
    let width = match format {
        GitObjectFormat::Sha1 => 40,
        GitObjectFormat::Sha256 => 64,
    };
    let mut names = 0_u64;
    for entry in Dir::read_from(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        let bytes = name.to_bytes();
        if matches!(bytes, b"." | b"..") {
            continue;
        }
        names = names
            .checked_add(bytes.len() as u64)
            .ok_or(GitError::Budget {
                stream: "pack metadata",
                limit: limit.get(),
            })?;
        if names > limit.get() {
            return Err(GitError::Budget {
                stream: "pack metadata",
                limit: limit.get(),
            });
        }
        if !pack_name(bytes, width) {
            return Err(GitError::UnsupportedProfile(
                "promisor/unknown pack metadata",
            ));
        }
        let file = openat(
            directory,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )?;
        let stat = fstat(&file)?;
        if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
            return Err(GitError::UnsupportedProfile("nonregular pack metadata"));
        }
    }
    Ok(())
}

fn pack_name(name: &[u8], width: usize) -> bool {
    if name == b"multi-pack-index" {
        return true;
    }
    let (name, bitmap_only) = if let Some(name) = name.strip_prefix(b"pack-") {
        (name, false)
    } else if let Some(name) = name.strip_prefix(b"multi-pack-index-") {
        (name, true)
    } else {
        return false;
    };
    let Some((hash, suffix)) = name.split_at_checked(width) else {
        return false;
    };
    hash.iter()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        && if bitmap_only {
            suffix == b".bitmap"
        } else {
            matches!(
                suffix,
                b".pack" | b".idx" | b".rev" | b".bitmap" | b".keep" | b".mtimes"
            )
        }
}

#[derive(Clone, Copy)]
enum Section {
    Core,
    Extensions,
    Remote,
    Branch,
    User,
}

pub(super) fn check_config(bytes: &[u8], format: GitObjectFormat) -> Result<(), GitError> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| GitError::UnsupportedProfile("non-UTF8 config"))?;
    let mut section = None;
    let mut version = None;
    let mut object_format = None;
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            section = Some(parse_section(line)?);
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or(GitError::UnsupportedProfile("config syntax"))?;
        let (key, value) = (key.trim(), value.trim());
        check_scalar(value)?;
        match section {
            Some(Section::Core)
                if key.eq_ignore_ascii_case("repositoryformatversion")
                    && version.is_none()
                    && matches!(value, "0" | "1") =>
            {
                version = Some(value);
            }
            Some(Section::Core)
                if any_key(key, &["filemode", "bare", "logallrefupdates"])
                    && matches!(value, "true" | "false") => {}
            Some(Section::Extensions)
                if key.eq_ignore_ascii_case("objectformat")
                    && object_format.is_none()
                    && matches!(value, "sha1" | "sha256") =>
            {
                object_format = Some(value);
            }
            // These values are inert to the closed local object-read vocabulary.
            // No command resolves a remote, invokes transport, or creates commits.
            Some(Section::Remote) if any_key(key, &["url", "fetch", "pushurl", "tagopt"]) => {}
            Some(Section::Branch) if any_key(key, &["remote", "merge", "pushremote", "rebase"]) => {
            }
            Some(Section::User)
                if any_key(key, &["name", "email", "signingkey", "useconfigonly"]) => {}
            _ => return Err(GitError::UnsupportedProfile("config key/value")),
        }
    }
    match (format, version, object_format) {
        (GitObjectFormat::Sha1, Some("0"), None)
        | (GitObjectFormat::Sha1, Some("1"), Some("sha1"))
        | (GitObjectFormat::Sha256, Some("1"), Some("sha256")) => Ok(()),
        _ => Err(GitError::ObjectFormat),
    }
}

fn any_key(key: &str, allowed: &[&str]) -> bool {
    allowed
        .iter()
        .any(|candidate| key.eq_ignore_ascii_case(candidate))
}

fn parse_section(line: &str) -> Result<Section, GitError> {
    if line.eq_ignore_ascii_case("[core]") {
        return Ok(Section::Core);
    }
    if line.eq_ignore_ascii_case("[extensions]") {
        return Ok(Section::Extensions);
    }
    if line.eq_ignore_ascii_case("[user]") {
        return Ok(Section::User);
    }
    let inner = line
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or(GitError::UnsupportedProfile("config section syntax"))?;
    let (section, name) = inner
        .split_once(' ')
        .ok_or(GitError::UnsupportedProfile("config section"))?;
    let name = name
        .trim()
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or(GitError::UnsupportedProfile("config subsection syntax"))?;
    if name.is_empty()
        || name
            .bytes()
            .any(|byte| matches!(byte, b'\\' | b'"') || byte.is_ascii_control())
    {
        return Err(GitError::UnsupportedProfile("config subsection escapes"));
    }
    if section.eq_ignore_ascii_case("remote") {
        return Ok(Section::Remote);
    }
    if section.eq_ignore_ascii_case("branch") {
        return Ok(Section::Branch);
    }
    Err(GitError::UnsupportedProfile("config section"))
}

fn check_scalar(value: &str) -> Result<(), GitError> {
    let mut quoted = false;
    let mut bytes = value.bytes();
    while let Some(byte) = bytes.next() {
        match byte {
            b'"' => quoted = !quoted,
            b'\\' => {
                if !matches!(bytes.next(), Some(b'n' | b't' | b'b' | b'\\' | b'"')) {
                    return Err(GitError::UnsupportedProfile(
                        "config scalar escape/continuation",
                    ));
                }
            }
            b'#' | b';' if !quoted => break,
            byte if byte.is_ascii_control() && byte != b'\t' => {
                return Err(GitError::UnsupportedProfile("config scalar control byte"));
            }
            _ => {}
        }
    }
    if quoted {
        return Err(GitError::UnsupportedProfile("config scalar quote"));
    }
    Ok(())
}
