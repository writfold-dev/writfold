//! One supervised command whose output is kept as bounded tails: the gate
//! runner of `writfold kb gate`. Unlike [`super::run`], a non-zero exit, a
//! signal or a timeout is a result, not an error, and output beyond the
//! limit drops its oldest bytes instead of failing. The command runs in its
//! own process group with stdin at end of file; when the leader exits, or
//! the deadline passes (TERM, a grace period, then KILL), the whole group
//! is killed and reaped, so nothing the command started outlives it.
use super::{OwnedChild, retryable};
use crate::process::{ProcessError, ProcessSupervisor};
use rustix::process::Signal;
use std::io::Read;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;
use std::time::{Duration, Instant};

/// How long pipes are still read after the group was killed; a descendant
/// that escaped the group may hold them open.
const DRAIN: Duration = Duration::from_secs(2);

/// The last bytes of one stream and how many it wrote in all.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Tail {
    pub bytes: Vec<u8>,
    pub total: u64,
}

/// How the command ended.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TailedEnd {
    Exited(i32),
    Signaled(i32),
    /// Stopped at the deadline (TERM, then KILL after the grace period).
    TimedOut,
}

/// What [`run_tailed`] observed.
#[derive(Clone, Debug)]
pub struct Tailed {
    pub end: TailedEnd,
    pub stdout: Tail,
    pub stderr: Tail,
    pub elapsed: Duration,
}

/// Limits for [`run_tailed`].
#[derive(Clone, Copy, Debug)]
pub struct TailLimits {
    /// Bytes kept from the end of each stream.
    pub tail: usize,
    pub timeout: Duration,
    /// Between TERM and KILL at the deadline.
    pub grace: Duration,
}

/// Run `command` to its end. An error means it could not be started or
/// supervised; the caller must treat that as an unknown outcome.
pub fn run_tailed(
    command: &mut Command,
    limits: TailLimits,
    supervisor: &ProcessSupervisor,
) -> Result<Tailed, ProcessError> {
    let started = Instant::now();
    let deadline = started
        .checked_add(limits.timeout)
        .ok_or(ProcessError::UnsupportedProfile("deadline overflow"))?;
    let mut child = OwnedChild::spawn(command, supervisor)?;
    child.stdin.take();
    let mut out = (Tail::default(), Tail::default());
    let mut timed_out = false;
    let mut termed: Option<Instant> = None;
    loop {
        let progressed = pump(&mut child, &mut out, limits.tail)?;
        if child.exited()? {
            break;
        }
        let now = Instant::now();
        match termed {
            None if now >= deadline => {
                timed_out = true;
                child.signal_group(Signal::TERM)?;
                termed = Some(now);
            }
            Some(at) if now.duration_since(at) >= limits.grace => {
                child.kill_group()?;
            }
            _ => {}
        }
        if !progressed {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    child.kill_group()?;
    let until = Instant::now() + DRAIN;
    while (child.stdout.is_some() || child.stderr.is_some()) && Instant::now() < until {
        if !pump(&mut child, &mut out, limits.tail)? {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    child.stdout.take();
    child.stderr.take();
    let status = child.finish()?;
    let end = match (timed_out, status.code(), status.signal()) {
        (true, _, _) => TailedEnd::TimedOut,
        (false, Some(code), _) => TailedEnd::Exited(code),
        (false, None, Some(signal)) => TailedEnd::Signaled(signal),
        (false, None, None) => return Err(ProcessError::UnsupportedProfile("exit status")),
    };
    Ok(Tailed {
        end,
        stdout: out.0,
        stderr: out.1,
        elapsed: started.elapsed(),
    })
}

/// Read what both pipes hold now. Returns whether anything moved.
fn pump(
    child: &mut OwnedChild<'_>,
    out: &mut (Tail, Tail),
    limit: usize,
) -> Result<bool, ProcessError> {
    let a = read_tail(&mut child.stdout, &mut out.0, limit)?;
    let b = read_tail(&mut child.stderr, &mut out.1, limit)?;
    Ok(a || b)
}

fn read_tail<R: Read>(
    pipe: &mut Option<R>,
    tail: &mut Tail,
    limit: usize,
) -> Result<bool, ProcessError> {
    let mut progressed = false;
    let mut buffer = [0_u8; 65536];
    while let Some(reader) = pipe {
        match reader.read(&mut buffer) {
            Ok(0) => {
                pipe.take();
                progressed = true;
            }
            Ok(count) => {
                progressed = true;
                tail.total = tail.total.saturating_add(count as u64);
                tail.bytes.extend_from_slice(&buffer[..count]);
                if tail.bytes.len() > limit {
                    let excess = tail.bytes.len() - limit;
                    tail.bytes.drain(..excess);
                }
            }
            Err(error) if retryable(&error) => break,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(progressed)
}
