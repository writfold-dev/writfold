//! A trial refusal decided before the operation wrote anything.
//!
//! It displays exactly as its cause, so the CLI prints the same text. A typed
//! caller (`serve`) can tell it apart from a failure after a write began,
//! whose outcome it must report as unknown.
use super::Result;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct BeforeEffects(pub Box<dyn Error + Send + Sync>);

impl fmt::Display for BeforeEffects {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl Error for BeforeEffects {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&*self.0)
    }
}

/// Mark every error of `result` as decided before any write.
pub(super) fn before_effects<T>(result: Result<T>) -> Result<T> {
    result.map_err(|error| Box::new(BeforeEffects(error)) as _)
}
