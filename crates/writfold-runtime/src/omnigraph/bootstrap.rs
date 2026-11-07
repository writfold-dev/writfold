use super::admission::{PrivateFile, descriptor_path};
use super::{
    NativeCall, NativeError, NativeExecutable, NativeLimits, NativeRoot, ShutdownObservation,
};
use crate::fs::{ContainedRoot, LockedFile, RelativePath};
use crate::process::{ProcessLimits, Service};
use rustix::fs::{Mode, fsync, mkdirat};
use std::path::Path;
use std::time::Duration;
use writfold_domain::{ByteCount, GraphId};

/// One local graph's explicit bootstrap assets. Runtime generates the cluster
/// configuration itself: no caller-supplied roots, remote stores or load sources.
/// These are initialization inputs only, never a server startup source bundle.
pub struct BootstrapGraph<'a> {
    pub graph: &'a GraphId,
    pub schema: &'a [u8],
    pub queries: &'a [u8],
    pub policy: &'a [u8],
}

/// Actual stopped CLI observations, including exact returned bytes. Constructed
/// only after clean owned-group quiescence and exact fresh-root lock handoff.
#[derive(Debug)]
pub struct BootstrapObservation {
    pub apply: ShutdownObservation,
    pub unlock: ShutdownObservation,
}

/// Completed CLI bytes remain available even when bootstrap refuses. An apply
/// failure cannot unlock; an unlock failure also retains the preceding apply.
#[derive(Debug, thiserror::Error)]
#[error("native bootstrap {phase}: {reason}")]
pub struct BootstrapFailure {
    pub phase: &'static str,
    #[source]
    pub reason: NativeError,
    pub apply: ShutdownObservation,
    pub unlock: Option<ShutdownObservation>,
}

fn failed(
    phase: &'static str,
    reason: NativeError,
    apply: ShutdownObservation,
    unlock: Option<ShutdownObservation>,
) -> NativeError {
    NativeError::Bootstrap(Box::new(BootstrapFailure {
        phase,
        reason,
        apply,
        unlock,
    }))
}

#[derive(Clone, Copy)]
struct BootstrapAssets<'a> {
    bundle: &'a ContainedRoot,
    files: &'a [PrivateFile],
}

impl NativeRoot {
    /// Explicit one-time initialization. The target leaf MUST be absent and is
    /// created privately here. A failure retains that root for diagnosis and
    /// cannot be retried through this API. There is deliberately no public
    /// stale-lock or force-unlock function.
    ///
    /// Only the exact lock announced by this completed, bounded, reaped local
    /// CLI invocation may be released, while the caller's realm lease is held.
    /// Process cleanup alone does not authorize release after a failed apply.
    pub fn bootstrap_fresh(
        path: &Path,
        lease: &LockedFile<'_>,
        cli: &NativeExecutable<'_>,
        graph: BootstrapGraph<'_>,
        asset_limit: ByteCount,
        limits: NativeLimits,
        call: NativeCall<'_>,
    ) -> Result<(Self, BootstrapObservation), NativeError> {
        super::http::stop(call.deadline, call.cancelled)?;
        lease.check_current()?;
        let (root, bundle, files) = Self::bootstrap_bundle(path, graph, asset_limit)?;
        let assets = BootstrapAssets {
            bundle: &bundle,
            files: &files,
        };
        let mut command = cli.command(&root)?;
        command
            .args([
                "--target",
                root.canonical_uri(),
                "cluster",
                "apply",
                "--config",
            ])
            .arg(descriptor_path(bundle.descriptor()))
            .arg("--json");
        let apply = complete_cli(&mut command, cli, lease, &root, assets, limits, call)?;
        if let Err(error) = validate_apply(&apply) {
            return Err(failed("apply", error, apply, None));
        }
        let lock = match exact_lock(&apply.stderr, root.canonical_uri()) {
            Ok(lock) => lock,
            Err(error) => return Err(failed("apply", error, apply, None)),
        };
        // No lock ID can enter here from a caller, filesystem stale lock, HTTP
        // error or a killed/failed/unreaped CLI. This is the only unlock call.
        let mut command = match cli.command(&root) {
            Ok(command) => command,
            Err(error) => return Err(failed("unlock", error, apply, None)),
        };
        command.args([
            "--target",
            root.canonical_uri(),
            "cluster",
            "force-unlock",
            lock,
            "--json",
        ]);
        let unlock = match complete_cli(&mut command, cli, lease, &root, assets, limits, call) {
            Ok(unlock) => unlock,
            Err(error) => return Err(failed("unlock", error, apply, None)),
        };
        if let Err(error) = validate_unlock(&unlock, lock) {
            return Err(failed("unlock", error, apply, Some(unlock)));
        }
        Ok((root, BootstrapObservation { apply, unlock }))
    }

