//! Publications cancelled (their native processes killed) after 0, 20, 40, ...
//! ms until one completes uncancelled. Each attempt is its own signed operation
//! and candidate. Every outcome is resolved by `observe` alone; an `Interrupted`
//! publication is finished by `complete`, which issues only Git steps. At the
//! end every attempt is published exactly once or not at all.
use super::world::{Result, STOP, World, candidate, reads};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use writfold_app::publication::{Observation, Publication, complete, observe, publish};
use writfold_domain::GitObjectId;
use writfold_domain::dolt::DoltReadPin;

const STEP: Duration = Duration::from_millis(20);
const ATTEMPTS: u32 = 400;

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
    let mut pin = world.realm.after.clone();
    for attempt in 0..ATTEMPTS {
        tally.attempts += 1;
        let content = format!("kill attempt {attempt}");
        let next = candidate("doc:kill", &content)?;
        let approval = world.signer.approve(&release, &pin, attempt + 1)?;
        let cancelled = AtomicBool::new(false);
        let outcome = std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(STEP * attempt);
                cancelled.store(true, Ordering::SeqCst);
            });
            publish(world.backend, world.releases, &approval, &next, &cancelled)
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
                Some(finish(world, &approval, &next, &applied)?)
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
            if reads(world, &new_pin, "doc:kill")? != content.as_bytes() {
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

/// `complete` an interrupted publication; it must publish exactly `applied`.
fn finish(
    world: &mut World<'_, '_, '_, '_>,
    approval: &writfold_store::backend::dolt::DoltWriteApproval,
    next: &writfold_app::publication::Candidate,
    applied: &DoltReadPin,
) -> Result<(GitObjectId, DoltReadPin)> {
    match complete(world.backend, world.releases, approval, next, &STOP)? {
        Publication::Published { release, pin } if &pin == applied => {
            let seen = observe(world.backend, world.releases, approval, next, &STOP)?;
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
    if world.realm.commits_mentioning("writfold-dolt-write/1")? != published {
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
