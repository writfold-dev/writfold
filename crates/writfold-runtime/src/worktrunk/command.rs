use super::{Limits, Result, Worktrunk, WorktrunkError};
use crate::process::{self, ProcessLimits};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use writfold_domain::ByteCount;

pub(super) struct Session<'a, 'tool, 'file> {
    pub adapter: &'a Worktrunk<'tool, 'file>,
    pub limits: Limits,
    pub deadline: Instant,
    pub cancelled: &'a AtomicBool,
}
impl<'a, 'tool, 'file> Session<'a, 'tool, 'file> {
    pub fn new(
        adapter: &'a Worktrunk<'tool, 'file>,
        limits: Limits,
        cancelled: &'a AtomicBool,
    ) -> Result<Self> {
        let deadline = Instant::now()
            .checked_add(limits.timeout)
            .ok_or(WorktrunkError::Profile("deadline overflow"))?;
        let session = Self {
            adapter,
            limits,
            deadline,
            cancelled,
        };
        session.check()?;
        Ok(session)
    }
    pub fn check(&self) -> Result<()> {
        process::check_stop(self.deadline, self.cancelled)?;
        self.adapter.check()
    }
    pub fn git(&self, directory: &Path, args: &[&str]) -> Result<Vec<u8>> {
        self.git_io(directory, args, &[], self.limits.stdout)
    }
    pub fn git_io(
        &self,
        directory: &Path,
        args: &[&str],
        input: &[u8],
        stdout: ByteCount,
    ) -> Result<Vec<u8>> {
        self.check()?;
        let mut command = self.command(self.adapter.git.path()?, directory);
        command
            .args(["--no-replace-objects", "--literal-pathspecs"])
            .args(args);
        Ok(process::run(
            &mut command,
            input,
            ProcessLimits::new(stdout, self.limits.stderr),
            self.deadline,
            self.cancelled,
            self.adapter.git.supervisor(),
        )?
        .stdout)
    }
    pub fn command(&self, executable: impl AsRef<std::ffi::OsStr>, directory: &Path) -> Command {
        let mut command = Command::new(executable);
        command.env_clear().current_dir(directory);
        let controls = &self.adapter.controls;
        command
            .env("PATH", controls)
            .env("GIT_EXEC_PATH", controls)
            .env("HOME", controls.join("home"))
            .env("XDG_CONFIG_HOME", controls.join("home"))
            .env("WORKTRUNK_SYSTEM_CONFIG_PATH", controls.join("empty.toml"))
            .env("WORKTRUNK_PROJECT_CONFIG_PATH", "");
        for (key, value) in [
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
        let settings = [
            ("core.hooksPath", "/dev/null"),
            ("core.fsmonitor", "false"),
            ("core.attributesFile", "/dev/null"),
            ("gc.auto", "0"),
            ("maintenance.auto", "false"),
            ("submodule.recurse", "false"),
            ("protocol.allow", "never"),
            ("core.untrackedCache", "false"),
        ];
        command.env("GIT_CONFIG_COUNT", settings.len().to_string());
        for (index, (key, value)) in settings.into_iter().enumerate() {
            command
                .env(format!("GIT_CONFIG_KEY_{index}"), key)
                .env(format!("GIT_CONFIG_VALUE_{index}"), value);
        }
        command
    }
    pub fn run(&self, command: &mut Command) -> Result<Vec<u8>> {
        Ok(process::run(
            command,
            &[],
            ProcessLimits::new(self.limits.stdout, self.limits.stderr),
            self.deadline,
            self.cancelled,
            self.adapter.git.supervisor(),
        )?
        .stdout)
    }
}

pub(super) fn line(bytes: &[u8]) -> Result<&str> {
    let text = std::str::from_utf8(bytes).map_err(|_| WorktrunkError::Observation("UTF8 line"))?;
    let text = text
        .strip_suffix('\n')
        .ok_or(WorktrunkError::Observation("line terminator"))?;
    if text.is_empty() || text.chars().any(char::is_control) {
        return Err(WorktrunkError::Observation("single line"));
    }
    Ok(text)
}
