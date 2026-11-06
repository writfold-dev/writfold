use super::*;
use std::error::Error;
use std::fs;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use writfold_domain::ByteCount;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Rendezvous {
    path: PathBuf,
    listener: UnixListener,
}

impl Rendezvous {
    fn new() -> TestResult<Self> {
        let path = std::env::temp_dir().join(format!(
            "wfg-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let listener = UnixListener::bind(&path)?;
        listener.set_nonblocking(true)?;
        Ok(Self { path, listener })
    }

    fn child(&self, role: &str) -> TestResult<Command> {
        let mut command = Command::new(std::env::current_exe()?);
        command
            .args([
                "--exact",
                "process::runner::tests::child_entry",
                "--nocapture",
            ])
            .env("WRITFOLD_PROCESS_TEST_SOCKET", &self.path)
            .env("WRITFOLD_PROCESS_TEST_ROLE", role);
        Ok(command)
    }

    fn accept(&self) -> TestResult<(UnixStream, u32)> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
                    let mut pid = [0_u8; 4];
                    stream.read_exact(&mut pid)?;
                    return Ok((stream, u32::from_be_bytes(pid)));
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err("child rendezvous deadline".into());
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(error) => return Err(error.into()),
            }
        }
    }
}

impl Drop for Rendezvous {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

// These are actual test-executable children, not command-output mocks. The
// socket establishes readiness and controls exit independently of timing.
#[test]
fn child_entry() -> TestResult {
    let Some(socket) = std::env::var_os("WRITFOLD_PROCESS_TEST_SOCKET") else {
        return Ok(());
    };
    let role = std::env::var("WRITFOLD_PROCESS_TEST_ROLE")?;
    let mut stream = UnixStream::connect(&socket)?;
    stream.write_all(&std::process::id().to_be_bytes())?;
    if role == "leader" || role == "leader-silent" {
        let mut descendant = Command::new(std::env::current_exe()?);
        descendant
            .args([
                "--exact",
                "process::runner::tests::child_entry",
                "--nocapture",
            ])
            .env("WRITFOLD_PROCESS_TEST_ROLE", "leaf");
        if role == "leader-silent" {
            descendant
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
        }
        let _descendant = descendant.spawn()?;
        let mut release = [0_u8; 1];
        stream.read_exact(&mut release)?;
        std::process::exit(0);
    }
    let mut release = [0_u8; 1];
    stream.read_exact(&mut release)?;
    match role.as_str() {
        "stdout" => std::io::stdout().write_all(&[b'x'; 32768])?,
        "stderr" => std::io::stderr().write_all(&[b'x'; 32768])?,
        "failure" => std::process::exit(7),
        _ => {}
    }
    Ok(())
}

fn limits() -> TestResult<ProcessLimits> {
    Ok(ProcessLimits::new(
        ByteCount::new(4096)?,
        ByteCount::new(4096)?,
    ))
}

fn reaped(pid: u32) -> TestResult {
    match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
        Ok(stat) => Err(format!("owned child was not reaped: {stat}").into()),
    }
}

#[test]
fn owned_children_deadline_cancellation_overflow_and_error_cleanup() -> TestResult {
    let supervisor = ProcessSupervisor::enable()?;
    for role in ["deadline", "cancel", "stdout", "stderr", "failure"] {
        let rendezvous = Rendezvous::new()?;
        let mut child = OwnedChild::spawn(&mut rendezvous.child(role)?, &supervisor)?;
        let (mut stream, pid) = rendezvous.accept()?;
        stream.write_all(&[1])?;
        let cancelled = AtomicBool::new(role == "cancel");
        let deadline = if role == "deadline" {
            Instant::now()
        } else {
            Instant::now() + Duration::from_secs(10)
        };
        let result = communicate(&mut child, &[], limits()?, deadline, &cancelled);
        let status = child.finish()?;
        match role {
            "deadline" => assert!(matches!(result, Err(ProcessError::Deadline))),
            "cancel" => assert!(matches!(result, Err(ProcessError::Cancelled))),
            "stdout" => assert!(matches!(
                result,
                Err(ProcessError::Budget {
                    stream: "stdout",
                    ..
                })
            )),
            "stderr" => assert!(matches!(
                result,
                Err(ProcessError::Budget {
                    stream: "stderr",
                    ..
                })
            )),
            _ => {
                assert!(result.is_ok());
                assert_eq!(status.code(), Some(7));
            }
        }
        reaped(pid)?;
    }
    Ok(())
}

#[test]
fn leader_exit_reaps_pipe_holding_and_silent_descendants() -> TestResult {
    let supervisor = ProcessSupervisor::enable()?;
    for role in ["leader", "leader-silent"] {
        let rendezvous = Rendezvous::new()?;
        let mut child = OwnedChild::spawn(&mut rendezvous.child(role)?, &supervisor)?;
        let (mut leader, leader_pid) = rendezvous.accept()?;
        let (_leaf, leaf_pid) = rendezvous.accept()?;
        leader.write_all(&[1])?;
        let result = communicate(
            &mut child,
            &[],
            limits()?,
            Instant::now() + Duration::from_secs(10),
            &AtomicBool::new(false),
        );
        child.finish()?;
        assert!(result.is_ok());
        reaped(leader_pid)?;
        reaped(leaf_pid)?;
    }
    Ok(())
}

#[test]
fn dropping_owned_child_kills_and_reaps_without_completion() -> TestResult {
    let supervisor = ProcessSupervisor::enable()?;
    let rendezvous = Rendezvous::new()?;
    let child = OwnedChild::spawn(&mut rendezvous.child("leaf")?, &supervisor)?;
    let (_stream, pid) = rendezvous.accept()?;
    drop(child);
    reaped(pid)?;
    Ok(())
}

#[test]
fn owned_group_reaping_does_not_consume_an_unrelated_child() -> TestResult {
    let supervisor = ProcessSupervisor::enable()?;
    let unrelated_ready = Rendezvous::new()?;
    let mut unrelated = unrelated_ready
        .child("leaf")?
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let (mut release_unrelated, _) = unrelated_ready.accept()?;
    let owned_ready = Rendezvous::new()?;
    let mut owned = OwnedChild::spawn(&mut owned_ready.child("leaf")?, &supervisor)?;
    let (_owned_stream, owned_pid) = owned_ready.accept()?;
    owned.finish()?;
    reaped(owned_pid)?;
    assert!(unrelated.try_wait()?.is_none());
    release_unrelated.write_all(&[1])?;
    assert!(unrelated.wait()?.success());
    Ok(())
}
