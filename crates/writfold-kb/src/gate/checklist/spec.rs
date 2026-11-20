//! The keys of a `checklist` gate definition:
//!
//! ```toml
//! items = ["errors: errors are handled or propagated", "perf: no new hot-path allocation"]
//! optional = ["perf"]                # items that never hold the gate open
//! ```
//!
//! Each item is `id: text`, its ID a slug unique in the gate; there is
//! at least one and at most [`MAX_ITEMS`]. `optional` names declared
//! items. Both keys are part of the definition digest. The reasoned
//! outcomes' keys are [`super::allowed`]'s, what answers an item is
//! [`super::evidence`]'s.
use crate::defs::{Check, Spanned, Text};
use crate::write::is_slug;

/// The keys only a checklist gate may use.
pub const KEYS: [&str; 7] = [
    "items",
    "optional",
    "not_applicable",
    "none_found",
    "snapshot",
    "own_evidence",
    "evidence_from",
];
/// The most items one checklist may declare.
pub const MAX_ITEMS: usize = 64;

/// One declared item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Item {
    pub id: String,
    /// What the item asks, as written after its ID.
    pub text: String,
    /// False for an item named in `optional`.
    pub required: bool,
    /// `not-applicable` covers it ([`super::allowed`]).
    pub na: bool,
    /// `none-found` covers it.
    pub none: bool,
    /// `supported` covers it only citing a look that kept a snapshot.
    pub snapshot: bool,
    /// What answers it ([`super::evidence`]).
    pub need: writfold_core::bind::Need,
    /// The gates `evidence_from` names for it.
    pub from: Vec<String>,
}

/// A checklist gate's own fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Spec {
    pub items: Vec<Item>,
    /// `not_applicable`, `none_found` and `snapshot` as written
    /// ([`super::allowed`]).
    pub lists: super::allowed::Lists,
    /// `own_evidence` and `evidence_from` as written ([`super::evidence`]).
    pub own_evidence: super::allowed::Listed,
    pub evidence_from: super::evidence::From,
}

impl Spec {
    /// The lines the definition digest adds for these fields.
    pub fn canonical(&self) -> String {
        let items: Vec<String> = self
            .items
            .iter()
            .map(|i| format!("{}: {}", i.id, i.text))
            .collect();
        let optional: Vec<&str> = self
            .items
            .iter()
            .filter(|i| !i.required)
            .map(|i| i.id.as_str())
            .collect();
        let own = super::allowed::canonical(&self.items, &self.lists);
        let needs =
            super::evidence::canonical(&self.items, &self.own_evidence, &self.evidence_from);
        format!(
            "items={}\noptional={}\n{own}{needs}",
            serde_json::to_string(&items).unwrap_or_default(),
            serde_json::to_string(&optional).unwrap_or_default()
        )
    }

    /// Whether the policy restricts the reasoned outcomes at all.
    pub fn restricted(&self) -> bool {
        self.lists.na.is_some() || self.lists.none.is_some()
    }

    /// The item with this ID.
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }
}

fn item(c: &mut Check<'_>, text: &Spanned<Text>) -> Option<Item> {
    let Some((id, rest)) = text.str().split_once(':') else {
        let why = format!("item {:?} must be written \"id: text\"", text.str());
        c.error("value.form", Some(text.span), why)
            .label("no `:` after the item's ID");
        return None;
    };
    let (shown, rest) = (id.trim(), rest.trim());
    if !is_slug(shown) {
        let at = text.sub(id.len() - id.trim_start().len(), shown.len());
        let why = format!("item ID {shown:?} is not a slug");
        c.error("value.form", Some(at), why).label("not a slug");
        return None;
    }
    if rest.is_empty() || rest.contains(['\n', '\r']) {
        let why = format!("item {shown} needs one line of text after its ID");
        c.error("value.form", Some(text.span), why)
            .label("no text after the ID");
        return None;
    }
    Some(Item {
        id: shown.to_owned(),
        text: rest.to_owned(),
        required: true,
        na: true,
        none: false,
        snapshot: false,
        need: writfold_core::bind::Need::Any,
        from: Vec::new(),
    })
}

