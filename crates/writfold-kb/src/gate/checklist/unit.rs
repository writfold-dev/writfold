//! Unit-local staleness (#166, RFC 2026-11-26-research-contracts-second-slice):
//! a generated checklist's row for the pair `a~b` records the digests of
//! the seed, `a` and `b` it is about, and is current while the checklist's
//! definition and those three are unchanged ([`writfold_core::units::unit_moved`]).
//! Another member's edit cannot move it: it is not an input. A row without
//! unit digests (written before) is judged as before. Finding the digests
//! is tested; the decision is the kernel's.
use super::cover::Cover;
use super::generated::Universe;
use crate::gate::def::Definition;
use writfold_core::units::{Moved, unit_moved};

/// The digests one unit is about now: the seed's and its two members'.
pub(super) fn digests(u: &Universe, item: &str) -> Option<[String; 3]> {
    let seed = u.seed.clone()?;
    let (a, b) = item.split_once('~')?;
    let find = |id: &str| (u.members.iter().find(|m| m.id == id)).map(|m| m.digest.clone());
    Some([seed, find(a)?, find(b)?])
}

/// Whether a unit row is current and, when not, what moved; `None` for a
/// row with no unit digests.
pub(super) fn currency(
    d: &Definition,
    u: Option<&Universe>,
    c: &Cover,
) -> Option<(bool, Option<String>)> {
    let then = c.unit.as_ref()?;
    let now = u.and_then(|u| digests(u, &c.item));
    let same = |i: usize| now.as_ref().is_some_and(|n| n.get(i) == then.get(i));
    let moved = unit_moved(c.definition == d.digest, same(0), same(1), same(2));
    let (a, b) = c.item.split_once('~').unwrap_or_default();
    let why = moved.map(|m| match m {
        Moved::Definition => "the definition changed since this row".to_owned(),
        Moved::Seed => "the seed changed since this row".to_owned(),
        Moved::First => format!("{a} changed since this row"),
        Moved::Second => format!("{b} changed since this row"),
    });
    Some((moved.is_none(), why))
}

#[cfg(test)]
mod tests {
    use super::super::generated::{Member, Universe};
    use super::currency;
    use crate::gate::checklist::cover::Cover;
    use writfold_core::checklist::Outcome;

    fn universe(edited: u32, seed: &str) -> Universe {
        let members = (0..6u32)
            .map(|i| Member {
                id: format!("m{i}"),
                path: format!("m{i}.md"),
                digest: format!("d{i}-{}", u32::from(edited & (1 << i) != 0)),
            })
            .collect();
        Universe {
            present: true,
            units: 15,
            why: None,
            members,
            seed: Some(seed.to_owned()),
        }
    }

    fn row(d: &crate::gate::def::Definition, u: &Universe, item: &str) -> Option<Cover> {
        Some(Cover {
            gate: d.id.clone(),
            item: item.to_owned(),
            outcome: Outcome::Supported,
            reason: None,
            note: None,
            by: "agent".into(),
            run: "run-1".into(),
            repo: "kb".into(),
            commit: "c".into(),
            content: "sha256:00".into(),
            definition: d.digest.clone(),
            evidence: Vec::new(),
            seq: 1,
            recorded: "2026-11-26T00:00:00Z".into(),
            unit: super::digests(u, item).map(Some)?,
        })
    }

    /// Over every set of edited members: exactly the units naming an
    /// edited member go stale, and editing the seed stales every unit.
    #[test]
    fn exactly_the_units_naming_an_edited_member_go_stale() -> Result<(), String> {
        let text = "schema = \"writfold-gate/1\"\nid = \"pair-review\"\nversion = 1\nclass = \"checklist\"\nscope = [\"**\"]\nitems_from = \"related-scope/manifest.json\"\nunits = \"pairs\"\n";
        let origin = crate::gate::def::Origin::Kb;
        let d = crate::gate::def::parse(text, "pair-review", origin, std::path::Path::new("."))
            .map_err(|e| e.to_string())?;
        let before = universe(0, "s");
        for edited in 0..64u32 {
            let after = universe(edited, "s");
            for a in 0..6u32 {
                for b in (a + 1)..6 {
                    let item = format!("m{a}~m{b}");
                    let c = row(&d, &before, &item).ok_or(format!("no unit digests for {item}"))?;
                    let touched = edited & ((1 << a) | (1 << b)) != 0;
                    let got = currency(&d, Some(&after), &c);
                    assert_eq!(
                        got.map(|(current, _)| current),
                        Some(!touched),
                        "{item} {edited:06b}"
                    );
                    let seeded = currency(&d, Some(&universe(edited, "s2")), &c);
                    assert_eq!(seeded.map(|(current, _)| current), Some(false));
                }
            }
        }
        Ok(())
    }

    /// No look can be recorded --for a unit (#168): `observe --for` takes
    /// a slug item, and a unit's name holds `~`. That is what keeps an
    /// observation from covering a unit; removing it fails here.
    #[test]
    fn no_look_is_recorded_for_a_unit() {
        assert!(crate::observe::item("pair-review.a~b").is_none());
        assert!(crate::observe::item("pair-review.errors").is_some());
    }
}
