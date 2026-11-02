use crate::{macros, syntax::Audit, tokens};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
};

struct Unit<'a> {
    path: PathBuf,
    package: &'a str,
    package_root: &'a Path,
    file: syn::File,
    lines: BTreeSet<usize>,
}

fn load<'a>(
    root: &Path,
    metadata: &'a Value,
    diagnostics: &mut Vec<Value>,
) -> Result<Vec<Unit<'a>>, String> {
    let packages = metadata["packages"].as_array().ok_or("packages absent")?;
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("members absent")?;
    let mut units = Vec::new();
    let mut found_source = false;
    for package in packages
        .iter()
        .filter(|package| members.contains(&package["id"]))
    {
        let manifest = Path::new(
            package["manifest_path"]
                .as_str()
                .ok_or("manifest path absent")?,
        );
        let directory = manifest.parent().ok_or("manifest directory absent")?;
        if !directory.starts_with(root) {
            return Err(format!(
                "workspace source outside root: {}",
                directory.display()
            ));
        }
        let name = package["name"].as_str().ok_or("package name absent")?;
        let mut files = Vec::new();
        discover(directory, &mut files)?;
        for path in files {
            found_source = true;
            let source = fs::read_to_string(&path).map_err(|error| error.to_string())?;
            let tokens = source
                .parse::<proc_macro2::TokenStream>()
                .map_err(|error| error.to_string())?;
            let mut lines = tokens::lines(tokens);
            let nonblank: BTreeSet<_> = source
                .lines()
                .enumerate()
                .filter_map(|(index, line)| (!line.trim().is_empty()).then_some(index + 1))
                .collect();
            lines.retain(|line| nonblank.contains(line));
            match syn::parse_file(&source) {
                Ok(file) => units.push(Unit {
                    path,
                    package: name,
                    package_root: directory,
                    file,
                    lines,
                }),
                Err(error) => {
                    let span = error.span();
                    diagnostics.push(json!({"rule":"syntax","severity":"error","path":path,
                        "line":span.start().line,"column":span.start().column + 1,
                        "end_line":span.end().line,"end_column":span.end().column + 1,
                        "message":error.to_string()}));
                }
            }
        }
    }
    if !found_source {
        return Err("no workspace Rust sources inspected".into());
    }
    Ok(units)
}

pub fn check(root: &Path, metadata: &Value, diagnostics: &mut Vec<Value>) -> Result<(), String> {
    let units = load(root, metadata, diagnostics)?;
    let mut definitions = Definitions(BTreeSet::new());
    for unit in &units {
        definitions.visit_file(&unit.file);
    }
    let owned_sources: BTreeSet<_> = units
        .iter()
        .map(|unit| (unit.package, unit.path.as_path()))
        .collect();
    for unit in &units {
        let relative = unit
            .path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_string_lossy();
        let dev = unit.path.components().any(|component| {
            matches!(
                component.as_os_str().to_str(),
                Some("tests" | "examples" | "benches")
            )
        });
        let mut audit = Audit {
            path: &relative,
            package: unit.package,
            file: unit.path.as_path(),
            package_root: unit.package_root,
            owned_sources: &owned_sources,
            lines: &unit.lines,
            diagnostics,
            definitions: &definitions.0,
            test: dev,
            ghost: false,
        };
        let orchestration = unit
            .path
            .file_name()
            .is_some_and(|name| name == "main.rs" || name == "build.rs")
            || unit.package == "writfold-app";
        // No proof-family 500-line exception is currently admitted. All Verus
        // files therefore retain the stricter ordinary 400-line budget.
        let limit = if orchestration { 250 } else { 400 };
        audit.budget(unit.file.span(), limit, 250, "module-budget");
        audit.visit_file(&unit.file);
    }
    Ok(())
}

fn discover(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        let path = entry.path();
        if kind.is_symlink() {
            return Err(format!("uninspected source symlink: {}", path.display()));
        }
        if kind.is_dir() && entry.file_name() != "target" && entry.file_name() != ".git" {
            discover(&path, files)?;
        } else if kind.is_file() && path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files.sort();
    Ok(())
}

struct Definitions(BTreeSet<String>);
impl<'ast> Visit<'ast> for Definitions {
    fn visit_item_macro(&mut self, node: &'ast syn::ItemMacro) {
        if node.mac.path.is_ident("macro_rules")
            && let Some(name) = &node.ident
        {
            self.0.insert(name.to_string());
        }
        if node.mac.path.is_ident("verus")
            && let Ok(file) = verus_syn::parse2::<verus_syn::File>(node.mac.tokens.clone())
        {
            verus_syn::visit::Visit::visit_file(self, &file);
        }
        visit::visit_item_macro(self, node);
    }
}

impl<'ast> verus_syn::visit::Visit<'ast> for Definitions {
    fn visit_item_macro(&mut self, node: &'ast verus_syn::ItemMacro) {
        if let Some(name) = macros::declared_name(node) {
            self.0.insert(name.to_string());
        }
        if node.mac.path.is_ident("verus")
            && let Ok(file) = verus_syn::parse2::<verus_syn::File>(node.mac.tokens.clone())
        {
            verus_syn::visit::Visit::visit_file(self, &file);
        }
        verus_syn::visit::visit_item_macro(self, node);
    }
}
