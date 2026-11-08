//! Pure judgments over explicitly supplied evidence, context and model authority.
//! Production authentication and durable effects belong to concrete adapters.
#![forbid(unsafe_code)]

pub mod checker;
pub mod citation;
pub mod design_workflow;
pub mod ledger;
pub mod portfolio;
pub mod procedure;
pub mod time;
pub mod wellformed;
pub mod workflow;
