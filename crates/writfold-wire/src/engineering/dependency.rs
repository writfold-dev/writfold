//! Closed, budgeted dependency-lock import and canonical fingerprint material.
//!
//! This boundary validates flat references and declared call ranks, not package
//! authenticity, ABI compatibility, acquisition, or permission to execute.

use serde_json::{Map, Value};
use writfold_domain::{
    RealmId,
    engineering::{
        DependencyLockDigest, EngineeringDigest, FingerprintError, PackagePayloadDigest,
        dependency::{
            DependencyKind, DependencyLock, Edge, Endpoint, Export, ExportName, IdentityError,
            KindError, LockError, Package, PackageId, PackageRef,
        },
    },
};

use super::{cbor, fingerprint};
use crate::{Budget, Limits, RawJson, semantic::codec};

const NAMESPACE: &str = "writfold-engineering";
const KIND: &str = "dependency-lock";
const ROOT_FIELDS: &str = "namespace schema_version kind realm packages edges";

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum Error {
    #[error(transparent)]
    Json(#[from] crate::Error),
    #[error("unsupported dependency-lock namespace")]
    UnsupportedNamespace,
    #[error("unsupported dependency-lock schema version")]
    UnsupportedSchema,
    #[error("unsupported dependency-lock kind")]
    UnsupportedKind,
    #[error("invalid dependency-lock record shape")]
    Shape,
    #[error("invalid dependency-lock field {0}")]
    Field(&'static str),
    #[error("invalid dependency-lock identity {field}: {source}")]
    Identity {
        field: &'static str,
        #[source]
        source: IdentityError,
    },
    #[error(transparent)]
    Realm(#[from] writfold_domain::IdentityError),
    #[error(transparent)]
    Fingerprint(#[from] FingerprintError),
    #[error(transparent)]
    DependencyKind(#[from] KindError),
    #[error(transparent)]
    Lock(#[from] LockError),
    #[error(transparent)]
    Cbor(#[from] cbor::Error),
}

type Result<T, E = Error> = std::result::Result<T, E>;

/// Admit a closed JSON schema before moving any part of its raw tree.
/// Invalid arbitrarily nested values remain under RawJson's iterative drop.
pub fn decode(bytes: &[u8], limits: Limits) -> Result<DependencyLock> {
    let mut raw = RawJson::parse(bytes, limits)?;
    shape(raw.value())?;
    let mut root = object(std::mem::take(&mut raw.value))?;
    let realm = RealmId::try_from(string(take(&mut root, "realm")?, "realm")?)?;
    let packages = array(take(&mut root, "packages")?, "packages")?
        .into_iter()
        .map(package)
        .collect::<Result<Vec<_>>>()?;
    let edges = array(take(&mut root, "edges")?, "edges")?
        .into_iter()
        .map(edge)
        .collect::<Result<Vec<_>>>()?;
    Ok(DependencyLock::new(realm, packages, edges)?)
}

fn shape(value: &Value) -> Result<()> {
    let root = codec::obj(value, ROOT_FIELDS).map_err(|_| Error::Shape)?;
    if text(root, "namespace")? != NAMESPACE {
        return Err(Error::UnsupportedNamespace);
    }
    if number(field(root, "schema_version")?, "schema_version")? != 1 {
        return Err(Error::UnsupportedSchema);
    }
    if text(root, "kind")? != KIND {
        return Err(Error::UnsupportedKind);
    }
    text(root, "realm")?;
    for package in items(root, "packages")? {
        let package = codec::obj(package, "id payload exports").map_err(|_| Error::Shape)?;
        text(package, "id")?;
        text(package, "payload")?;
        for export in items(package, "exports")? {
            let export = codec::obj(export, "name rank").map_err(|_| Error::Shape)?;
            text(export, "name")?;
            number(field(export, "rank")?, "rank")?;
        }
    }
    for edge in items(root, "edges")? {
        let edge = codec::obj(edge, "source kind target").map_err(|_| Error::Shape)?;
        text(edge, "kind")?;
        for name in ["source", "target"] {
            let endpoint =
                codec::obj(field(edge, name)?, "id payload export").map_err(|_| Error::Shape)?;
            for name in ["id", "payload", "export"] {
                text(endpoint, name)?;
            }
        }
    }
    Ok(())
}

fn package(value: Value) -> Result<Package> {
    let mut fields = object(value)?;
    let reference = reference(&mut fields)?;
    let exports = array(take(&mut fields, "exports")?, "exports")?
        .into_iter()
        .map(|value| {
            let mut fields = object(value)?;
            let name = identity(&mut fields, "name")?;
            let rank = number(&take(&mut fields, "rank")?, "rank")?;
            Ok(Export::new(name, rank))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Package::new(reference, exports))
}

fn reference(fields: &mut Map<String, Value>) -> Result<PackageRef> {
    let id: PackageId = identity(fields, "id")?;
    let payload: PackagePayloadDigest = string(take(fields, "payload")?, "payload")?.parse()?;
    Ok(PackageRef::new(id, payload))
}

fn endpoint(value: Value) -> Result<Endpoint> {
    let mut fields = object(value)?;
    let reference = reference(&mut fields)?;
    let export: ExportName = identity(&mut fields, "export")?;
    Ok(Endpoint::new(reference, export))
}

fn edge(value: Value) -> Result<Edge> {
    let mut fields = object(value)?;
    let source = endpoint(take(&mut fields, "source")?)?;
    let kind: DependencyKind = string(take(&mut fields, "kind")?, "kind")?.parse()?;
    let target = endpoint(take(&mut fields, "target")?)?;
    Ok(Edge::new(source, kind, target))
}

/// Encode domain-normalized sets as schema-specific borrowed CBOR rows.
/// No source strings, digests, or nested value trees are cloned or re-sorted.
pub fn canonical(lock: &DependencyLock, limits: Limits) -> Result<cbor::RawCbor<'static>> {
    material_budget(lock, limits)?;
    use cbor::Value as C;
    let export_rows: Vec<Vec<[C<'_>; 2]>> = lock
        .packages()
        .iter()
        .map(|package| {
            package
                .exports()
                .iter()
                .map(|export| [C::Text(export.name().as_str()), C::Unsigned(export.rank())])
                .collect()
        })
        .collect();
    let export_arrays: Vec<Vec<C<'_>>> = export_rows
        .iter()
        .map(|rows| rows.iter().map(|row| C::Array(row)).collect())
        .collect();
    let package_rows: Vec<[C<'_>; 3]> = lock
        .packages()
        .iter()
        .zip(&export_arrays)
        .map(|(package, exports)| {
            [
                C::Text(package.reference().id().as_str()),
                C::Bytes(package.reference().payload().as_digest()),
                C::Array(exports),
            ]
        })
        .collect();
    let packages: Vec<_> = package_rows.iter().map(|row| C::Array(row)).collect();
    let edge_rows: Vec<[C<'_>; 7]> = lock
        .edges()
        .iter()
        .map(|edge| {
            [
                C::Text(edge.source().package().id().as_str()),
                C::Bytes(edge.source().package().payload().as_digest()),
                C::Text(edge.source().export().as_str()),
                C::Text(edge.kind().as_str()),
                C::Text(edge.target().package().id().as_str()),
                C::Bytes(edge.target().package().payload().as_digest()),
                C::Text(edge.target().export().as_str()),
            ]
        })
        .collect();
    let edges: Vec<_> = edge_rows.iter().map(|row| C::Array(row)).collect();
    let root = [
        ("namespace", C::Text(NAMESPACE)),
        ("schema_version", C::Unsigned(1)),
        ("kind", C::Text(KIND)),
        ("realm", C::Text(lock.realm().as_str())),
        ("packages", C::Array(&packages)),
        ("edges", C::Array(&edges)),
    ];
    Ok(cbor::encode(&C::Map(&root), limits)?)
}

/// Fingerprint canonical material in the dependency-lock domain only.
pub fn digest(lock: &DependencyLock, limits: Limits) -> Result<DependencyLockDigest> {
    Ok(fingerprint(&canonical(lock, limits)?))
}

// Bound temporary row allocation before constructing CBOR views. Exact output
// byte accounting remains with the encoder; every node requires at least a byte.
fn material_budget(lock: &DependencyLock, limits: Limits) -> Result<()> {
    let fail = |budget| Error::Cbor(cbor::Error::Limit { budget, offset: 0 });
    let container = |entries: usize, depth: usize| -> Result<()> {
        if depth > limits.depth() {
            return Err(fail(Budget::Depth));
        }
        if entries > limits.container_entries() {
            return Err(fail(Budget::ContainerEntries));
        }
        Ok(())
    };
    let mut nodes = 7_usize;
    let mut add = |count: usize, width: usize| -> Result<()> {
        nodes = count
            .checked_mul(width)
            .and_then(|n| nodes.checked_add(n))
            .ok_or_else(|| fail(Budget::Nodes))?;
        if nodes > limits.nodes() {
            return Err(fail(Budget::Nodes));
        }
        if nodes > limits.bytes() {
            return Err(fail(Budget::Bytes));
        }
        Ok(())
    };
    add(0, 0)?;
    container(6, 1)?;
    container(lock.packages().len(), 2)?;
    container(lock.edges().len(), 2)?;
    add(lock.packages().len(), 4)?;
    add(lock.edges().len(), 8)?;
    for package in lock.packages() {
        container(3, 3)?;
        container(package.exports().len(), 4)?;
        if !package.exports().is_empty() {
            container(2, 5)?;
        }
        add(package.exports().len(), 3)?;
    }
    if !lock.edges().is_empty() {
        container(7, 3)?;
    }
    Ok(())
}

fn object(value: Value) -> Result<Map<String, Value>> {
    match value {
        Value::Object(map) => Ok(map),
        _ => Err(Error::Shape),
    }
}
fn array(value: Value, name: &'static str) -> Result<Vec<Value>> {
    match value {
        Value::Array(values) => Ok(values),
        _ => Err(Error::Field(name)),
    }
}
fn take(fields: &mut Map<String, Value>, name: &'static str) -> Result<Value> {
    fields.remove(name).ok_or(Error::Field(name))
}
fn string(value: Value, name: &'static str) -> Result<String> {
    match value {
        Value::String(text) => Ok(text),
        _ => Err(Error::Field(name)),
    }
}
fn identity<T: TryFrom<String, Error = IdentityError>>(
    fields: &mut Map<String, Value>,
    name: &'static str,
) -> Result<T> {
    T::try_from(string(take(fields, name)?, name)?).map_err(|source| Error::Identity {
        field: name,
        source,
    })
}
fn field<'a>(fields: &'a Map<String, Value>, name: &'static str) -> Result<&'a Value> {
    fields.get(name).ok_or(Error::Field(name))
}
fn text<'a>(fields: &'a Map<String, Value>, name: &'static str) -> Result<&'a str> {
    field(fields, name)?.as_str().ok_or(Error::Field(name))
}
fn items<'a>(fields: &'a Map<String, Value>, name: &'static str) -> Result<&'a [Value]> {
    field(fields, name)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or(Error::Field(name))
}
fn number(value: &Value, name: &'static str) -> Result<u64> {
    value
        .as_number()
        .filter(|number| !number.as_str().contains(['.', 'e', 'E']))
        .and_then(serde_json::Number::as_u64)
        .ok_or(Error::Field(name))
}
