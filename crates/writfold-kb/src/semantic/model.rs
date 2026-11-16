//! Local-only pinned Potion inference. No ordinary KB path calls `get`.
use std::sync::OnceLock;

use model2vec_rs::model::StaticModel;
use sha2::{Digest, Sha256};

use crate::error::{KbError, Result};

const PIN: &str = "bf8b056651a2c21b8d2565580b8569da283cab23";
const ASSETS: [(&str, &str); 3] = [
    (
        "tokenizer.json",
        "e67e803f624fb4d67dea1c730d06e1067e1b14d830e2c2202569e3ef0f70bb50",
    ),
    (
        "model.safetensors",
        "f65d0f325faadc1e121c319e2faa41170d3fa07d8c89abd48ca5358d9a223de2",
    ),
    (
        "config.json",
        "2a6ac0e9aaa356a68a5688070db78fc3a464fefe85d2f06a1905ce3718687553",
    ),
];
static MODEL: OnceLock<std::result::Result<Model, String>> = OnceLock::new();

pub(super) struct Model {
    inner: StaticModel,
    identity: String,
}

pub(super) fn get() -> Result<&'static Model> {
    MODEL
        .get_or_init(load)
        .as_ref()
        .map_err(|error| KbError::Mismatch(format!("semantic model: {error}")))
}

impl Model {
    pub(super) fn identity(&self) -> &str {
        &self.identity
    }

    pub(super) fn dimension(&self) -> usize {
        self.inner.dim()
    }

    pub(super) fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Upstream's borrowed single-text API caps at 512 tokens after its
        // median-token-length character pre-truncation. Bind both in identity.
        let vector = self.inner.encode_single(text);
        if vector.iter().any(|value| !value.is_finite()) {
            return Err(KbError::Mismatch(
                "semantic model produced a non-finite embedding".into(),
            ));
        }
        Ok(vector)
    }
}

fn checked(name: &str, bytes: &[u8], expected: &str) -> std::result::Result<(), String> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let matches = digest
        .iter()
        .zip(expected.as_bytes().chunks_exact(2))
        .all(|(&byte, pair)| pair == [HEX[usize::from(byte >> 4)], HEX[usize::from(byte & 15)]]);
    if expected.len() != 64 || !matches {
        return Err(format!(
            "{name}: SHA-256 mismatch; fetch the pinned model assets"
        ));
    }
    Ok(())
}

fn from_bytes(assets: [&[u8]; 3]) -> std::result::Result<Model, String> {
    for ((name, expected), bytes) in ASSETS.iter().zip(assets) {
        checked(name, bytes, expected)?;
    }
    // from_bytes decodes the F32 table once, as required by upstream's safe API;
    // neither packaging clones input bytes or converts queries into Strings.
    let inner = StaticModel::from_bytes(assets[0], assets[1], assets[2], Some(true))
        .map_err(|error| format!("cannot decode pinned assets: {error}"))?;
    if inner.dim() != 256 {
        return Err("pinned model must have 256 dimensions".into());
    }
    let identity = format!(
        "minishlab/potion-base-8M@{PIN};tokenizer={};weights={};config={};model2vec-rs=0.4.0;f32;l2;encode_single;max_tokens=512;char_cap=512*median_token_length",
        ASSETS[0].1, ASSETS[1].1, ASSETS[2].1,
    );
    Ok(Model { inner, identity })
}

#[cfg(feature = "semantic-embedded")]
fn load() -> std::result::Result<Model, String> {
    from_bytes([
        include_bytes!(concat!(
            env!("WRITFOLD_SEMANTIC_MODEL_BUILD"),
            "/tokenizer.json"
        )),
        include_bytes!(concat!(
            env!("WRITFOLD_SEMANTIC_MODEL_BUILD"),
            "/model.safetensors"
        )),
        include_bytes!(concat!(
            env!("WRITFOLD_SEMANTIC_MODEL_BUILD"),
            "/config.json"
        )),
    ])
}

#[cfg(not(feature = "semantic-embedded"))]
fn load() -> std::result::Result<Model, String> {
    load_directory(&model_directory()?)
}

#[cfg(not(feature = "semantic-embedded"))]
fn model_directory() -> std::result::Result<std::path::PathBuf, String> {
    use std::{env, path::PathBuf};

    if let Some(path) = env::var_os("WRITFOLD_SEMANTIC_MODEL") {
        return Ok(PathBuf::from(path));
    }
    let share = match env::var_os("XDG_DATA_HOME").filter(|path| !path.is_empty()) {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(
            env::var_os("HOME")
                .filter(|path| !path.is_empty())
                .ok_or("set WRITFOLD_SEMANTIC_MODEL (or HOME/XDG_DATA_HOME)")?,
        )
        .join(".local/share"),
    };
    Ok(share.join("writfold/models/potion-base-8M").join(PIN))
}

#[cfg(any(test, not(feature = "semantic-embedded")))]
fn load_directory(path: &std::path::Path) -> std::result::Result<Model, String> {
    let read = |name| {
        let file = path.join(name);
        std::fs::read(&file).map_err(|error| format!("{}: {error}", file.display()))
    };
    let tokenizer = read(ASSETS[0].0)?;
    let weights = read(ASSETS[1].0)?;
    let config = read(ASSETS[2].0)?;
    from_bytes([&tokenizer, &weights, &config])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_and_corrupt_assets() {
        let impossible = std::path::Path::new(file!()).join("not-a-model-directory");
        assert!(load_directory(&impossible).is_err());
        for (name, checksum) in ASSETS {
            assert!(checked(name, b"", checksum).is_err());
            assert!(checked(name, b"corrupt model", checksum).is_err());
        }
        assert!(from_bytes([b"{}", b"", b"{}"]).is_err());
    }

    #[test]
    fn checksum_accepts_only_exact_bytes() {
        let digest = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(checked("test", b"abc", digest).is_ok());
        assert!(checked("test", b"abd", digest).is_err());
        assert!(checked("test", b"abc", &digest[..62]).is_err());
    }

    #[cfg(feature = "semantic-embedded")]
    #[test]
    fn pinned_embeddings_are_finite_normalized_and_deterministic() -> Result<()> {
        let model = get()?;
        assert_eq!(model.dimension(), 256);
        assert!(model.identity().contains(PIN));
        for text in ["database backup restore", "", "日本語", "\u{2603}"] {
            let vector = model.embed(text)?;
            assert_eq!(vector.len(), 256);
            assert!(vector.iter().all(|value| value.is_finite()));
            let norm: f32 = vector.iter().map(|value| value * value).sum();
            assert!(norm == 0.0 || (norm - 1.0).abs() < 1e-5);
            assert_eq!(vector, model.embed(text)?);
        }
        assert!(model.embed("")?.iter().all(|value| *value == 0.0));
        Ok(())
    }
}
