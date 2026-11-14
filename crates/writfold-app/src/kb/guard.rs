//! Arguments for the read-only `writfold kb guard status` projection.
use writfold_kb::Verb;
use writfold_kb::guard::GuardVerb;

pub const USAGE: &str = "usage: guard status";

/// Accept only the status subcommand; common flags are parsed separately.
pub fn verb(positional: &[String]) -> Result<Verb, String> {
    match positional {
        [sub] if sub == "status" => Ok(Verb::Guard(GuardVerb::Status)),
        _ => Err(USAGE.into()),
    }
}
