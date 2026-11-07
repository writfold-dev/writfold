use super::{OwnedChild, check_stop, read_bounded};
use crate::process::{ProcessError, ProcessLimits, ProcessSupervisor};
use rustix::process::Signal;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

/// A live process retains its reserved leader and group ownership until cleanup.
pub(crate) struct Service<'a> {
    child: OwnedChild<'a>,
    limits: ProcessLimits,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    budget_failure: Option<(&'static str, u64)>,
}

pub(crate) struct ServiceOutput {
    pub(crate) status: std::process::ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
    pub(crate) forced_kill: bool,
}

impl<'a> Service<'a> {
    pub(crate) fn spawn(
        command: &mut Command,
        limits: ProcessLimits,
        supervisor: &'a ProcessSupervisor,
        deadline: Instant,
        cancelled: &AtomicBool,
    ) -> Result<Self, ProcessError> {
        check_stop(deadline, cancelled)?;
        let mut child = OwnedChild::spawn(command, supervisor)?;
        child.stdin.take();
        check_stop(deadline, cancelled)?;
        Ok(Self {
            child,
            limits,
            stdout: Vec::new(),
            stderr: Vec::new(),
            budget_failure: None,
        })
    }

    pub(crate) fn pid(&self) -> u32 {
        self.child.child.id()
    }

    pub(crate) fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    /// One bounded nonblocking read per stream keeps pumping fair to callers.
    /// Observing exit neither reaps the leader nor relinquishes descendants.
    pub(crate) fn pump(&mut self) -> Result<bool, ProcessError> {
        if self.child.reaped {
            return Ok(false);
        }
        self.read_output()?;
        Ok(!self.child.exited()?)
    }

    /// Stop further native work without relinquishing the reserved leader or
    /// reaping/lease obligations. Repeated containment never signals a reaped PID.
    pub(crate) fn contain(&mut self) -> Result<(), ProcessError> {
        if self.child.reaped {
            Ok(())
        } else {
            self.child.kill_group()
        }
    }

    fn read_output(&mut self) -> Result<(), ProcessError> {
        if let Some((stream, limit)) = self.budget_failure {
            return Err(ProcessError::Budget { stream, limit });
        }
        let result = read_bounded(
            &mut self.child.stdout,
            &mut self.stdout,
            self.limits.stdout.get(),
            "stdout",
        )
        .and_then(|()| {
            read_bounded(
                &mut self.child.stderr,
                &mut self.stderr,
                self.limits.stderr.get(),
                "stderr",
            )
        });
        if let Err(ProcessError::Budget { stream, limit }) = &result {
            self.budget_failure = Some((*stream, *limit));
        }
        result
    }

    pub(crate) fn finish(
        &mut self,
        grace: Duration,
        deadline: Instant,
    ) -> Result<ServiceOutput, ProcessError> {
        if self.child.reaped {
            return Err(ProcessError::UnsupportedProfile("service already finished"));
        }
        self.child.signal_group(Signal::TERM)?;
        let now = Instant::now();
        let until = now.checked_add(grace).unwrap_or(deadline).min(deadline);
        let mut forced_kill = false;
        loop {
            self.read_output()?;
            if self.child.exited()?
                && self
                    .child
                    .supervisor
                    .reap_exited_descendants(self.child.group)?
            {
                break;
            }
            if Instant::now() >= until {
                self.child.kill_group()?;
                forced_kill = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        // After escalation, drain with the same lifetime budgets before reaping.
        // A failure retains this Service. Drop cannot relinquish ownership until
        // the owned child/group cleanup has actually completed.
        while self.child.stdout.is_some() || self.child.stderr.is_some() {
            self.read_output()?;
            if self.child.stdout.is_some() || self.child.stderr.is_some() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        let status = self.child.finish()?;
        Ok(ServiceOutput {
            status,
            stdout: std::mem::take(&mut self.stdout),
            stderr: std::mem::take(&mut self.stderr),
            forced_kill,
        })
    }
}
