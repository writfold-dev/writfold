//! What answers a checklist item (gate-evidence spec, stage 3):
//!
//! ```toml
//! own_evidence = ["errors"]                 # a look recorded --for this item
//! evidence_from = ["docs: docs-sweep"]      # only receipts or standings of these gates
//! ```
//!
//! An item in neither takes any grounded reference, as before
//! ([`Need::Any`]). An item in `evidence_from` takes only the gates it
//! names ([`Need::Named`]); one also in `own_evidence` takes those gates or
//! a look recorded for it ([`Need::Own`]). Each key names declared items
//! and joins the definition's canonical lines only when written, so a
//! checklist without them keeps its digest. The gates named are not
//! checked to exist: a gate that does not exist grounds nothing, so the
//! item cannot stand on it. The decision is [`writfold_core::bind`].
use super::allowed::Listed;
use super::spec::Item;
use crate::defs::{Check, Field};
use crate::write::is_slug;
use writfold_core::bind::Need;

/// The checklist keys this module reads.
pub const KEYS: [&str; 2] = ["own_evidence", "evidence_from"];

/// `item: gate gate …` entries as written, each item once.
pub type From = Option<Vec<(String, Vec<String>)>>;

/// Read `evidence_from`; `Some(None)` when absent.
fn from(c: &mut Check<'_>, items: &[Item]) -> Option<From> {
    let list = match c.list("evidence_from") {
        Field::Absent => return Some(None),
        Field::Bad => return None,
        Field::Got(l) => l,
    };
    let mut ok = true;
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for entry in &list.value {
        let text = entry.str();
        let why = match text.split_once(':') {
            None => Some(format!("{text:?} must be written \"item: gate …\"")),
            Some((id, gates)) => {
                let (id, gates) = (id.trim(), gates.split_whitespace());
                let gates: Vec<String> = gates.map(str::to_owned).collect();
                if !items.iter().any(|i| i.id == id) {
                    Some(format!(
                        "`evidence_from` names {id:?}, which is not a declared item"
                    ))
                } else if out.iter().any(|(x, _)| x == id) {
                    Some(format!("`evidence_from` names {id} twice"))
                } else if gates.is_empty() || gates.iter().any(|g| !is_slug(g)) {
                    Some(format!(
                        "{text:?} needs one or more gate IDs after the item"
                    ))
                } else {
                    out.push((id.to_owned(), gates));
                    None
                }
            }
        };
        if let Some(why) = why {
            c.error("value.form", Some(entry.span), why)
                .label("not `item: gate …`");
            ok = false;
        }
    }
    ok.then_some(Some(out))
}

/// Read both keys and set each item's need and named gates.
pub fn check(c: &mut Check<'_>, items: &mut [Item]) -> Option<(Listed, From)> {
    let own = super::allowed::listed(c, "own_evidence", items);
    let from = from(c, items);
    let (own, from) = (own?, from?);
    for item in items.iter_mut() {
        let mine = own.as_ref().is_some_and(|l| l.contains(&item.id));
        let named = (from.iter().flatten()).find(|(id, _)| *id == item.id);
        item.from = named.map(|(_, g)| g.clone()).unwrap_or_default();
        item.need = match (mine, named.is_some()) {
            (true, _) => Need::Own,
            (false, true) => Need::Named,
            (false, false) => Need::Any,
        };
    }
    Some((own, from))
}

/// The canonical lines these keys add, in declaration order, only for the
/// keys written.
pub fn canonical(items: &[Item], own: &Listed, from: &From) -> String {
    let mut out = String::new();
    if let Some(l) = own {
        let ids: Vec<&str> = (items.iter())
            .filter(|i| l.contains(&i.id))
            .map(|i| i.id.as_str())
            .collect();
        let json = serde_json::to_string(&ids).unwrap_or_default();
        out.push_str(&format!("own_evidence={json}\n"));
    }
    if let Some(l) = from {
        let lines: Vec<String> = (items.iter())
            .filter_map(|i| l.iter().find(|(id, _)| *id == i.id))
            .map(|(id, gates)| format!("{id}: {}", gates.join(" ")))
            .collect();
        let json = serde_json::to_string(&lines).unwrap_or_default();
        out.push_str(&format!("evidence_from={json}\n"));
    }
    out
}

/// What the item takes, for a refusal or a `why:`: `own evidence (a look
/// recorded --for gate.item, or gate:unit)`.
pub fn wanted(gate: &str, item: &Item) -> String {
    let gates: Vec<String> = item.from.iter().map(|g| format!("gate:{g}")).collect();
    match item.need {
        Need::Any => "any grounded evidence".to_owned(),
        // A generated unit (#156): only a worker's report prepared for it,
        // since no look can be recorded --for a name holding `~` (#168).
        Need::Own if gates.is_empty() && item.id.contains('~') => format!(
            "its own evidence (a report prepared with agent prepare --for {gate}.{})",
            item.id
        ),
        Need::Own if gates.is_empty() => {
            format!(
                "its own evidence (a look recorded --for {gate}.{})",
                item.id
            )
        }
        Need::Own => format!(
            "its own evidence (a look recorded --for {gate}.{}, or {})",
            item.id,
            gates.join(", ")
        ),
        Need::Named => format!("evidence from {}", gates.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use crate::defs::Check;
    use crate::gate::checklist::spec::{Spec, check};
    use writfold_core::bind::Need;

    fn spec(text: &str) -> Result<Spec, String> {
        let mut c = Check::read("f.toml", text, "a gate definition");
        let got = check(&mut c, "checklist");
        c.finish(got)
            .map_err(|r| crate::defs::why(&r))?
            .ok_or_else(|| "no spec".to_owned())
    }

    const ITEMS: &str = "items = [\"a: x\", \"b: y\", \"c: z\"]\n";

    #[test]
    fn needs_follow_the_keys_and_join_the_digest_only_when_written() -> Result<(), String> {
        let plain = spec(ITEMS)?;
        assert!(plain.items.iter().all(|i| i.need == Need::Any));
        assert!(!plain.canonical().contains("evidence"));
        let both = spec(&format!(
            "{ITEMS}own_evidence = [\"a\", \"b\"]\nevidence_from = [\"b: unit docs-sweep\", \"c: unit\"]\n"
        ))?;
        let needs: Vec<Need> = both.items.iter().map(|i| i.need).collect();
        assert_eq!(needs, [Need::Own, Need::Own, Need::Named]);
        assert_eq!(both.items[1].from, ["unit", "docs-sweep"]);
        assert!(both.items[0].from.is_empty());
        let canonical = both.canonical();
        assert!(
            canonical.contains("own_evidence=[\"a\",\"b\"]\n"),
            "{canonical}"
        );
        assert!(
            canonical.contains("evidence_from=[\"b: unit docs-sweep\",\"c: unit\"]\n"),
            "{canonical}"
        );
        for bad in [
            "own_evidence = [\"x\"]\n",
            "own_evidence = [\"a\", \"a\"]\n",
            "evidence_from = [\"a\"]\n",
            "evidence_from = [\"a:\"]\n",
            "evidence_from = [\"x: unit\"]\n",
            "evidence_from = [\"a: unit\", \"a: docs\"]\n",
            "evidence_from = [\"a: Not A Slug!\"]\n",
        ] {
            assert!(spec(&format!("{ITEMS}{bad}")).is_err(), "{bad}");
        }
        Ok(())
    }
}
