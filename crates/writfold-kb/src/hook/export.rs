//! `session-start` hands the session's ID to the shell commands the agent
//! runs later: one `export WRITFOLD_SESSION=<id>` line appended to the
//! file Claude Code names in `CLAUDE_ENV_FILE` (it runs that file before
//! each Bash command). Verbs then read the same session the hooks do, so
//! `checkpoint --new-run` writes the session's current-run pointer
//! ([`crate::seat`]) and `propose` counts against the session.
use crate::seat::{SESSION_ENV, session_id};
use serde_json::Value;
use std::io::Write;
use std::path::Path;

/// The line for a session.
pub fn line(session: &str) -> String {
    format!("export {SESSION_ENV}={session}\n")
}

/// Append the session's line to `env_file` unless it already ends the
/// file's exports for the variable. `Ok(false)` when the event names no
/// usable session or the line is already the last one for the variable.
pub fn export_session(env_file: &Path, input: &str) -> Result<bool, String> {
    let Ok(Value::Object(input)) = serde_json::from_str::<Value>(input) else {
        return Ok(false);
    };
    let Some(session) = session_id(input.get("session_id").and_then(Value::as_str)) else {
        return Ok(false);
    };
    let wanted = line(&session);
    let existing = std::fs::read_to_string(env_file).unwrap_or_default();
    let prefix = format!("export {SESSION_ENV}=");
    let last = existing.lines().rev().find(|l| l.starts_with(&prefix));
    if last == Some(wanted.trim_end()) {
        return Ok(false);
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(env_file)
        .map_err(|e| format!("{}: {e}", env_file.display()))?;
    let lead = if existing.is_empty() || existing.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    file.write_all(format!("{lead}{wanted}").as_bytes())
        .map_err(|e| format!("{}: {e}", env_file.display()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::export_session;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("writfold-export-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap_or_default();
        dir.join("env")
    }

    #[test]
    fn the_session_line_is_appended_once_and_follows_a_new_session() {
        let file = scratch("append");
        std::fs::write(&file, "export OTHER=1").unwrap_or_default();
        let event =
            |id: &str| format!(r#"{{"hook_event_name":"SessionStart","session_id":"{id}"}}"#);
        assert_eq!(export_session(&file, &event("s-1")), Ok(true));
        // The same session again (a resume, a compaction) adds nothing.
        assert_eq!(export_session(&file, &event("s-1")), Ok(false));
        assert_eq!(export_session(&file, &event("s-2")), Ok(true));
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        assert_eq!(
            text,
            "export OTHER=1\nexport WRITFOLD_SESSION=s-1\nexport WRITFOLD_SESSION=s-2\n"
        );
    }

    /// An ID that is not a session ID never reaches a file a shell runs.
    #[test]
    fn an_unusable_session_is_not_exported() {
        let file = scratch("unusable");
        for bad in [
            r#"{"session_id":"a; rm -rf b"}"#,
            r#"{"session_id":".x"}"#,
            "{}",
            "no",
        ] {
            assert_eq!(export_session(&file, bad), Ok(false), "{bad}");
        }
        assert!(!file.exists());
    }
}
