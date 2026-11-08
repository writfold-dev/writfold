//! Real Linux/static-musl consumer; no mocked storage or authority effects.
//!
//! Parent-owned qualification plan, through admitted Compose/BuildKit storage:
//! 1. Build `cargo build --release --target x86_64-unknown-linux-musl -p
//!    writfold-store --example retained_objects_smoke`.
//! 2. Inspect the resulting ELF with `file`/`readelf` for static linking and no
//!    dynamic interpreter, then execute it with one nonexistent directory on the
//!    admitted operator mount. Its parent must already exist.
//! 3. Require exit zero and the final smoke marker. This exercises fresh/reopened
//!    stores, empty/binary/UTF-8+CRLF bytes, duplicate inode reuse, scope/length/
//!    budget refusals, corruption, symlink/hardlink/privacy and identity drift.
//!    It leaves the private evidence directory intact for inspection; do not
//!    reuse that directory for another invocation or as an effect-ready realm.

#[path = "support/object_smoke.rs"]
mod exercise;

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let Some(directory) = args.next() else {
        eprintln!("supply one new, nonexistent evidence directory");
        return ExitCode::FAILURE;
    };
    if args.next().is_some() {
        eprintln!("expected exactly one evidence directory");
        return ExitCode::FAILURE;
    }
    match exercise::run(&PathBuf::from(directory)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("retained object smoke refused: {error}");
            ExitCode::FAILURE
        }
    }
}
