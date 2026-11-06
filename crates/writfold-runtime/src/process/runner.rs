use super::{ProcessError, ProcessLimits, ProcessSupervisor};
use rustix::fd::AsFd;
use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
use rustix::process::{Pid, Signal, WaitId, WaitIdOptions, kill_process_group, waitid};
use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub(crate) struct Output {
    pub stdout: Vec<u8>,
}

struct OwnedChild<'a> {
    child: Child,
    group: Pid,
    stdin: Option<ChildStdin>,
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
    reaped: bool,
    supervisor: &'a ProcessSupervisor,
}

impl<'a> OwnedChild<'a> {
    fn spawn(
        command: &mut Command,
        supervisor: &'a ProcessSupervisor,
    ) -> Result<Self, ProcessError> {
        supervisor.ensure_current()?;
        command
            .process_group(0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn()?;
        let group = Pid::from_child(&child);
        let owned = Self {
            stdin: child.stdin.take(),
            stdout: child.stdout.take(),
            stderr: child.stderr.take(),
            child,
            group,
            reaped: false,
            supervisor,
        };
        if let Some(pipe) = &owned.stdin {
            nonblocking(pipe)?;
        }
        if let Some(pipe) = &owned.stdout {
            nonblocking(pipe)?;
        }
        if let Some(pipe) = &owned.stderr {
            nonblocking(pipe)?;
        }
        Ok(owned)
    }

    fn kill_group(&self) -> Result<(), ProcessError> {
        match kill_process_group(self.group, Signal::KILL) {
            Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
            Err(error) => Err(ProcessError::Cleanup(error.into())),
        }
    }

    fn exited(&self) -> Result<bool, ProcessError> {
        // NOWAIT keeps the group leader's PID reserved until descendant reaping.
        loop {
            match waitid(
                WaitId::Pid(self.group),
                WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
            ) {
                Ok(status) => return Ok(status.is_some()),
                Err(rustix::io::Errno::INTR) => continue,
                Err(error) => return Err(error.into()),
            }
        }
    }

    fn await_exit(&self) -> Result<(), ProcessError> {
        loop {
            match waitid(
                WaitId::Pid(self.group),
                WaitIdOptions::EXITED | WaitIdOptions::NOWAIT,
            ) {
                Ok(Some(_)) => return Ok(()),
                Ok(None) => {
                    return Err(ProcessError::Cleanup(std::io::Error::other(
                        "blocking wait returned no leader",
                    )));
                }
                Err(rustix::io::Errno::INTR) => continue,
                Err(error) => return Err(ProcessError::Cleanup(error.into())),
            }
        }
    }

    fn finish(&mut self) -> Result<std::process::ExitStatus, ProcessError> {
        self.kill_group()?;
        self.stdin.take();
        // EOF only closes pipe ownership. Silent descendants must also be reaped.
        let drained = drain(&mut self.stdout, &mut self.stderr);
        self.await_exit()?;
        self.supervisor.reap_descendants(self.group)?;
        // Reap the reserved leader last; never signal or enumerate this group
        // after its identity can be reused by another process.
        let waited = self.child.wait().map_err(ProcessError::Cleanup);
        self.reaped = waited.is_ok();
        drained?;
        waited
    }
}

impl Drop for OwnedChild<'_> {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.finish();
        }
    }
}

pub(crate) fn run(
    command: &mut Command,
    input: &[u8],
    budgets: ProcessLimits,
    deadline: Instant,
    cancelled: &AtomicBool,
    supervisor: &ProcessSupervisor,
) -> Result<Output, ProcessError> {
    check_stop(deadline, cancelled)?;
    let mut child = OwnedChild::spawn(command, supervisor)?;
    let result = communicate(&mut child, input, budgets, deadline, cancelled);
    let status = child.finish()?;
    let (stdout, stderr) = result?;
    if !status.success() {
        return Err(ProcessError::Command {
            status: status.code(),
            stderr,
        });
    }
    Ok(Output { stdout })
}

