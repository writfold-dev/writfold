//! `kb lint --records`: an alias written in a Markdown record's front
//! matter (#41) that another record's name
//! already holds. The measured way an alias goes wrong is that it attracts
//! phrases meant for another record (find-next RFC), so a warning is
//! given when every word of an alias is among the words of another
//! record's ID (whole or after its typed prefix), title or alias, compared
//! as `find` compares names ([`writfold_core::find::norm`]). A warning,
//! not an error: two records may share a word on purpose. Reads the
//! records as the index does; tested.
use super::Sink;
use super::lint::File;
use std::collections::BTreeSet;
use writfold_core::diag::{Severity, Span};
use writfold_core::find::{norm, tokens};

/// One record's names, as `find` reads them.
struct Named {
    path: String,
    id: String,
    /// Each name with its normalized words.
    names: Vec<(String, BTreeSet<String>)>,
    /// The aliases written in its front matter.
    authored: Vec<String>,
}

fn words(name: &str) -> BTreeSet<String> {
    tokens(&norm(name)).into_iter().map(str::to_owned).collect()
}

fn named(file: &File) -> Option<Named> {
    let kind = crate::layout::classify(&file.shown)?;
    let facts = crate::parse::parse(&file.shown, kind, file.text.as_bytes()).ok()?;
    let record = facts.record?;
    let bare = record
        .id
        .split_once([':', '/'])
        .map(|(_, rest)| rest.to_owned());
    let mut names: Vec<String> = vec![record.id.clone()];
    names.extend(bare);
    names.extend(record.title.clone());
    names.extend(facts.aliases.iter().map(|(a, _)| a.clone()));
    // Only the Markdown key (#41) is checked: a JSON asset's `aliases`
    // predate it and are left as they were.
    let markdown = file.shown.ends_with(".md");
    let authored = (facts.aliases.iter())
        .filter(|(_, source)| markdown && *source == "alias")
        .map(|(a, _)| a.clone())
        .collect();
    Some(Named {
        path: file.shown.clone(),
        id: record.id,
        names: names.into_iter().map(|n| (n.clone(), words(&n))).collect(),
        authored,
    })
}

/// Where `alias` is written in the front matter, if it can be found.
fn span(text: &str, alias: &str) -> Option<Span> {
    let start = text.find("aliases")?;
    let at = start + text[start..].find(alias)?;
    Some(Span::new(at, at + alias.len()))
}

/// The warnings, attached to the files they are about.
pub fn check(files: &mut [File]) {
    let records: Vec<Named> = files.iter().filter_map(named).collect();
    for r in &records {
        let Some(file) = files.iter_mut().find(|f| f.shown == r.path) else {
            continue;
        };
        let mut sink = Sink::new(&r.path, &file.text);
        for alias in &r.authored {
            let mine = words(alias);
            if mine.is_empty() {
                continue;
            }
            let other = records.iter().filter(|o| o.id != r.id).find_map(|o| {
                let (name, _) = o.names.iter().find(|(_, w)| mine.is_subset(w))?;
                Some((o, name))
            });
            let Some((o, name)) = other else { continue };
            let d = sink.push(
                Severity::Warning,
                "alias.collides",
                span(&file.text, alias),
                format!(
                    "alias {alias:?} is within {} ({name:?}): a phrase using its words may find that record instead",
                    o.id
                ),
            );
            d.help = Some(format!(
                "make the alias name what only {} is, or drop it",
                r.id
            ));
        }
        let report = sink.report();
        if !report.diagnostics.is_empty() {
            file.report.extend(report);
        }
    }
}
