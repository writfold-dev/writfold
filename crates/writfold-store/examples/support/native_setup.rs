use super::{Result, count};
use std::fs;
use std::io::Read;
use std::path::Path;
use writfold_domain::GraphId;
use writfold_runtime::fs::{ContainedRoot, LockedFile, ReadExtent, RelativePath};
use writfold_runtime::omnigraph::{
    BootstrapGraph, Credential, NativeCall, NativeError, NativeExecutable, NativeLimits, NativeRoot,
};

pub enum Source<'a> {
    Fresh(&'a Path),
    Prepared(&'a Path),
}

pub fn prepare(
    work: &Path,
    source: Source<'_>,
    graph: &GraphId,
    cli: &NativeExecutable<'_>,
    lease: &LockedFile<'_>,
    limits: NativeLimits,
    call: NativeCall<'_>,
) -> Result<NativeRoot> {
    let fixture = match source {
        Source::Prepared(path) => return Ok(NativeRoot::prepared(path)?),
        Source::Fresh(path) => ContainedRoot::open(path)?,
    };
    let schema = asset(&fixture, "schema.pg")?;
    let queries = asset(&fixture, "queries.gq")?;
    let policy = asset(&fixture, "graph.policy.yaml")?;
    let prepared = NativeRoot::bootstrap_fresh(
        &work.join("cluster"),
        lease,
        cli,
        BootstrapGraph {
            graph,
            schema: &schema,
            queries: &queries,
            policy: &policy,
        },
        count(1 << 20)?,
        limits,
        call,
    );
    let (root, observation) = match prepared {
        Ok(result) => result,
        Err(NativeError::Bootstrap(failure)) => {
            fs::write(work.join("failed-apply.stdout"), &failure.apply.stdout)?;
            fs::write(work.join("failed-apply.stderr"), &failure.apply.stderr)?;
            if let Some(unlock) = &failure.unlock {
                fs::write(work.join("failed-unlock.stdout"), &unlock.stdout)?;
                fs::write(work.join("failed-unlock.stderr"), &unlock.stderr)?;
            }
            return Err(failure.to_string().into());
        }
        Err(error) => return Err(error.into()),
    };
    fs::write(work.join("apply.stdout"), observation.apply.stdout)?;
    fs::write(work.join("apply.stderr"), observation.apply.stderr)?;
    fs::write(work.join("unlock.stdout"), observation.unlock.stdout)?;
    fs::write(work.join("unlock.stderr"), observation.unlock.stderr)?;
    Ok(root)
}

pub fn credentials() -> Result<Vec<Credential>> {
    Ok(vec![
        Credential::new("act-reader", &random_token()?)?,
        Credential::new("act-writer", &random_token()?)?,
    ])
}

fn asset(root: &ContainedRoot, name: &str) -> Result<Vec<u8>> {
    let result = root.pin(&RelativePath::new(name)?)?.read(count(1 << 20)?)?;
    if result.extent() != ReadExtent::Complete {
        return Err("fixture asset exceeds bound".into());
    }
    Ok(result.into_bytes())
}

fn random_token() -> Result<String> {
    let mut bytes = [0; 32];
    fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let mut token = String::with_capacity(64);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in bytes {
        token.push(char::from(HEX[usize::from(byte >> 4)]));
        token.push(char::from(HEX[usize::from(byte & 15)]));
    }
    Ok(token)
}
