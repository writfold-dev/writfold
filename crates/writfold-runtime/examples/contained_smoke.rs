use std::{path::PathBuf, process::ExitCode};
use writfold_domain::{ByteCount, QuantityError};
use writfold_runtime::fs::{
    ContainedRoot, FsError, LockKey, ManagedLocks, ReadExtent, RelativePath, WriteError,
};

#[derive(Debug, thiserror::Error)]
enum SmokeError {
    #[error("supply one new, nonexistent destination directory")]
    Arguments,
    #[error(transparent)]
    Filesystem(#[from] FsError),
    #[error(transparent)]
    Publication(#[from] WriteError),
    #[error(transparent)]
    Quantity(#[from] QuantityError),
    #[error("native behavior differs: {0}")]
    Behavior(&'static str),
}

fn exercise() -> Result<(), SmokeError> {
    let mut args = std::env::args_os().skip(1);
    let directory = PathBuf::from(args.next().ok_or(SmokeError::Arguments)?);
    if args.next().is_some() {
        return Err(SmokeError::Arguments);
    }
    let root = ContainedRoot::create(&directory)?;
    let locks = ManagedLocks::create(directory.join("locks"))?;
    let path = RelativePath::new("record.json").map_err(FsError::from)?;
    let target = root.pin(&path)?;
    let mut writer = target.try_lock(&locks, &LockKey::new("example-record")?)?;
    let limit = ByteCount::new(128)?;
    let first = writer.create(br#"{"revision":1}"#, limit)?;
    let second =
        writer.compare_and_write(Some(first.digest()), br#"{"revision":2}"#, limit, limit)?;
    match writer.compare_and_write(Some(first.digest()), br#"{"revision":3}"#, limit, limit) {
        Err(WriteError::NotPublished(FsError::StaleRevision { .. })) => {}
        Err(error) => return Err(error.into()),
        Ok(_) => return Err(SmokeError::Behavior("stale revision was published")),
    }
    let retained = writer.read(limit)?;
    if retained.extent() != ReadExtent::Complete
        || retained.bytes() != br#"{"revision":2}"#
        || retained.digest() != second.digest()
        || retained.identity() != second.identity()
    {
        return Err(SmokeError::Behavior("retained bytes or identity changed"));
    }
    println!(
        "record={} digest={} stale_update=refused",
        directory.join("record.json").display(),
        retained.digest()
    );
    Ok(())
}

fn main() -> ExitCode {
    match exercise() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
