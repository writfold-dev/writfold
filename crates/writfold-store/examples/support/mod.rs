use std::error::Error;
use std::io::{self, Write};
use writfold_protocol::dolt::ReadRefusal;
use writfold_runtime::{dolt::DoltError, fs::FsError, process::ProcessError};
use writfold_store::dolt::Error as StoreError;
use writfold_wire::dolt::Error as WireError;

#[derive(serde::Serialize)]
struct Diagnostic<'a> {
    kind: &'static str,
    message: &'a str,
}

pub fn emit(error: &(dyn Error + 'static)) -> io::Result<()> {
    let message = error.to_string();
    let diagnostic = Diagnostic {
        kind: kind(error),
        message: &message,
    };
    let mut stderr = io::stderr().lock();
    serde_json::to_writer(&mut stderr, &diagnostic).map_err(io::Error::other)?;
    stderr.write_all(b"\n")
}

fn kind(error: &(dyn Error + 'static)) -> &'static str {
    if let Some(error) = error.downcast_ref::<StoreError>() {
        return match error {
            StoreError::Runtime(error) => runtime(error),
            StoreError::Wire(error) => wire(error),
            StoreError::Pin(error) => match error {
                ReadRefusal::Realm => "pin.realm",
                ReadRefusal::Kb => "pin.kb",
                ReadRefusal::Database => "pin.database",
                ReadRefusal::Commit => "pin.commit",
                ReadRefusal::Epoch => "pin.epoch",
                ReadRefusal::Profile => "pin.profile",
                ReadRefusal::Producer => "pin.producer",
            },
            StoreError::Schema => "schema_mismatch",
            StoreError::Query => "query_mismatch",
            StoreError::RetentionRoot => "retention_root",
            StoreError::DanglingEdge { .. } => "dangling_edge",
            StoreError::RowLimitOverflow => "row_overflow",
            StoreError::Deadline | StoreError::DeadlineOverflow => "deadline",
            StoreError::Cancelled => "cancelled",
            StoreError::Digest(_) => "contract_digest",
        };
    }
    if let Some(error) = error.downcast_ref::<DoltError>() {
        return runtime(error);
    }
    if let Some(error) = error.downcast_ref::<WireError>() {
        return wire(error);
    }
    if let Some(error) = error.downcast_ref::<FsError>() {
        return filesystem(error);
    }
    "invalid_argument"
}

fn runtime(error: &DoltError) -> &'static str {
    match error {
        DoltError::Filesystem(error) => filesystem(error),
        DoltError::Process(error) => match error {
            ProcessError::Budget {
                stream: "stdout", ..
            } => "stdout_limit",
            ProcessError::Budget {
                stream: "stderr", ..
            } => "stderr_limit",
            ProcessError::Budget { .. } => "process_budget",
            ProcessError::Deadline => "deadline",
            ProcessError::Cancelled => "cancelled",
            ProcessError::Command {
                status: Some(1), ..
            } => "native_command",
            ProcessError::Command { .. } => "native_failure",
            ProcessError::Cleanup(_) => "cleanup_failed",
            ProcessError::UnsupportedProfile(_) => "process_profile",
            ProcessError::Io(_) | ProcessError::ClosedStdin => "process_io",
        },
        DoltError::Claim(_) => "writer_claim",
        DoltError::ExecutableMismatch => "executable_mismatch",
        DoltError::UnsupportedProfile(_) => "unsupported_profile",
        DoltError::Budget { .. } => "native_budget",
        DoltError::RowLimit => "row_overflow",
        DoltError::QueryContract => "query_contract",
    }
}

fn filesystem(error: &FsError) -> &'static str {
    match error {
        FsError::Missing => "fs.missing",
        FsError::Symlink => "fs.symlink",
        FsError::NotDirectory => "fs.not_directory",
        FsError::NonRegular => "fs.nonregular",
        FsError::ChangedSource => "fs.changed",
        FsError::HardLinked => "fs.hardlinked",
        FsError::LockBusy => "fs.lock_busy",
        FsError::Permission => "fs.permission",
        FsError::Budget { .. } => "fs.budget",
        _ => "fs.refused",
    }
}

fn wire(error: &WireError) -> &'static str {
    match error {
        WireError::InvalidIdentity { .. } => "wire.identity",
        WireError::RowCount => "scalar_rows",
        WireError::RowLimit => "row_limit",
        WireError::RowOrder { .. } => "row_order",
        WireError::InvalidShape => "wire.shape",
        WireError::InvalidField { .. } => "wire.field",
        WireError::NullPayload => "wire.null_payload",
        WireError::UnsupportedNamespace => "wire.namespace",
        WireError::UnsupportedVersion => "wire.version",
        WireError::UnsupportedKind => "wire.kind",
        WireError::Wire(_) => "wire.syntax_or_budget",
    }
}
