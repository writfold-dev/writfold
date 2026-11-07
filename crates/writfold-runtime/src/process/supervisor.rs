use super::ProcessError;
use rustix::process::{
    Pid, WaitOptions, child_subreaper, getpgid, getpid, set_child_subreaper, waitpid,
};
use std::fs::{self, File};
use std::io::{self, Read};

/// Explicit, process-wide Linux child-subreaper admission.
///
/// This permanent opt-in belongs to the coordinator's startup, not a read side
/// effect. The host must keep procfs available, preserve normal SIGCHLD and
/// subreaper behavior, and never independently reap a managed process group.
/// Groups must not escape through setsid/setpgid. This is not a hostile-process
/// sandbox and does not reap unrelated groups. Cleanup retains the leader until
/// all adopted descendants have been waited, preventing group-ID reuse races.
#[derive(Debug)]
pub struct ProcessSupervisor {
    process: u32,
}

impl ProcessSupervisor {
    pub fn enable() -> Result<Self, ProcessError> {
        // The pinned rustix API represents PR_SET_CHILD_SUBREAPER's nonzero flag
        // as Some(Pid); it does not select another process as the subreaper.
        set_child_subreaper(Some(getpid()))?;
        let supervisor = Self {
            process: std::process::id(),
        };
        supervisor.ensure_current()?;
        Ok(supervisor)
    }

    pub(crate) fn ensure_current(&self) -> Result<(), ProcessError> {
        if self.process != std::process::id() || child_subreaper()?.is_none() {
            return Err(ProcessError::UnsupportedProfile(
                "process supervisor is not current",
            ));
        }
        Ok(())
    }

    /// Poll adopted descendants only after the reserved leader has exited.
    /// A live descendant is not quiescence, even if it closed every pipe.
    pub(super) fn reap_exited_descendants(&self, leader: Pid) -> Result<bool, ProcessError> {
        self.ensure_current()?;
        while let Some(child) = group_descendant(leader).map_err(ProcessError::Cleanup)? {
            loop {
                match waitpid(Some(child), WaitOptions::NOHANG) {
                    Ok(Some((_, status))) if status.exited() || status.signaled() => break,
                    Ok(Some(_)) => continue,
                    Ok(None) => return Ok(false),
                    Err(rustix::io::Errno::INTR) => continue,
                    Err(error) => return Err(ProcessError::Cleanup(error.into())),
                }
            }
        }
        Ok(true)
    }

    /// The killed leader must already be observed exited with NOWAIT. Its PID
    /// remains reserved throughout this method; the caller waits it last.
    pub(super) fn reap_descendants(&self, leader: Pid) -> Result<(), ProcessError> {
        self.ensure_current()?;
        while let Some(child) = group_descendant(leader).map_err(ProcessError::Cleanup)? {
            loop {
                match waitpid(Some(child), WaitOptions::empty()) {
                    Ok(Some((_, status))) if status.exited() || status.signaled() => break,
                    Ok(Some(_)) => continue,
                    Ok(None) => {
                        return Err(ProcessError::Cleanup(io::Error::other(
                            "blocking wait returned no child",
                        )));
                    }
                    Err(rustix::io::Errno::INTR) => continue,
                    Err(error) => return Err(ProcessError::Cleanup(error.into())),
                }
            }
        }
        Ok(())
    }
}

fn group_descendant(leader: Pid) -> io::Result<Option<Pid>> {
    for task in fs::read_dir("/proc/self/task")? {
        let path = task?.path().join("children");
        let file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        if let Some(child) = child_in_listing(file, leader)? {
            return Ok(Some(child));
        }
    }
    Ok(None)
}

fn child_in_listing(mut reader: impl Read, leader: Pid) -> io::Result<Option<Pid>> {
    let mut buffer = [0_u8; 4096];
    let mut number = 0_u32;
    let mut digits = false;
    loop {
        let count = match reader.read(&mut buffer) {
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if count == 0 {
            return if digits {
                in_group(number, leader)
            } else {
                Ok(None)
            };
        }
        for &byte in &buffer[..count] {
            if byte.is_ascii_digit() {
                number = number
                    .checked_mul(10)
                    .and_then(|n| n.checked_add(u32::from(byte - b'0')))
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "invalid child PID")
                    })?;
                digits = true;
            } else if byte.is_ascii_whitespace() {
                if digits && let Some(child) = in_group(number, leader)? {
                    return Ok(Some(child));
                }
                number = 0;
                digits = false;
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid procfs child listing",
                ));
            }
        }
    }
}

fn in_group(number: u32, leader: Pid) -> io::Result<Option<Pid>> {
    let child = i32::try_from(number)
        .ok()
        .and_then(Pid::from_raw)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid child PID"))?;
    if child == leader {
        return Ok(None);
    }
    loop {
        match getpgid(Some(child)) {
            Ok(group) => return Ok((group == leader).then_some(child)),
            // Unrelated children may exit while their owning threads reap them.
            Err(rustix::io::Errno::SRCH) => return Ok(None),
            Err(rustix::io::Errno::INTR) => continue,
            Err(error) => return Err(error.into()),
        }
    }
}
