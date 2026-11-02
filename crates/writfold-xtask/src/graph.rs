use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn allowed(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "writfold-domain" => &[],
        "writfold-kernel" | "writfold-protocol" => &["writfold-domain"],
        "writfold-wire" => &["writfold-domain", "writfold-kernel"],
        "writfold-sources" | "writfold-dolt" | "writfold-catalog" => {
            &["writfold-domain", "writfold-wire"]
        }
        "writfold-runtime" => &["writfold-domain", "writfold-protocol"],
        "writfold-store" => &[
            "writfold-domain",
            "writfold-protocol",
            "writfold-wire",
            "writfold-sources",
            "writfold-dolt",
            "writfold-runtime",
        ],
        "writfold-app" => &[
            "writfold-kernel",
            "writfold-store",
            "writfold-sources",
            "writfold-catalog",
            "writfold-wire",
            "writfold-runtime",
        ],
        "writfold" => &["writfold-app", "writfold-wire", "writfold-domain"],
        "writfold-xtask" => &[],
        _ => return None,
    })
}

pub fn check(metadata: &Value, diagnostics: &mut Vec<Value>) -> Result<(), String> {
    let packages = metadata["packages"]
        .as_array()
        .ok_or("metadata packages missing")?;
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("workspace members missing")?;
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or("resolved dependency graph missing")?;
    let by_id: BTreeMap<_, _> = packages
        .iter()
        .filter_map(|package| package["id"].as_str().map(|id| (id, package)))
        .collect();
    let local: BTreeSet<_> = members.iter().filter_map(Value::as_str).collect();
    if local.is_empty() || by_id.len() != packages.len() {
        return Err("invalid package identities".into());
    }
    let edges: BTreeMap<_, _> = nodes
        .iter()
        .filter_map(|node| node["id"].as_str().map(|id| (id, node)))
        .collect();
    for id in &local {
        let package = by_id
            .get(id)
            .ok_or("workspace member absent from packages")?;
        let name = package["name"].as_str().ok_or("package name missing")?;
        let Some(permitted) = allowed(name) else {
            issue(
                diagnostics,
                "architecture",
                name,
                "unclassified workspace member",
            );
            continue;
        };
        // Cargo's declaration list retains disabled optional, renamed, build,
        // dev and target-specific dependencies, independent of the host graph.
        for dependency in package["dependencies"]
            .as_array()
            .ok_or("dependencies missing")?
        {
            let target = dependency["name"]
                .as_str()
                .ok_or("dependency name missing")?;
            if dependency["path"].is_string() && !permitted.contains(&target) {
                issue(
                    diagnostics,
                    "architecture",
                    name,
                    &format!("forbidden local dependency {target} ({dependency})"),
                );
            }
            if forbidden_tls(target) {
                issue(diagnostics, "tls", name, target);
            }
        }
        let mut seen = BTreeSet::new();
        walk(id, name, &by_id, &edges, &local, &mut seen, diagnostics)?;
    }
    Ok(())
}

fn walk<'a>(
    id: &'a str,
    owner: &str,
    packages: &BTreeMap<&'a str, &'a Value>,
    edges: &BTreeMap<&'a str, &'a Value>,
    local: &BTreeSet<&str>,
    seen: &mut BTreeSet<&'a str>,
    diagnostics: &mut Vec<Value>,
) -> Result<(), String> {
    if !seen.insert(id) {
        return Ok(());
    }
    let package = packages.get(id).ok_or("resolved package absent")?;
    let name = package["name"]
        .as_str()
        .ok_or("resolved package name absent")?;
    if forbidden_tls(name) {
        issue(diagnostics, "tls", owner, name);
    }
    if matches!(
        owner,
        "writfold-domain" | "writfold-kernel" | "writfold-protocol"
    ) && !local.contains(id)
        && !pure_dependency(name)
    {
        issue(
            diagnostics,
            "core-dependency",
            owner,
            &format!("unadmitted external dependency {name}"),
        );
    }
    let node = edges.get(id).ok_or("resolved node absent")?;
    for dependency in node["deps"].as_array().ok_or("resolved deps absent")? {
        let next = dependency["pkg"]
            .as_str()
            .ok_or("resolved dependency identity absent")?;
        if local.contains(id) {
            let target = packages.get(next).ok_or("edge target absent")?;
            let target_name = target["name"].as_str().ok_or("edge target name absent")?;
            if local.contains(next)
                && !allowed(name).is_some_and(|list| list.contains(&target_name))
            {
                issue(
                    diagnostics,
                    "architecture",
                    name,
                    &format!("resolved edge to {target_name}: {dependency}"),
                );
            }
        }
        walk(next, owner, packages, edges, local, seen, diagnostics)?;
    }
    Ok(())
}

fn forbidden_tls(name: &str) -> bool {
    name == "native-tls"
        || name == "tokio-native-tls"
        || name == "hyper-tls"
        || name == "openssl"
        || name.starts_with("openssl-")
}

fn pure_dependency(name: &str) -> bool {
    // Exact reviewed pure arithmetic/proof/compiler-support closure. New
    // dependencies need classification, not an IO denylist that silently ages.
    matches!(
        name,
        "num-bigint"
            | "num-integer"
            | "num-traits"
            | "autocfg"
            | "vstd"
            | "verus_builtin"
            | "verus_builtin_macros"
            | "verus_state_machines_macros"
            | "verus_syn"
            | "syn"
            | "quote"
            | "proc-macro2"
            | "unicode-ident"
            | "verus_prettyplease"
            | "convert_case"
            | "synstructure"
            | "indexmap"
            | "hashbrown"
            | "equivalent"
    )
}

fn issue(diagnostics: &mut Vec<Value>, rule: &str, package: &str, message: &str) {
    diagnostics.push(json!({"rule":rule,"package":package,"message":message,"severity":"error"}));
}
