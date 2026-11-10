//! DINT02 first slice end to end: approved publication of a prepared candidate
//! over the real Dolt conditional writer and a real Git release repository.
//!
//! The admitted Dolt executable (path and SHA-256 from
//! `qualification/dolt-integration-profile.json`) and the admitted Git must be
//! present, as they are in the qualified tool image. A different or missing
//! Dolt fails these tests; they are never skipped.
#[path = "publication/kill.rs"]
mod kill;
#[path = "publication/realm.rs"]
mod realm;
#[path = "publication/signer.rs"]
mod signer;
#[path = "publication/world.rs"]
mod world;

use world::{Result, STOP, candidate, reads, with_world};
use writfold_app::publication::{Observation, Publication, PublicationError, observe, publish};
use writfold_domain::dolt::DoltReadPin;
use writfold_store::backend::{StorageBackend, WriteOutcome};

#[test]
fn publishes_with_retained_closure_and_preserved_history() -> Result {
    with_world(|world| {
        let first = candidate("doc:c", "first publication")?;
        let approval = world
            .signer
            .approve(&world.genesis, &world.realm.after, 1)?;
        let Publication::Published { release, pin } =
            publish(world.backend, world.releases, &approval, &first, &STOP)?
        else {
            return Err("expected Published".into());
        };
        let seen = observe(world.backend, world.releases, &approval, &first, &STOP)?;
        assert_eq!(
            seen,
            Observation::Published {
                release: release.clone(),
                pin: pin.clone()
            }
        );
        // Complete retained closure: manifest, candidate blob and Dolt root.
        let released = world.releases.interpret::<DoltReadPin>(&release, &STOP)?;
        assert_eq!(released.predecessor.as_ref(), Some(&world.genesis));
        assert_eq!(&released.manifest.candidate, first.digest());
        assert_eq!(released.manifest.pin, pin);
        assert!(
            world
                .realm
                .branches()?
                .contains(&format!("wf-retained/{}", pin.commit().as_str()))
        );
        // A second publication; the first release still reads its own state.
        let second = candidate("doc:c", "second publication")?;
        let next = world.signer.approve(&release, &pin, 2)?;
        let Publication::Published {
            release: later,
            pin: newer,
        } = publish(world.backend, world.releases, &next, &second, &STOP)?
        else {
            return Err("expected the second publication".into());
        };
        let old = world.releases.interpret::<DoltReadPin>(&release, &STOP)?;
        assert_eq!(old.manifest.pin, pin);
        assert_eq!(
            reads(world, &old.manifest.pin, "doc:c")?,
            b"first publication"
        );
        assert_eq!(reads(world, &newer, "doc:c")?, b"second publication");
        assert_eq!(world.releases.completed(&STOP)?, Some(later));
        // The first approval is spent: its base release is no longer current.
        let replay = publish(world.backend, world.releases, &approval, &first, &STOP)?;
        assert!(matches!(replay, Publication::Conflict { .. }), "{replay:?}");
        assert_eq!(world.realm.commits_mentioning("writfold-dolt-write/1")?, 2);
        Ok(())
    })
}

#[test]
fn git_conflict_and_misinterpretation_write_nothing() -> Result {
    with_world(|world| {
        let winner = candidate("doc:c", "winner")?;
        let approval = world
            .signer
            .approve(&world.genesis, &world.realm.after, 1)?;
        let won = publish(world.backend, world.releases, &approval, &winner, &STOP)?;
        let Publication::Published { release, pin } = won else {
            return Err("expected Published".into());
        };
        // A different signed operation over the same base: Git CAS conflict.
        let loser = candidate("doc:c", "loser")?;
        let rival = world
            .signer
            .approve(&world.genesis, &world.realm.after, 2)?;
        let lost = publish(world.backend, world.releases, &rival, &loser, &STOP)?;
        assert_eq!(
            lost,
            Publication::Conflict {
                observed: Some(release.clone())
            }
        );
        assert_eq!(
            observe(world.backend, world.releases, &rival, &loser, &STOP)?,
            Observation::NotPublished
        );
        // An approval whose base pin is not what the base release names.
        let wrong = world.signer.approve(&release, &world.realm.before, 3)?;
        let refused = publish(world.backend, world.releases, &wrong, &loser, &STOP);
        assert!(
            matches!(refused, Err(PublicationError::Misinterpreted)),
            "{refused:?}"
        );
        assert_eq!(world.releases.completed(&STOP)?, Some(release));
        assert_eq!(reads(world, &pin, "doc:c")?, b"winner");
        assert_eq!(world.realm.commits_mentioning("writfold-dolt-write/1")?, 1);
        Ok(())
    })
}

#[test]
fn a_moved_storage_head_is_stale_and_never_published() -> Result {
    with_world(|world| {
        // A B0 storage write outside publication moves the Dolt head only.
        let direct = candidate("doc:d", "storage only")?;
        let grant = world
            .signer
            .approve(&world.genesis, &world.realm.after, 1)?;
        let moved =
            world
                .backend
                .admit_write(&direct.request(&world.realm.after)?, Some(&grant), &STOP)?;
        let WriteOutcome::Committed { new } = moved else {
            return Err("expected the direct write to commit".into());
        };
        let next = candidate("doc:c", "after a moved head")?;
        let approval = world
            .signer
            .approve(&world.genesis, &world.realm.after, 2)?;
        let stale = publish(world.backend, world.releases, &approval, &next, &STOP)?;
        assert_eq!(stale, Publication::Stale { observed: new });
        assert_eq!(
            observe(world.backend, world.releases, &approval, &next, &STOP)?,
            Observation::NotPublished
        );
        assert_eq!(
            world.releases.completed(&STOP)?,
            Some(world.genesis.clone())
        );
        Ok(())
    })
}

#[test]
fn interrupted_publications_resolve_without_double_publish() -> Result {
    with_world(|world| {
        let tally = kill::sweep(world)?;
        eprintln!("publication kill sweep: {tally:?}");
        assert!(
            tally.unknown_not_published + tally.unknown_completed + tally.unknown_published > 0
        );
        Ok(())
    })
}
