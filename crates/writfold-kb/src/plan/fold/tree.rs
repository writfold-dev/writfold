//! Post-order expansion retains each step's template-use identity.
use super::super::event::{self, Splice};
use super::super::template::Template;
use super::{MAX_DEPTH, Step, Use};

pub(super) struct Tree<'a> {
    pub splices: &'a [(String, Splice, Template)],
    pub library: &'a [Template],
    pub used: Vec<bool>,
    pub steps: Vec<Step>,
    pub uses: Vec<Use>,
}

/// Post-order expansion of the splices under `parent`.
pub(super) fn expand(tree: &mut Tree<'_>, parent: &str, depth: usize) {
    if depth >= MAX_DEPTH {
        return;
    }
    let splices = tree.splices;
    for (i, (_, s, t)) in splices.iter().enumerate() {
        if s.parent != parent || tree.used[i] {
            continue;
        }
        tree.used[i] = true;
        tree.uses.push(Use {
            parent: s.parent.clone(),
            template: s.template.clone(),
            digest: s.digest.clone(),
            ordinal: s.ordinal,
        });
        let prefix = event::prefix(&s.parent, &s.template, s.ordinal);
        steps_of(tree, i, (t, &s.source), &prefix, depth);
    }
}

/// The steps of `t` (the template of splice `i`, or one it includes)
/// under `prefix`, each after its include and the splices under it.
fn steps_of(
    tree: &mut Tree<'_>,
    i: usize,
    (t, source): (&Template, &str),
    prefix: &str,
    depth: usize,
) {
    let (splices, library) = (tree.splices, tree.library);
    let (path, s, _) = &splices[i];
    let usage = tree.uses.len() - 1;
    for step in &t.steps {
        let node = format!("{prefix}/{}", step.id);
        let include = step.include.as_deref().and_then(|name| {
            let recorded = s.includes.iter().find(|i| i.name == name)?;
            let found = library.iter().find(|l| l.digest == recorded.digest)?;
            Some((found, recorded.source.as_str()))
        });
        if let (Some(included), true) = (include, depth + 1 < MAX_DEPTH) {
            let inner = event::prefix(&node, &included.0.name, 1);
            tree.uses.push(Use {
                parent: node.clone(),
                template: included.0.name.clone(),
                digest: included.0.digest.clone(),
                ordinal: 1,
            });
            steps_of(tree, i, included, &inner, depth + 1);
        }
        expand(tree, &node, depth + 1);
        tree.steps.push(Step {
            path: node,
            id: step.id.clone(),
            gates: step.gates.clone(),
            template: t.name.clone(),
            digest: t.digest.clone(),
            source: source.to_owned(),
            splice: path.clone(),
            usage,
            root: s.parent.is_empty(),
            done: None,
            skipped: None,
            begin: s.records.clone(),
        });
    }
}
