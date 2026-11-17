//! What a mode's policy takes (`waivable`, a mode gate's own `accept`):
//! the refusal of a waiver it does not take, and the `why:` of a statement
//! that exists anyway (written before the key, or by hand). The decision
//! itself is [`writfold_core::policy::met`] through
//! [`super::config::Obligation::accept`].
use super::config::Obligation;

/// Where a mode gate's definition lives, as shown.
fn gate_file(id: &str) -> String {
    format!("{}/{id}.toml", crate::gate::def::DIR)
}

/// The command that meets an obligation without a statement.
fn meet(ob: &Obligation, id: &str) -> String {
    match ob.class {
        "attested" => format!(
            "writfold kb deliver attest {id} --check {} --statement TEXT --by WHO",
            ob.name
        ),
        _ => format!("writfold kb deliver check {id}"),
    }
}

/// The refusal of `deliver waive` when the policy does not take a waiver
/// of `ob`; `None` when it does.
pub fn no_waiver(mode: &str, ob: &Obligation, id: &str) -> Option<String> {
    if ob.accept().waived {
        return None;
    }
    Some(match &ob.gate {
        Some(d) => format!(
            "gate {} does not accept a waiver ({}); meet it: {}",
            d.id,
            crate::gate::policy::accept_at(d, &gate_file(&d.id)),
            meet(ob, id)
        ),
        None => format!(
            "{} does not accept a waiver of {} ({}); meet it: {}",
            mode,
            ob.name,
            ob.waivable_at,
            meet(ob, id)
        ),
    })
}

/// The `why:` of a met word the policy does not take.
pub fn why_rejected(ob: &Obligation, word: &str) -> String {
    match &ob.gate {
        Some(d) => format!(
            "{} does not accept {word} ({})",
            ob.name,
            crate::gate::policy::accept_at(d, &gate_file(&d.id))
        ),
        None => format!("{} does not accept {word} ({})", ob.name, ob.waivable_at),
    }
}
