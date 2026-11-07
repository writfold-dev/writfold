use super::{GitError, GitExecutable};
use std::path::Path;
use std::process::Command;

/// Shared closed-command environment. Never inherits credentials or Git config.
pub(super) fn command(
    executable: &GitExecutable<'_>,
    directory: &Path,
) -> Result<Command, GitError> {
    let mut command = Command::new(executable.path()?);
    command.env_clear().current_dir(directory);
    for (key, value) in [
        ("PATH", ""),
        ("HOME", "/nonexistent"),
        ("XDG_CONFIG_HOME", "/nonexistent"),
        ("LC_ALL", "C"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_CONFIG_SYSTEM", "/dev/null"),
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
        ("GIT_NO_REPLACE_OBJECTS", "1"),
        ("GIT_NO_LAZY_FETCH", "1"),
        ("GIT_TERMINAL_PROMPT", "0"),
        ("GIT_OPTIONAL_LOCKS", "0"),
        ("GIT_ATTR_NOSYSTEM", "1"),
        ("GIT_ALLOW_PROTOCOL", ""),
    ] {
        command.env(key, value);
    }
    command
        .arg("--no-replace-objects")
        .arg("--literal-pathspecs")
        .arg("--git-dir")
        .arg(directory);
    for setting in [
        "core.fsmonitor=false",
        "core.hooksPath=/dev/null",
        "maintenance.auto=false",
        "gc.auto=0",
        "core.pager=cat",
        "diff.external=",
        "protocol.allow=never",
        "core.attributesFile=/dev/null",
        "core.fsync=loose-object,reference",
        "core.fsyncMethod=fsync",
        "core.logAllRefUpdates=false",
    ] {
        command.arg("-c").arg(setting);
    }
    Ok(command)
}
