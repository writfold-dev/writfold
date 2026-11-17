//! A gate's policy as the verbs read it: which standings meet the gate
//! (`accept`, decided by [`writfold_core::policy::met`]), and whether any
//! policy is configured at all.
use writfold_core::policy::Accept;

/// Every standing that meets a gate: the rule for a definition without
/// `accept`.
pub fn all() -> Accept {
    Accept {
        satisfied: true,
        attested: true,
        waived: true,
    }
}

/// The line `resume`, the session-start context and `kb policy` show when
/// nothing is configured.
pub const NONE_CONFIGURED: &str = "policy: none configured (continuity only: checkpoints, records and citations work without one)";

/// The directories under `.writfold/` that hold policy files.
const POLICY_DIRS: [&str; 4] = ["gates", "plans", "guards", "deliveries"];

/// Whether `top/.writfold/` holds any gate, plan template, required file,
/// guard rule or delivery file (a `.toml` in one of those directories).
fn holds(top: &std::path::Path) -> bool {
    POLICY_DIRS.iter().any(|dir| {
        std::fs::read_dir(top.join(".writfold").join(dir)).is_ok_and(|entries| {
            entries.flatten().any(|e| {
                let path = e.path();
                path.extension().is_some_and(|x| x == "toml")
            })
        })
    })
}

/// Whether the KB root or the subject repository configures any policy.
pub fn configured(root: &std::path::Path, repo: Option<&std::path::Path>) -> bool {
    holds(root) || repo.is_some_and(holds)
}

/// The line of `key` in a definition file (1 when it cannot be found).
pub fn key_line(d: &super::def::Definition, key: &str) -> u32 {
    let file = d.dir.join(format!("{}.toml", d.id));
    let Ok(text) = std::fs::read_to_string(file) else {
        return 1;
    };
    let at = text.lines().position(|l| {
        let l = l.trim_start();
        l.strip_prefix(key)
            .is_some_and(|rest| rest.trim_start().starts_with('='))
    });
    at.map_or(1, |n| u32::try_from(n + 1).unwrap_or(1))
}

/// `accept = ["satisfied"] @.writfold/gates/unit.toml:8`.
pub fn accept_at(d: &super::def::Definition, shown: &str) -> String {
    let words = d.accept.clone().unwrap_or_default();
    let list = serde_json::to_string(&words).unwrap_or_default();
    format!(
        "accept = {} @{shown}:{}",
        list.replace(',', ", "),
        key_line(d, "accept")
    )
}

/// The `why:` of a standing the gate does not accept.
pub fn why_not_accepted(d: &super::def::Definition, standing: &str, shown: &str) -> String {
    let words = d.accept.clone().unwrap_or_default().join(", ");
    let what = match standing {
        "waived" => "the waiver",
        "attested" => "the attestation",
        _ => "this standing",
    };
    format!(
        "this gate accepts only: {words} (@{shown}:{}); {what} is recorded and does not meet it",
        key_line(d, "accept")
    )
}

/// A gate's standing word, with `, not accepted` when its `accept` leaves
/// that word out: `waived, not accepted`.
pub fn shown(j: &super::status::Judged<'_>) -> String {
    match j.not_accepted() {
        true => format!("{}, not accepted", j.standing.name()),
        false => j.standing.name().to_owned(),
    }
}

/// The refusal of `gate waive` on a gate whose `accept` leaves out
/// `waived`, naming what meets it instead.
pub fn no_waiver(d: &super::def::Definition, subject: &super::subject::Subject) -> String {
    let shown = super::status::shown_path(d, subject);
    let meet = if d.is_checklist() {
        format!(
            "writfold kb gate cover {} <item> supported REF… --by WHO",
            d.id
        )
    } else if d.attested() {
        format!("writfold kb gate attest {} --statement TEXT --by WHO", d.id)
    } else {
        format!("writfold kb gate run {}", d.id)
    };
    format!(
        "gate {} does not accept a waiver ({}); meet it: {meet}",
        d.id,
        accept_at(d, &shown)
    )
}

/// The additive JSON keys of a gate row: `accept` when the definition
/// writes it, `accepted: false` when its standing is a met word `accept`
/// leaves out.
pub fn json_keys(row: &mut crate::emit::Row, j: &super::status::Judged<'_>) {
    if let Some(words) = &j.definition.accept {
        row.extra.push(("accept", serde_json::json!(words)));
    }
    if j.not_accepted() {
        row.extra.push(("accepted", serde_json::json!(false)));
    }
}

impl super::status::Judged<'_> {
    /// Whether the gate is met under its policy ([`writfold_core::policy::met`]):
    /// the one meaning of met wherever a gate is judged.
    pub fn met(&self) -> bool {
        writfold_core::policy::met(self.standing, &self.definition.policy())
    }

    /// A met word (attested or waived) that the gate's `accept` leaves out.
    pub fn not_accepted(&self) -> bool {
        self.standing.met() && !self.met()
    }
}

/// Flag a row whose standing the gate's `accept` leaves out:
/// `!not-accepted`, with why.
pub fn flag(row: &mut crate::emit::Row, j: &super::status::Judged<'_>, shown: &str) {
    if !j.not_accepted() {
        return;
    }
    row.flags.push("not-accepted".into());
    row.more.push(crate::emit::More {
        label: "why",
        text: why_not_accepted(&j.definition, j.standing.name(), shown),
        ..crate::emit::More::default()
    });
}
