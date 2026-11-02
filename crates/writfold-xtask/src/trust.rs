use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    modules: Vec<Module>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Module {
    source: String,
    sha256: String,
    decision: String,
    attributes: Vec<String>,
}

struct Approval {
    digest: [u8; 32],
    attributes: BTreeSet<String>,
}

pub struct Policy {
    modules: BTreeMap<String, Approval>,
}

impl Policy {
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join("qualification/trusted-boundaries.json");
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self {
                    modules: BTreeMap::new(),
                });
            }
            Err(error) => return Err(error.to_string()),
        };
        let manifest: Manifest =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        if manifest.schema_version != 1 {
            return Err("unsupported proof trust schema".into());
        }
        let mut modules = BTreeMap::new();
        for module in manifest.modules {
            let path = Path::new(&module.source);
            if path.is_absolute()
                || path
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
                || module.decision.trim().is_empty()
                || module.attributes.is_empty()
            {
                return Err("invalid explicit proof trust approval".into());
            }
            let mut attributes = BTreeSet::new();
            for attribute in module.attributes {
                if !matches!(
                    attribute.as_str(),
                    "external_type_specification"
                        | "external_body"
                        | "external_fn_specification"
                        | "accept_recursive_types"
                ) || !attributes.insert(attribute)
                {
                    return Err("unsupported or duplicate proof trust attribute".into());
                }
            }
            let approval = Approval {
                digest: digest_bytes(&module.sha256)?,
                attributes,
            };
            if modules.insert(module.source, approval).is_some() {
                return Err("duplicate proof trust source".into());
            }
        }
        Ok(Self { modules })
    }

    pub fn admit<'a>(
        &'a self,
        source: &str,
        digest: &[u8; 32],
    ) -> Result<Option<&'a BTreeSet<String>>, String> {
        match self.modules.get(source) {
            None => Ok(None),
            Some(approval) if &approval.digest == digest => Ok(Some(&approval.attributes)),
            Some(_) => Err(format!(
                "proof trust source changed without admission: {source}"
            )),
        }
    }

    pub fn require_owned_sources(&self, sources: &BTreeSet<String>) -> Result<(), String> {
        if self.modules.keys().all(|source| sources.contains(source)) {
            Ok(())
        } else {
            Err("proof trust approval does not name inspected workspace source".into())
        }
    }
}

fn digest_bytes(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 {
        return Err("proof trust digest must have 64 lowercase hex digits".into());
    }
    let mut digest = [0; 32];
    for (slot, pair) in digest.iter_mut().zip(value.as_bytes().as_chunks::<2>().0) {
        let nibble = |byte| match byte {
            b'0'..=b'9' => Ok(byte - b'0'),
            b'a'..=b'f' => Ok(byte - b'a' + 10),
            _ => Err("invalid proof trust hexadecimal digest".to_owned()),
        };
        *slot = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Ok(digest)
}
