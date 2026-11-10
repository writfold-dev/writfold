//! Shared typed application decisions. `serve` exposes transport control and,
//! when a session negotiates `writfold.trial/1`, the trial show/retain/prepare
//! decisions over the startup-configured state. No corpus, retained-release,
//! grounding or publication service is enabled, and approval stays CLI-only.
#![forbid(unsafe_code)]

mod contract;
mod engine;
pub mod ledger;
pub mod publication;
pub mod trial;
pub mod tuft;

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use writfold_domain::ByteCount;
use writfold_protocol::session::SessionError;
use writfold_runtime::stdio::{FrameReadError, FrameReader, InvocationIdentity};
use writfold_wire::rpc::{MAX_REQUEST_BYTES, response::ResponseError};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PROFILE: &str = writfold_wire::rpc::response::PROFILE;

#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    #[error(transparent)]
    Identity(#[from] writfold_runtime::stdio::IdentityError),
    #[error(transparent)]
    Frame(#[from] FrameReadError),
    #[error(transparent)]
    Response(#[from] ResponseError),
    #[error(transparent)]
    Output(#[from] io::Error),
    #[error(transparent)]
    ContractIdentity(#[from] writfold_domain::IdentityError),
    #[error(transparent)]
    Quantity(#[from] writfold_domain::QuantityError),
    #[error(transparent)]
    Limits(#[from] writfold_wire::Error),
    #[error("unexpected session transition refusal: {0:?}")]
    Session(SessionError),
    #[error("request allocator reported an occupied generation without active work")]
    GenerationInUse,
}

impl ServeError {
    /// A broken output pipe is cancellation, not a fabricated remote outcome.
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Output(error) if error.kind() == io::ErrorKind::BrokenPipe => 130,
            _ => 1,
        }
    }
}

/// The trial state for `serve [--state DIR]`: the option, else `WRITFOLD_STATE`,
/// else none (and `writfold.trial/1` then refuses). Never from a request.
pub fn serve_state(args: &[String]) -> Result<Option<PathBuf>, &'static str> {
    match args {
        [] => Ok(std::env::var_os("WRITFOLD_STATE").map(PathBuf::from)),
        [flag, dir] if flag == "--state" && !dir.is_empty() => Ok(Some(PathBuf::from(dir))),
        _ => Err("usage: writfold serve [--state DIR]"),
    }
}

/// Serve one sequential invocation over caller-owned IO. No filesystem corpus is
/// opened and no supplied state becomes authority. Only a session that
/// negotiated `writfold.trial/1` reaches `trial_state`, through the same trial
/// functions as the CLI (which launch Git). EOF/shutdown closes this
/// invocation. Output failure is fatal; never retry a partially delivered
/// frame or silently replace a response ID.
pub fn serve(
    trial_state: Option<&Path>,
    reader: impl BufRead,
    mut writer: impl Write,
) -> Result<(), ServeError> {
    let identity = InvocationIdentity::new()?;
    let contract = contract::digest()?;
    let mut engine = engine::Engine::new(&identity, &contract, trial_state)?;
    let maximum = ByteCount::new(MAX_REQUEST_BYTES as u64)?;
    let mut frames = FrameReader::new(reader, maximum)?;
    while let Some(frame) = frames.next_frame()? {
        let dispatch = engine.handle(frame)?;
        if let Some(bytes) = dispatch.response {
            writer.write_all(&bytes)?;
            writer.flush()?;
        }
        if dispatch.flow == engine::Flow::Stop {
            return Ok(());
        }
    }
    Ok(())
}