    fn bootstrap_bundle(
        path: &Path,
        graph: BootstrapGraph<'_>,
        asset_limit: ByteCount,
    ) -> Result<(Self, ContainedRoot, [PrivateFile; 4]), NativeError> {
        let asset_bytes = graph
            .schema
            .len()
            .checked_add(graph.queries.len())
            .and_then(|size| size.checked_add(graph.policy.len()))
            .ok_or(NativeError::Budget("bootstrap assets"))?;
        if asset_bytes as u64 > asset_limit.get() {
            return Err(NativeError::Budget("bootstrap assets"));
        }
        let graph_id = graph.graph.as_str();
        if !graph_id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
        {
            return Err(NativeError::Input(
                "bootstrap graph must be a simple local identifier",
            ));
        }
        let root = Self::bind(ContainedRoot::create(path)?)?;
        mkdirat(root.root.descriptor(), ".writfold-bootstrap", Mode::RWXU)
            .map_err(crate::fs::FsError::from)?;
        fsync(root.root.descriptor()).map_err(crate::fs::FsError::from)?;
        let bundle = root.root.open_child(
            &RelativePath::new(".writfold-bootstrap").map_err(crate::fs::FsError::from)?,
        )?;
        // JSON quoting is YAML-compatible and cannot inject a second YAML key.
        let graph_key = serde_json::to_string(graph_id)?;
        let config = format!(
            "version: 1\nmetadata:\n  name: writfold-native-bootstrap\ngraphs:\n  {graph_key}:\n    schema: schema.pg\n    queries: [queries.gq]\npolicies:\n  native-access:\n    file: graph.policy.yaml\n    applies_to: [{graph_key}]\n"
        );
        let files = [
            PrivateFile::create(&bundle, "schema.pg", graph.schema)?,
            PrivateFile::create(&bundle, "queries.gq", graph.queries)?,
            PrivateFile::create(&bundle, "graph.policy.yaml", graph.policy)?,
            PrivateFile::create(&bundle, "cluster.yaml", config.as_bytes())?,
        ];
        Ok((root, bundle, files))
    }
}

fn completed_json(observation: &ShutdownObservation) -> Result<serde_json::Value, NativeError> {
    if !observation.status.success() || observation.forced_kill {
        return Err(NativeError::Protocol("CLI did not complete successfully"));
    }
    Ok(serde_json::from_slice(&observation.stdout)?)
}

fn validate_apply(observation: &ShutdownObservation) -> Result<(), NativeError> {
    let value = completed_json(observation)?;
    if value.get("status").and_then(serde_json::Value::as_str) != Some("complete")
        || value
            .get("result")
            .and_then(|result| result.get("converged"))
            .and_then(serde_json::Value::as_bool)
            != Some(true)
    {
        return Err(NativeError::Protocol(
            "fresh cluster apply did not converge",
        ));
    }
    Ok(())
}

fn validate_unlock(observation: &ShutdownObservation, lock: &str) -> Result<(), NativeError> {
    if completed_json(observation)? != serde_json::json!({"unlocked": true, "lock_id": lock}) {
        return Err(NativeError::Protocol("exact lock release not acknowledged"));
    }
    Ok(())
}

fn complete_cli(
    command: &mut std::process::Command,
    cli: &NativeExecutable<'_>,
    lease: &LockedFile<'_>,
    root: &NativeRoot,
    assets: BootstrapAssets<'_>,
    limits: NativeLimits,
    call: NativeCall<'_>,
) -> Result<ShutdownObservation, NativeError> {
    let NativeCall {
        deadline,
        cancelled,
    } = call;
    let BootstrapAssets { bundle, files } = assets;
    let mut service = Service::spawn(
        command,
        ProcessLimits::new(limits.stdout, limits.stderr),
        cli.supervisor,
        deadline,
        cancelled,
    )?;
    loop {
        super::http::stop(deadline, cancelled)?;
        cli.check()?;
        lease.check_current()?;
        root.check()?;
        bundle.check_current()?;
        for file in files {
            file.check(bundle)?;
        }
        if !service.pump()? {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let output = service.finish(Duration::ZERO, deadline)?;
    lease.check_current()?;
    root.check()?;
    bundle.check_current()?;
    for file in files {
        file.check(bundle)?;
    }
    Ok(ShutdownObservation {
        status: output.status,
        stdout: output.stdout,
        stderr: output.stderr,
        forced_kill: output.forced_kill,
    })
}

pub(super) fn exact_lock<'a>(diagnostics: &'a [u8], root: &str) -> Result<&'a str, NativeError> {
    let text = std::str::from_utf8(diagnostics)
        .map_err(|_| NativeError::Protocol("invalid CLI diagnostics"))?;
    let mut roots = text
        .lines()
        .filter_map(|line| line.strip_prefix("Cluster-root: "));
    if roots.next() != Some(root) || roots.next().is_some() {
        return Err(NativeError::Protocol("ambiguous CLI root identity"));
    }
    let mut locks = text
        .lines()
        .filter_map(|line| line.strip_prefix("Admission lock: "));
    let lock = locks
        .next()
        .and_then(|line| {
            line.strip_suffix("; retain until prior work is quiescent, then exact-ID force-unlock")
        })
        .ok_or(NativeError::Protocol(
            "missing exact CLI admission identity",
        ))?;
    if locks.next().is_some()
        || lock.is_empty()
        || lock.len() > 128
        || !lock
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
        return Err(NativeError::Protocol("ambiguous CLI admission identity"));
    }
    Ok(lock)
}
