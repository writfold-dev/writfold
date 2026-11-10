//! Publications cancelled (their native processes killed) after 0, 20, 40, ...
//! ms until one completes uncancelled. Each attempt is its own signed operation
//! and candidate. Every outcome is resolved by `observe` alone; an `Interrupted`
//! publication is finished by `complete`, which issues only Git steps. At the
//! end every attempt is published exactly once or not at all.
use super::world::{Result, STOP, World, candidate, reads};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use writfold_app::publication::{
    Candidate, Observation, Publication, Releases, complete, observe, publish,
};
use writfold_domain::GitObjectId;
use writfold_domain::dolt::DoltReadPin;
use writfold_store::backend::dolt::{DoltBackend, DoltWriteApproval};

pub const STEP: Duration = Duration::from_millis(20);
pub const ATTEMPTS: u32 = 400;

/// How the sweep's attempts ended and how they resolved.
#[derive(Clone, Copy, Debug, Default)]
pub struct Tally {
    pub attempts: u32,
    /// Refused or stale before the storage commit; observed `NotPublished`.
    pub refused: u32,
    /// `Unknown`; observed `NotPublished`.
    pub unknown_not_published: u32,
    /// `Unknown`; observed `Interrupted`, then `complete` published it.
    pub unknown_completed: u32,
    /// `Unknown`; observed `Published`.
    pub unknown_published: u32,
    pub published: u32,
}

pub fn sweep(world: &mut World<'_, '_, '_, '_>) -> Result<Tally> {
    let mut tally = Tally::default();
    let mut release = world.genesis.clone();
    let mut pin = world.base.clone();
    for attempt in 0..ATTEMPTS {
        tally.attempts += 1;
        let content = format!("kill attempt {attempt}");
        let next = candidate("doc:kill", &content)?;
        let approval = world.signer.approve(&release, &pin, attempt + 1)?;
        let outcome = cancelled_after(STEP * attempt, |cancelled| {
            publish(world.backend, world.releases, &approval, &next, cancelled)
        });
        let seen = observe(world.backend, world.releases, &approval, &next, &STOP)?;
        let direct = matches!(outcome, Ok(Publication::Published { .. }));
        let resolved = match (outcome, seen) {
            (
                Ok(Publication::Published { release, pin }),
                Observation::Published { release: r, pin: p },
            ) if release == r && pin == p => Some((release, pin)),
            (Ok(Publication::Unknown), Observation::NotPublished) => {
                tally.unknown_not_published += 1;
                None
            }
            (Ok(Publication::Unknown), Observation::Published { release, pin }) => {
                tally.unknown_published += 1;
                Some((release, pin))
            }
            (Ok(Publication::Unknown), Observation::Interrupted { pin: applied, .. }) => {
                tally.unknown_completed += 1;
                Some(finish(
                    world.backend,
                    world.releases,
                    &approval,
                    &next,
                    &applied,
                )?)
            }
            (Ok(Publication::Stale { .. }) | Err(_), Observation::NotPublished) => {
                tally.refused += 1;
                None
            }
            (outcome, seen) => {
                return Err(
                    format!("attempt {attempt}: {outcome:?} disagrees with {seen:?}").into(),
                );
            }
        };
        if let Some((new_release, new_pin)) = resolved {
            tally.published += 1;
            if reads(world.backend, &new_pin, "doc:kill")? != content.as_bytes() {
                return Err(format!("attempt {attempt}: published bytes differ").into());
            }
            release = new_release;
            pin = new_pin;
        }
        if direct {
            verify(world, &tally, &release, &pin)?;
            return Ok(tally);
        }
    }
    Err("no publication completed within the kill sweep".into())
}

/// Run `publish` with a cancellation flag raised after `delay`, which kills
/// its native processes.
pub fn cancelled_after<T>(delay: Duration, publish: impl FnOnce(&AtomicBool) -> T) -> T {
    let cancelled = AtomicBool::new(false);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(delay);
            cancelled.store(true, Ordering::SeqCst);
        });
        publish(&cancelled)
    })
}

/// `complete` an interrupted publication; it must publish exactly `applied`.
pub fn finish(
    backend: &mut DoltBackend<'_>,
    releases: &Releases<'_, '_>,
    approval: &DoltWriteApproval,
    next: &Candidate,
    applied: &DoltReadPin,
) -> Result<(GitObjectId, DoltReadPin)> {
    match complete(backend, releases, approval, next, &STOP)? {
        Publication::Published { release, pin } if &pin == applied => {
            let seen = observe(backend, releases, approval, next, &STOP)?;
            if seen
                != (Observation::Published {
                    release: release.clone(),
                    pin: pin.clone(),
                })
            {
                return Err("a completed publication does not observe as published".into());
            }
            Ok((release, pin))
        }
        other => Err(format!("completion did not publish the applied commit: {other:?}").into()),
    }
}

/// No double publish and no split: one storage commit and one release per
/// published attempt, the release chain ends at the last one, and the storage
/// head is exactly the last published pin.
fn verify(
    world: &World<'_, '_, '_, '_>,
    tally: &Tally,
    release: &GitObjectId,
    pin: &DoltReadPin,
) -> Result {
    let published = tally.published as usize;
    // The bootstrap write, then one per published attempt.
    if world.realm.commits_mentioning("writfold-dolt-write/1")? != published + 1 {
        return Err("storage commits differ from published attempts".into());
    }
    let mut chain = 0;
    let mut cursor = Some(release.clone());
    while let Some(current) = cursor {
        if current == world.genesis {
            break;
        }
        chain += 1;
        cursor = world
            .releases
            .interpret::<DoltReadPin>(&current, &STOP)?
            .predecessor;
    }
    if chain != published || world.releases.completed(&STOP)?.as_ref() != Some(release) {
        return Err(format!("release chain {chain} differs from {published} publications").into());
    }
    let head = world
        .releases
        .interpret::<DoltReadPin>(release, &STOP)?
        .manifest
        .pin;
    if &head != pin || world.realm.head()? != pin.commit().as_str() {
        return Err("the release, the storage head and the last published pin differ".into());
    }
    Ok(())
}
