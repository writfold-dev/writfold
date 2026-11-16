//! Arguments for the read-only `writfold kb guard status` and `guard log`.
use super::args::Common;
use writfold_kb::Verb;
use writfold_kb::guard::GuardVerb;

pub const USAGE: &str = "usage: guard status | guard log [--rule ID] [--since AGE]";

/// `status`, or `log` with its two flags; common flags are parsed
/// separately.
pub fn verb(common: &Common, positional: &[String]) -> Result<Verb, String> {
    let rule = common.values.get("rule").cloned();
    let since = common.values.get("since");
    match positional {
        [sub] if sub == "status" && rule.is_none() && since.is_none() => {
            Ok(Verb::Guard(GuardVerb::Status))
        }
        [sub] if sub == "log" => {
            let since = match since {
                None => None,
                Some(text) => Some(
                    writfold_kb::verbs::runs::idle_age(text)
                        .ok_or_else(|| format!("--since takes an age such as 7d, not {text}"))?,
                ),
            };
            Ok(Verb::Guard(GuardVerb::Log { rule, since }))
        }
        _ => Err(USAGE.into()),
    }
}
