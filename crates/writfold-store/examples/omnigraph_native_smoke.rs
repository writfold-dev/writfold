//! Real pinned-native and wire consumer for the omnigraph-runtime fixture.
//! Arguments: NATIVE_DIR SERVER_SHA256 CLI_SHA256 RUNTIME_FIXTURE NEW_WORK LOCKS_DIR
//! Optional final PREPARED_ROOT skips bootstrap; it must contain the empty
//! fixture graph `faults`, with no other native owner running.
//! LOCKS_DIR is the existing shared managed coordination namespace, not a fresh
//! per-process lock directory. All exact native response bytes remain in NEW_WORK.
//! This is transport/binding smoke, not public writer enablement or qualification.
#[path = "support/native_flow.rs"]
mod flow;
#[path = "support/native_setup.rs"]
mod setup;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
use writfold_domain::{ByteCount, ContentDigest, GraphId};
use writfold_runtime::fs::{ContainedRoot, LockKey, ManagedLocks, RelativePath};
use writfold_runtime::omnigraph::{NativeCall, NativeExecutable, NativeLimits, NativeServer};
use writfold_runtime::process::ProcessSupervisor;

type Result<T = (), E = Box<dyn Error>> = std::result::Result<T, E>;
fn count(value: u64) -> Result<ByteCount> {
    Ok(ByteCount::new(value)?)
}

fn main() -> Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 6 && args.len() != 7 {
        return Err("usage: omnigraph_native_smoke NATIVE_DIR SERVER_SHA256 CLI_SHA256 RUNTIME_FIXTURE NEW_WORK LOCKS_DIR [PREPARED_ROOT]".into());
    }
    let native = ContainedRoot::open(&args[0])?;
    let server_file = native.pin(&RelativePath::new("omnigraph-server")?)?;
    let cli_file = native.pin(&RelativePath::new("omnigraph")?)?;
    let server_digest: ContentDigest = args[1].parse()?;
    let cli_digest: ContentDigest = args[2].parse()?;
    let supervisor = ProcessSupervisor::enable()?;
    let server_executable =
        NativeExecutable::admit(&server_file, &server_digest, count(1 << 30)?, &supervisor)?;
    let cli = NativeExecutable::admit(&cli_file, &cli_digest, count(1 << 30)?, &supervisor)?;
    let work_path = PathBuf::from(&args[4]);
    let work = ContainedRoot::create(&work_path)?;
    let namespace = ManagedLocks::open(&args[5])?;
    let coordination_target = work.pin(&RelativePath::new("coordination-target")?)?;
    let lease = coordination_target.try_lock(&namespace, &LockKey::new("omnigraph-faults")?)?;
    let graph: GraphId = "faults".parse()?;
    let cancelled = AtomicBool::new(false);
    let limits = native_limits()?;
    let source = args
        .get(6)
        .map_or(setup::Source::Fresh(Path::new(&args[3])), |path| {
            setup::Source::Prepared(Path::new(path))
        });
    let root = setup::prepare(
        &work_path,
        source,
        &graph,
        &cli,
        &lease,
        limits,
        NativeCall {
            deadline: Instant::now() + Duration::from_secs(120),
            cancelled: &cancelled,
        },
    )?;
    let mut server = NativeServer::start(
        &server_executable,
        root,
        lease,
        &work_path.join("credentials"),
        setup::credentials()?,
        limits,
        NativeCall {
            deadline: Instant::now() + Duration::from_secs(45),
            cancelled: &cancelled,
        },
    )?;
    // Only read-only readiness may repeat. Retain ownership and shut down even
    // when an original mutation is uncertain or a semantic check fails.
    let exercised = flow::exercise(&mut server, &work_path, &graph, &cancelled);
    let shutdown = server.shutdown(Instant::now() + Duration::from_secs(20))?;
    fs::write(work_path.join("server.stdout"), shutdown.stdout)?;
    fs::write(work_path.join("server.stderr"), shutdown.stderr)?;
    if !shutdown.status.success() || shutdown.forced_kill {
        return Err("native server did not shut down cleanly".into());
    }
    exercised?;
    println!(
        "observed conditional insert/change, exact snapshots, stale-head refusal, retained history and clean shutdown; bytes: {}",
        work_path.display()
    );
    Ok(())
}

fn native_limits() -> Result<NativeLimits> {
    Ok(NativeLimits {
        request_header: count(16 << 10)?,
        request_body: count(1 << 20)?,
        response_header: count(16 << 10)?,
        response_body: count(4 << 20)?,
        stdout: count(4 << 20)?,
        stderr: count(4 << 20)?,
        shutdown_grace: Duration::from_secs(10),
    })
}
