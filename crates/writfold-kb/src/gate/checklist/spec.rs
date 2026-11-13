//! The keys of a `checklist` gate definition:
//!
//! ```toml
//! items = ["errors: errors are handled or propagated", "perf: no new hot-path allocation"]
//! optional = ["perf"]                # items that never hold the gate open
//! ```
//!
//! Each item is `id: text`, its ID a slug unique in the gate; there is
//! at least one and at most [`MAX_ITEMS`]. `optional` names declared
//! items. Both keys are part of the definition digest.
use super::super::toml::Value;
use crate::write::is_slug;
use std::collections::BTreeMap;

/// The keys only a checklist gate may use.
pub const KEYS: [&str; 2] = ["items", "optional"];
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
}

/// A checklist gate's own fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Spec {
    pub items: Vec<Item>,
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
        format!(
            "items={}\noptional={}\n",
            serde_json::to_string(&items).unwrap_or_default(),
            serde_json::to_string(&optional).unwrap_or_default()
        )
    }

    /// The item with this ID.
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }
}

fn item(text: &str) -> Result<Item, String> {
    let Some((id, rest)) = text.split_once(':') else {
        return Err(format!("item {text:?} must be written \"id: text\""));
    };
    let (id, rest) = (id.trim(), rest.trim());
    if !is_slug(id) {
        return Err(format!("item ID {id:?} is not a slug"));
    }
    if rest.is_empty() || rest.contains(['\n', '\r']) {
        return Err(format!("item {id} needs one line of text after its ID"));
    }
    Ok(Item {
        id: id.to_owned(),
        text: rest.to_owned(),
        required: true,
    })
}

/// The checklist fields of a definition of class `class`, read from
/// `map`; `None` for any other class, which may not use them.
pub fn parse(map: &BTreeMap<String, Value>, class: &str) -> Result<Option<Spec>, String> {
    if class != super::super::def::CHECKLIST {
        return match KEYS.iter().find(|k| map.contains_key(**k)) {
            Some(key) => Err(format!("{key} is for gates of class checklist")),
            None => Ok(None),
        };
    }
    let declared = match map.get("items") {
        Some(Value::List(items)) if !items.is_empty() => items,
        _ => return Err("items must be a non-empty array of \"id: text\" strings".into()),
    };
    if declared.len() > MAX_ITEMS {
        return Err(format!("a checklist has at most {MAX_ITEMS} items"));
    }
    let mut items = declared
        .iter()
        .map(|t| item(t))
        .collect::<Result<Vec<_>, String>>()?;
    if let Some(i) = (1..items.len()).find(|i| items[..*i].iter().any(|x| x.id == items[*i].id)) {
        return Err(format!("item {} is declared twice", items[i].id));
    }
    let optional = match map.get("optional") {
        None => Vec::new(),
        Some(Value::List(names)) => names.clone(),
        Some(_) => return Err("optional must be an array of item IDs".into()),
    };
    for (n, name) in optional.iter().enumerate() {
        if optional[..n].contains(name) {
            return Err(format!("optional names {name} twice"));
        }
        let found = items.iter_mut().find(|i| &i.id == name);
        let Some(found) = found else {
            return Err(format!(
                "optional names {name:?}, which is not a declared item"
            ));
        };
        found.required = false;
    }
    Ok(Some(Spec { items }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::toml;

    fn spec(text: &str) -> Result<Option<Spec>, String> {
        parse(&toml::parse(text)?, "checklist")
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
        let plain = toml::parse("items = [\"a: x\"]\n")?;
        assert!(parse(&plain, "attested").is_err());
        assert_eq!(parse(&toml::parse("")?, "attested")?, None);
        Ok(())
    }
}
