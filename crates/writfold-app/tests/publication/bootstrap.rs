//! Bootstrap publication: a verified signed bootstrap approval (no base, no
//! mapping) creates a realm's first release through the ordinary order, with
//! an absent completed ref as the Git expected-old state and the realm's
//! storage origin as the Dolt base. A second bootstrap, or one over storage
//! that already carries a write, is refused with nothing written.
use super::kill::{ATTEMPTS, STEP, Tally, cancelled_after, finish};
use super::world::{Fresh, Result, STOP, candidate, provision, reads, with_realm};
use writfold_app::publication::{Observation, Publication, PublicationError, observe, publish};
use writfold_domain::GitObjectId;
use writfold_domain::dolt::DoltReadPin;
use writfold_store::backend::{Origin, Refusal, StorageBackend, WriteOutcome};

const WRITES: &str = "writfold-dolt-write/1";

#[test]
fn bootstrap_creates_the_first_release_once() -> Result {
    with_realm(&provision()?, |fresh| {
        let first = candidate("doc:b", "first release")?;
        // An ordinary approval has no release to publish over.
        let elsewhere: GitObjectId = "f".repeat(64).parse()?;
        let ordinary = fresh.signer.approve(&elsewhere, &fresh.realm.after, 1)?;
        let early = publish(fresh.backend, fresh.releases, &ordinary, &first, &STOP)?;
        assert_eq!(early, Publication::Conflict { observed: None });
        let approval = fresh.signer.bootstrap(1)?;
        let Publication::Published { release, pin } =
            publish(fresh.backend, fresh.releases, &approval, &first, &STOP)?
        else {
            return Err("expected the bootstrap to publish".into());
        };
        let seen = observe(fresh.backend, fresh.releases, &approval, &first, &STOP)?;
        let expected = Observation::Published {
            release: release.clone(),
            pin: pin.clone(),
        };
        assert_eq!(seen, expected);
        verify(fresh, &release, &pin, "doc:b", b"first release")?;
        Ok(())
    })
}

#[test]
fn bootstrap_over_written_storage_writes_nothing() -> Result {
    with_realm(&provision()?, |fresh| {
        // A direct B0 write moves the Dolt head before any release exists.
        let elsewhere: GitObjectId = "f".repeat(64).parse()?;
        let grant = fresh.signer.approve(&elsewhere, &fresh.realm.after, 1)?;
        let direct = candidate("doc:d", "storage only")?;
        let request = direct.request(&fresh.realm.after)?;
        let WriteOutcome::Committed { new } =
            fresh.backend.admit_write(&request, Some(&grant), &STOP)?
        else {
            return Err("expected the direct write to commit".into());
        };
        let approval = fresh.signer.bootstrap(1)?;
        let next = candidate("doc:b", "over a write")?;
        let refused = publish(fresh.backend, fresh.releases, &approval, &next, &STOP);
        assert!(
            matches!(refused, Err(PublicationError::Initialized)),
            "{refused:?}"
        );
        let seen = observe(fresh.backend, fresh.releases, &approval, &next, &STOP)?;
        assert_eq!(seen, Observation::NotPublished);
        // The writer itself refuses it under its native lease, even from the
        // current head.
        let raw = fresh
            .backend
            .admit_write(&next.request(&new)?, Some(&approval), &STOP);
        assert!(matches!(raw, Err(Refusal::Unauthorized(_))), "{raw:?}");
        assert_eq!(fresh.releases.completed(&STOP)?, None);
        assert_eq!(fresh.realm.commits_mentioning(WRITES)?, 1);
        assert_eq!(fresh.realm.head()?, new.commit().as_str());
        Ok(())
    })
}

#[test]
fn interrupted_bootstraps_resolve_without_double_publish() -> Result {
    let template = provision()?;
    let mut tally = Tally::default();
    while tally.attempts < ATTEMPTS {
        let realm = template.copy()?;
        let mut direct = false;
        with_realm(&realm, |fresh| {
            direct = until_published(fresh, &mut tally)?;
            Ok(())
        })?;
        if direct {
            eprintln!("bootstrap kill sweep: {tally:?}");
            assert!(tally.unknown_not_published + tally.unknown_completed > 0);
            return Ok(());
        }
    }
    Err("no bootstrap completed uncancelled within the kill sweep".into())
}

