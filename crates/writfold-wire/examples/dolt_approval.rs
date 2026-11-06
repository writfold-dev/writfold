//! Development-only supplied-input approval consumer, not a publisher or installed CLI.
//! Qualification uses only the repository's public synthetic signing key.
use serde_json::json;
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use writfold_domain::{ContentDigest, RealmId};
use writfold_wire::Limits;
use writfold_wire::dolt_operation::DoltOperation;
use writfold_wire::policy::AuthorityPolicy;
use writfold_wire::signatures::ApprovalSigningKey;
use writfold_wire::signatures::dolt::{DoltApproval, verify_dolt_current};

type Result<T, E = Box<dyn Error>> = std::result::Result<T, E>;
const MAX_BYTES: usize = 1_048_576;
const USAGE: &str = "dolt_approval --sign OP POLICY PUBLIC_TEST_PEM TOOLCHAIN REALM; or --verify OP APPROVAL POLICY TOOLCHAIN REALM";

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            let _ = serde_json::to_writer(
                io::stderr().lock(),
                &json!({"status": "refused", "message": error.to_string()}),
            );
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args.len() != 6 {
        return Err(invalid(USAGE).into());
    }
    let mode = args[0].to_str().ok_or_else(|| invalid(USAGE))?;
    if mode != "--sign" && mode != "--verify" {
        return Err(invalid(USAGE).into());
    }
    let limits = Limits::new(MAX_BYTES, 128, 10_000, 100_000)?;
    let operation = DoltOperation::decode(&read(Path::new(&args[1]))?, limits)?;
    let policy_index = if mode == "--sign" { 2 } else { 3 };
    let policy = AuthorityPolicy::historical(&read(Path::new(&args[policy_index]))?, limits)?;
    let toolchain: ContentDigest = args[4].to_str().ok_or_else(|| invalid(USAGE))?.parse()?;
    let realm: RealmId = args[5].to_str().ok_or_else(|| invalid(USAGE))?.parse()?;
    if mode == "--sign" {
        let pem = read(Path::new(&args[3]))?;
        let signer = ApprovalSigningKey::from_pkcs8_pem(std::str::from_utf8(&pem)?)?;
        let approval = signer.sign_dolt(
            &operation,
            &policy,
            operation.operation().actor()?,
            &toolchain,
            &realm,
        )?;
        io::stdout()
            .lock()
            .write_all(&approval.canonical_bytes(MAX_BYTES)?)?;
    } else {
        let approval = DoltApproval::decode(&read(Path::new(&args[2]))?, limits)?;
        let verified = verify_dolt_current(&operation, &approval, &policy, &toolchain, &realm)?;
        serde_json::to_writer(
            io::stdout().lock(),
            &json!({
                "namespace": "writfold-dolt-approval-check", "schema": 1,
                "status": "supplied-inputs-verified", "plan": verified.operation().id().as_str(),
                "realm": realm.as_str(), "kb": policy.kb_id().as_str(), "write_authority": false,
            }),
        )?;
    }
    Ok(())
}

fn read(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        return Err(invalid("input exceeds byte limit").into());
    }
    Ok(bytes)
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
