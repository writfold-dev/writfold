//! Writer races: two writers from one head (exactly one wins), and writes killed
//! at increasing delays (every `Unknown` must be resolvable by `observe`).
use super::writes::{authorize, observe, open, reads, request};
use super::{ConformanceFailure, WriterFixture, fail, not_cancelled, require};
use crate::backend::{Refusal, StorageBackend, WriteObservation, WriteOutcome, WriteRequest};
use std::sync::Barrier;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Kill delays step by this much until one write completes uncancelled.
const KILL_STEP: Duration = Duration::from_millis(10);
const KILL_ATTEMPTS: u32 = 400;

type Attempt<P> = Result<WriteOutcome<P>, Refusal>;

/// Two writers race from `head`; exactly one commits. The other is refused the
/// realm (`NotOwner`) or finds the winner's head (`Stale`). Returns the winner.
pub(super) fn concurrent<B>(
    writes: &WriterFixture<'_, B>,
    head: &B::Pin,
) -> Result<B::Pin, ConformanceFailure>
where
    B: StorageBackend + Send,
    B::Pin: Send + Sync,
{
    const CHECK: &str = "write-concurrent";
    let racers = [
        request(CHECK, head, "doc:race", "writer one")?,
        request(CHECK, head, "doc:race", "writer two")?,
    ];
    let barrier = Barrier::new(racers.len());
    let outcomes: Vec<Attempt<B::Pin>> = std::thread::scope(|scope| {
        let handles: Vec<_> = racers
            .iter()
            .map(|write| scope.spawn(|| race(writes, write, &barrier)))
            .collect();
        handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .unwrap_or_else(|_| Err(Refusal::Backend("racer panicked".into())))
            })
            .collect()
    });
    let winners: Vec<(usize, &B::Pin)> = outcomes
        .iter()
        .enumerate()
        .filter_map(|(index, outcome)| match outcome {
            Ok(WriteOutcome::Committed { new }) => Some((index, new)),
            _ => None,
        })
        .collect();
    let [(winner, new)] = winners.as_slice() else {
        return Err(fail(
            CHECK,
            format!("expected one winner, got {outcomes:?}"),
        ));
    };
    let loser = 1 - winner;
    require(
        CHECK,
        match &outcomes[loser] {
            Err(Refusal::NotOwner(_)) => true,
            Ok(WriteOutcome::Stale { observed }) => observed == *new,
            _ => false,
        },
        "the losing writer was neither refused the realm nor stale",
    )?;
    let backend = open(CHECK, writes)?;
    let won = observe(CHECK, &backend, &racers[*winner])?;
    let lost = observe(CHECK, &backend, &racers[loser])?;
    require(
        CHECK,
        won.head == **new && won.applied.as_ref() == Some(*new) && lost.applied.is_none(),
        "the head is not exactly the winner's commit",
    )?;
    let content = if *winner == 0 {
        "writer one"
    } else {
        "writer two"
    };
    reads(CHECK, &backend, new, "doc:race", content)?;
    Ok((*new).clone())
}

fn race<B: StorageBackend>(
    writes: &WriterFixture<'_, B>,
    write: &WriteRequest<B::Pin>,
    barrier: &Barrier,
) -> Attempt<B::Pin> {
    let grant = (writes.authorize)(write);
    barrier.wait();
    let mut backend = (writes.open)()?;
    backend.admit_write(write, Some(&grant?), &not_cancelled())
}

/// Writes are killed (cancelled) after 0, 10, 20, ... ms until one completes.
/// Every outcome must agree with `observe`; at least one must be `Unknown`.
pub(super) fn kill<B>(
    backend: &mut B,
    writes: &WriterFixture<'_, B>,
    head: &B::Pin,
) -> Result<KillTally, ConformanceFailure>
where
    B: StorageBackend,
{
    const CHECK: &str = "write-kill-reconcile";
    let mut head = head.clone();
    let mut tally = KillTally::default();
    for attempt in 0..KILL_ATTEMPTS {
        tally.attempts += 1;
        let content = format!("kill attempt {attempt}");
        let write = request(CHECK, &head, "doc:kill", &content)?;
        let grant = authorize(CHECK, writes, &write)?;
        let cancelled = AtomicBool::new(false);
        let outcome = std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(KILL_STEP * attempt);
                cancelled.store(true, Ordering::SeqCst);
            });
            backend.admit_write(&write, Some(&grant), &cancelled)
        });
        let seen = observe(CHECK, backend, &write)?;
        if !tally.consistent(&outcome, &seen, &head) {
            return Err(fail(
                CHECK,
                format!("attempt {attempt}: outcome {outcome:?} disagrees with {seen:?}"),
            ));
        }
        if let Ok(WriteOutcome::Committed { new }) = outcome {
            reads(CHECK, backend, &new, "doc:kill", &content)?;
            require(
                CHECK,
                tally.unknown_applied + tally.unknown_absent > 0,
                "no attempt ended Unknown; the kill window was not exercised",
            )?;
            return Ok(tally);
        }
        head = seen.head;
    }
    Err(fail(CHECK, "no write completed within the kill sweep"))
}

/// How the kill sweep's attempts ended and how `observe` resolved them.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KillTally {
    pub attempts: u32,
    /// Refused as cancelled or past deadline; nothing was written.
    pub refused: u32,
    /// `Unknown`, and `observe` found the write's commit at the head.
    pub unknown_applied: u32,
    /// `Unknown`, and `observe` found the head unchanged without the write.
    pub unknown_absent: u32,
}

impl KillTally {
    fn consistent<P: Eq>(
        &mut self,
        outcome: &Attempt<P>,
        seen: &WriteObservation<P>,
        head: &P,
    ) -> bool {
        match outcome {
            Ok(WriteOutcome::Committed { new }) => {
                seen.head == *new && seen.applied.as_ref() == Some(new)
            }
            Err(Refusal::Cancelled | Refusal::Deadline) => {
                self.refused += 1;
                seen.head == *head && seen.applied.is_none()
            }
            Ok(WriteOutcome::Unknown) => match &seen.applied {
                Some(applied) => {
                    self.unknown_applied += 1;
                    seen.head == *applied
                }
                None => {
                    self.unknown_absent += 1;
                    seen.head == *head
                }
            },
            _ => false,
        }
    }
}