/// The declared items, each once.
fn items(c: &mut Check<'_>) -> Option<Vec<Item>> {
    let add = "items = [\"errors: errors are handled or propagated\"]";
    let declared = c.need_list("items", "a checklist gate", add)?;
    if declared.len() > MAX_ITEMS {
        let span = c.doc.get("items").and_then(|e| match &e.value {
            crate::defs::Slot::Value(v) => Some(v.span),
            crate::defs::Slot::Broken => None,
        });
        let why = format!(
            "a checklist has at most {MAX_ITEMS} items, and this has {}",
            declared.len()
        );
        c.error("key.range", span, why).label("too many items");
        return None;
    }
    let mut items: Vec<Item> = Vec::new();
    let mut ok = true;
    for text in &declared {
        match item(c, text) {
            Some(i) if items.iter().any(|x| x.id == i.id) => {
                let why = format!("item {} is declared twice", i.id);
                c.error("value.twice", Some(text.span), why)
                    .label("declared again");
                ok = false;
            }
            Some(i) => items.push(i),
            None => ok = false,
        }
    }
    ok.then_some(items)
}

/// The checklist fields of a definition of class `class`; `Some(None)` for
/// any other class, which may not use them. Every fault is reported to
/// `c`; `None` when there was one.
pub fn check(c: &mut Check<'_>, class: &str) -> Option<Option<Spec>> {
    if class != super::super::def::CHECKLIST {
        let mut ok = true;
        for key in KEYS {
            let why = format!("`{key}` is for gates of class checklist");
            ok &= !c.not_here(key, why, "only a checklist gate takes it");
        }
        return ok.then_some(None);
    }
    let items = items(c);
    let optional = c.list("optional").or(Spanned {
        value: Vec::new(),
        span: Default::default(),
    })?;
    let mut items = items?;
    let ids: Vec<String> = items.iter().map(|i| i.id.clone()).collect();
    let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mut ok = true;
    for (n, name) in optional.value.iter().enumerate() {
        if optional.value[..n].iter().any(|x| x.str() == name.str()) {
            let why = format!("`optional` names {} twice", name.str());
            c.error("value.twice", Some(name.span), why)
                .label("named again");
            ok = false;
            continue;
        }
        match items.iter_mut().find(|i| i.id == name.str()) {
            Some(found) => found.required = false,
            None => {
                let why = format!(
                    "`optional` names {:?}, which is not a declared item",
                    name.str()
                );
                c.unknown_value(
                    name.span,
                    why,
                    "not an item of this gate",
                    name.str(),
                    &ids,
                    '"',
                );
                c.sink
                    .found
                    .last_mut()
                    .into_iter()
                    .for_each(|d| d.code = "ref.undefined");
                ok = false;
            }
        }
    }
    let lists = super::allowed::check(c, &mut items)?;
    let (own_evidence, evidence_from) = super::evidence::check(c, &mut items)?;
    ok.then_some(Some(Spec {
        items,
        lists,
        own_evidence,
        evidence_from,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec_as(text: &str, class: &str) -> Result<Option<Spec>, String> {
        let mut c = Check::read("f.toml", text, "a gate definition");
        let got = check(&mut c, class);
        c.finish(got).map_err(|r| crate::defs::why(&r))
    }

    fn spec(text: &str) -> Result<Option<Spec>, String> {
        spec_as(text, "checklist")
    }

    #[test]
    fn items_and_optional_are_checked() -> Result<(), String> {
        let ok = spec(
            "items = [\"errors: handled\", \"perf : no new allocation\"]\noptional = [\"perf\"]\n",
        )?
        .ok_or("no spec")?;
        assert_eq!(ok.items.len(), 2);
        assert!(ok.items[0].required && !ok.items[1].required);
        assert_eq!(ok.items[1].text, "no new allocation");
        assert!(ok.canonical().contains("optional=[\"perf\"]"));
        let many: Vec<String> = (0..=MAX_ITEMS).map(|i| format!("\"i{i}: x\"")).collect();
        for bad in [
            "items = []\n".to_owned(),
            "optional = [\"a\"]\n".to_owned(),
            "items = [\"a\"]\n".to_owned(),
            "items = [\"A: x\"]\n".to_owned(),
            "items = [\"a:  \"]\n".to_owned(),
            "items = [\"a: x\", \"a: y\"]\n".to_owned(),
            "items = [\"a: x\"]\noptional = [\"b\"]\n".to_owned(),
            "items = [\"a: x\"]\noptional = [\"a\", \"a\"]\n".to_owned(),
            "items = [\"a: x\"]\noptional = \"a\"\n".to_owned(),
            format!("items = [{}]\n", many.join(", ")),
        ] {
            assert!(spec(&bad).is_err(), "{bad}");
        }
        assert!(spec_as("items = [\"a: x\"]\n", "attested").is_err());
        assert_eq!(spec_as("", "attested")?, None);
        Ok(())
    }
}