fn communicate(
    child: &mut OwnedChild<'_>,
    input: &[u8],
    budgets: ProcessLimits,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<(Vec<u8>, Vec<u8>), ProcessError> {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut sent = 0;
    let mut exited = false;
    loop {
        check_stop(deadline, cancelled)?;
        if sent == input.len() {
            child.stdin.take();
        }
        if let Some(pipe) = &mut child.stdin {
            match pipe.write(&input[sent..]) {
                Ok(0) => return Err(ProcessError::ClosedStdin),
                Ok(count) => sent += count,
                Err(error) if retryable(&error) => {}
                Err(error) => return Err(error.into()),
            }
        }
        read_bounded(
            &mut child.stdout,
            &mut stdout,
            budgets.stdout.get(),
            "stdout",
        )?;
        read_bounded(
            &mut child.stderr,
            &mut stderr,
            budgets.stderr.get(),
            "stderr",
        )?;
        if !exited && child.exited()? {
            child.kill_group()?;
            exited = true;
        }
        if exited && child.stdout.is_none() && child.stderr.is_none() {
            return Ok((stdout, stderr));
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn nonblocking(pipe: &impl AsFd) -> Result<(), ProcessError> {
    fcntl_setfl(pipe, fcntl_getfl(pipe)? | OFlags::NONBLOCK)?;
    Ok(())
}

fn read_bounded<R: Read>(
    pipe: &mut Option<R>,
    bytes: &mut Vec<u8>,
    limit: u64,
    stream: &'static str,
) -> Result<(), ProcessError> {
    let Some(reader) = pipe else {
        return Ok(());
    };
    let mut buffer = [0_u8; 8192];
    let remaining = limit.saturating_sub(bytes.len() as u64);
    let count = remaining.saturating_add(1).min(buffer.len() as u64) as usize;
    match reader.read(&mut buffer[..count]) {
        Ok(0) => {
            pipe.take();
        }
        Ok(count) => {
            if count as u64 > remaining {
                return Err(ProcessError::Budget { stream, limit });
            }
            let required = bytes.len().checked_add(count).ok_or(ProcessError::Budget {
                stream: "allocation",
                limit,
            })?;
            if required > bytes.capacity() {
                let ceiling = usize::try_from(limit).unwrap_or(usize::MAX);
                let capacity = required
                    .max(bytes.capacity().saturating_mul(2))
                    .min(ceiling);
                bytes
                    .try_reserve_exact(capacity - bytes.len())
                    .map_err(|_| ProcessError::Budget {
                        stream: "allocation",
                        limit,
                    })?;
            }
            bytes.extend_from_slice(&buffer[..count]);
        }
        Err(error) if retryable(&error) => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn drain<A: Read, B: Read>(a: &mut Option<A>, b: &mut Option<B>) -> Result<(), ProcessError> {
    let mut failure = None;
    while a.is_some() || b.is_some() {
        if let Err(error) = discard(a) {
            failure = Some(error);
        }
        if let Err(error) = discard(b) {
            failure = Some(error);
        }
        if a.is_some() || b.is_some() {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    failure.map_or(Ok(()), |error| Err(ProcessError::Cleanup(error)))
}

fn discard<R: Read>(pipe: &mut Option<R>) -> std::io::Result<()> {
    let Some(reader) = pipe else {
        return Ok(());
    };
    let mut buffer = [0_u8; 8192];
    match reader.read(&mut buffer) {
        Ok(0) => {
            pipe.take();
        }
        Ok(_) => {}
        Err(error) if retryable(&error) => {}
        Err(error) => {
            pipe.take();
            return Err(error);
        }
    }
    Ok(())
}

fn retryable(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
    )
}

pub(crate) fn check_stop(deadline: Instant, cancelled: &AtomicBool) -> Result<(), ProcessError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(ProcessError::Cancelled);
    }
    if Instant::now() >= deadline {
        return Err(ProcessError::Deadline);
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
