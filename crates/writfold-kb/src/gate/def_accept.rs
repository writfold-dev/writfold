//! `accept`: the standings that meet a gate (policy spec 2.2). Optional;
//! absent, every standing the class can reach meets it, which is
//! [`writfold_core::gate::Standing::met`]. The decision is
//! [`writfold_core::policy::met`]; this module only reads the key.
//!
//! | Class | Reaches | `accept` may say |
//! |---|---|---|
//! | mechanical, observed, run-record | satisfied, waived | `["satisfied"]` or both |
//! | attested, checklist | attested, waived | `["attested"]` or both |
//! | assessed | satisfied | `["satisfied"]` |
use super::def::{ASSESSED, ATTESTED, CHECKLIST};
use super::def_check::a_class;
use crate::defs::{Check, Field};
use writfold_core::policy::Accept;

/// Every standing word `accept` takes.
pub const STANDINGS: [&str; 3] = ["satisfied", "attested", "waived"];

/// The standings a class can reach, its own first.
pub fn reachable(class: &str) -> &'static [&'static str] {
    match class {
        ATTESTED | CHECKLIST => &["attested", "waived"],
        ASSESSED => &["satisfied"],
        _ => &["satisfied", "waived"],
    }
}

/// What the kernel reads: `None` (no `accept`) accepts every standing.
pub fn policy(accept: Option<&[String]>) -> Accept {
    let has = |w: &str| accept.is_none_or(|a| a.iter().any(|x| x == w));
    Accept {
        satisfied: has("satisfied"),
        attested: has("attested"),
        waived: has("waived"),
    }
}

/// The words of `accept`, in the order [`STANDINGS`] lists them, for the
/// digest and the JSON row.
fn canonical(words: &[String]) -> Vec<String> {
    STANDINGS
        .iter()
        .filter(|s| words.iter().any(|w| w == *s))
        .map(|s| (*s).to_owned())
        .collect()
}

/// Read `accept` for a gate of class `class`: `Some(None)` when absent,
/// `None` when it has a fault (reported to `c`).
pub fn check(c: &mut Check<'_>, class: &str) -> Option<Option<Vec<String>>> {
    let list = match c.list("accept") {
        Field::Absent => return Some(None),
        Field::Bad => return None,
        Field::Got(l) => l,
    };
    let reach = reachable(class);
    let mut ok = true;
    let mut words: Vec<String> = Vec::new();
    for (n, w) in list.value.iter().enumerate() {
        if list.value[..n].iter().any(|x| x.str() == w.str()) {
            let why = format!("`accept` names {} twice", w.str());
            c.error("value.twice", Some(w.span), why)
                .label("named again");
            ok = false;
        } else if !STANDINGS.contains(&w.str()) {
            let why = format!("unknown standing {:?}", w.str());
            c.unknown_value(w.span, why, "not a standing", w.str(), &STANDINGS, '"');
            ok = false;
        } else if !reach.contains(&w.str()) {
            let why = format!("{} is never {}", a_class(class), w.str());
            let d = c.error("value.class", Some(w.span), why);
            d.label("this class cannot reach it");
            d.help(format!("{} reaches: {}", a_class(class), reach.join(", ")));
            ok = false;
        } else {
            words.push(w.value.text.clone());
        }
    }
    if ok && !words.iter().any(|w| w == reach[0]) {
        let why = format!(
            "accept must include {:?}: {} that accepts only a waiver can never be met by its own evidence",
            reach[0],
            a_class(class)
        );
        let d = c.error("value.unmeetable", Some(list.span), why);
        d.label("its own standing is missing");
        d.help(format!("write accept = [{:?}]", reach[0]));
        ok = false;
    }
    ok.then(|| Some(canonical(&words)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str, class: &str) -> Result<Option<Vec<String>>, String> {
        let mut c = Check::read("f.toml", text, "a gate definition");
        let got = check(&mut c, class);
        c.finish(got).map_err(|r| crate::defs::why(&r))
    }

    #[test]
    fn accept_faults_are_named() {
        assert_eq!(read("", "mechanical"), Ok(None));
        let ok = read("accept = [\"waived\", \"satisfied\"]\n", "mechanical");
        assert_eq!(ok, Ok(Some(vec!["satisfied".into(), "waived".into()])));
        let unknown = read("accept = [\"satisfeid\"]\n", "mechanical");
        assert!(unknown.is_err_and(|e| e.contains("unknown standing")));
        let class = read("accept = [\"attested\"]\n", "mechanical");
        assert!(class.is_err_and(|e| e.contains("a mechanical gate is never attested")));
        let unmeetable = read("accept = [\"waived\"]\n", "checklist");
        assert!(unmeetable.is_err_and(|e| e.contains("accept must include \"attested\"")));
        let twice = read("accept = [\"satisfied\", \"satisfied\"]\n", "mechanical");
        assert!(twice.is_err_and(|e| e.contains("names satisfied twice")));
        let assessed = read("accept = [\"satisfied\", \"waived\"]\n", "assessed");
        assert!(assessed.is_err_and(|e| e.contains("never waived")));
    }

    #[test]
    fn no_accept_is_every_standing() {
        let all = policy(None);
        assert!(all.satisfied && all.attested && all.waived);
        let strict = policy(Some(&["satisfied".into()]));
        assert!(strict.satisfied && !strict.attested && !strict.waived);
    }
}
