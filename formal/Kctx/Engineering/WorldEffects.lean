import Kctx.Engineering.Core

set_option autoImplicit false
namespace Kctx.Engineering

/-- All currently pending deliveries of one logical operation settle together.
A later explicitly authorized retry has a fresh claim and is not covered by an
older rejected observation. -/
def responseWorld (world : World) (event : ExternalEvent) : World :=
  { world with
    pending := world.pending.filter (fun pending => pending.intent.effect != event.effect)
    events := event :: world.events
    replies := event :: world.replies }

/-- Qualified idempotency means one non-rejected logical application. A duplicate
response returns that exact original result without changing current provider
state, even if unrelated external changes have occurred since it applied.
Rejections remain attempt-scoped, not an eternal prohibition on a qualified retry. -/
def completeExternalEffect (contracts : Contracts) (s : State) (event : ExternalEvent)
    (producer consumer : Nat) : Option State := do
  let claim ← event.claim
  let pending ← s.world.pending.find? (fun pending =>
    pending.intent.effect == event.effect && pending.claim == claim)
  let intent := pending.intent
  if event.binding == intent.binding && event.expected == intent.expectedProvider then
    match s.world.applications.find? (fun applied => applied.intent.effect == event.effect) with
    | some applied =>
        if applied.intent == intent && applied.result != .rejected &&
            event.result == applied.result && event.providerAfter == applied.providerAfter then
          some { s with world := responseWorld s.world event }
        else none
    | none =>
        if event.result == .rejected then
          some { s with world := responseWorld s.world event }
        else if contracts.effectAuthorizations.contains intent &&
            s.world.provider == intent.expectedProvider &&
            contracts.compatibility.contains (event.providerAfter, producer, consumer) then
          some { s with world := { responseWorld s.world event with
            provider := event.providerAfter
            producer := producer
            consumer := consumer
            applications := ⟨intent, event.result, event.providerAfter⟩ :: s.world.applications } }
        else none
  else none

/-- A reply can be lost while the immutable application result remains observable
through a separately qualified provider query. No retrieval renews its identity. -/
def canonicalObservation (s : State) (intent : Intent) (event : ExternalEvent) : Bool :=
  s.world.applications.any (fun applied =>
    applied.intent == intent && applied.intent.effect == event.effect &&
    applied.intent.binding == event.binding && applied.intent.expectedProvider == event.expected &&
    applied.result == event.result && applied.providerAfter == event.providerAfter &&
    applied.result != .rejected)

end Kctx.Engineering