/// Bootstraps cancelled after 0, 20, 40, ... ms in one fresh realm until one
/// publishes, directly or by `complete`. Returns whether it was uncancelled.
fn until_published(fresh: &mut Fresh<'_, '_, '_, '_>, tally: &mut Tally) -> Result<bool> {
    while tally.attempts < ATTEMPTS {
        let attempt = tally.attempts;
        tally.attempts += 1;
        let content = format!("bootstrap attempt {attempt}");
        let next = candidate("doc:boot", &content)?;
        let approval = fresh.signer.bootstrap(attempt + 1)?;
        let outcome = cancelled_after(STEP * attempt, |cancelled| {
            publish(fresh.backend, fresh.releases, &approval, &next, cancelled)
        });
        let seen = observe(fresh.backend, fresh.releases, &approval, &next, &STOP)?;
        let direct = matches!(outcome, Ok(Publication::Published { .. }));
        let published = match (outcome, seen) {
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
                rival_refused(fresh, attempt)?;
                Some(finish(
                    fresh.backend,
                    fresh.releases,
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
        if let Some((release, pin)) = published {
            tally.published += 1;
            verify(fresh, &release, &pin, "doc:boot", content.as_bytes())?;
            return Ok(direct);
        }
    }
    Err("no bootstrap completed within the kill sweep".into())
}

/// While a bootstrap is interrupted, a different bootstrap is `Initialized`
/// and writes nothing: no Dolt commit, no release.
fn rival_refused(fresh: &mut Fresh<'_, '_, '_, '_>, attempt: u32) -> Result {
    let rival = fresh.signer.bootstrap(ATTEMPTS + attempt + 1)?;
    let other = candidate("doc:boot", "rival bootstrap")?;
    let head = fresh.realm.head()?;
    let refused = publish(fresh.backend, fresh.releases, &rival, &other, &STOP);
    if !matches!(refused, Err(PublicationError::Initialized)) {
        return Err(format!("attempt {attempt}: rival bootstrap was {refused:?}").into());
    }
    if fresh.realm.head()? != head || fresh.releases.completed(&STOP)?.is_some() {
        return Err(format!("attempt {attempt}: a refused bootstrap wrote").into());
    }
    Ok(())
}

/// The realm's only release: no predecessor, complete retained closure, one
/// storage write from the origin, the Dolt head exactly its pin. A second
/// bootstrap is a `Conflict` and writes nothing.
fn verify(
    fresh: &mut Fresh<'_, '_, '_, '_>,
    release: &GitObjectId,
    pin: &DoltReadPin,
    key: &str,
    content: &[u8],
) -> Result {
    let released = fresh.releases.interpret::<DoltReadPin>(release, &STOP)?;
    let retained = format!("wf-retained/{}", pin.commit().as_str());
    let origin = Origin {
        pin: fresh.realm.after.clone(),
        written: true,
    };
    let closure = released.predecessor.is_none()
        && &released.manifest.pin == pin
        && fresh.realm.branches()?.contains(&retained)
        && reads(fresh.backend, pin, key)? == content
        && fresh.backend.origin(&STOP)? == origin;
    if !closure {
        return Err(format!("bootstrap release {release:?} is not exactly {pin:?}").into());
    }
    let again = fresh.signer.bootstrap(2 * ATTEMPTS + 1)?;
    let second = candidate(key, "second bootstrap")?;
    let replay = publish(fresh.backend, fresh.releases, &again, &second, &STOP)?;
    let seen = observe(fresh.backend, fresh.releases, &again, &second, &STOP)?;
    let refused = replay
        == (Publication::Conflict {
            observed: Some(release.clone()),
        })
        && seen == Observation::NotPublished;
    let single = fresh.releases.completed(&STOP)?.as_ref() == Some(release)
        && fresh.realm.commits_mentioning(WRITES)? == 1
        && fresh.realm.head()? == pin.commit().as_str();
    if !refused || !single {
        return Err(format!("second bootstrap: {replay:?}, {seen:?}; single {single}").into());
    }
    Ok(())
}
