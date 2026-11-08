import Kctx.Engineering.ActionDomain
import Kctx.Engineering.WorldEffects

set_option autoImplicit false
namespace Kctx.Engineering

inductive Environment where
  | advanceTime (time : Nat)
  | installGrant (authorization : GrantAuthorization)
  | revokeGrant (grant : Id .grant)
  | changePolicy (policy : Id .policy) (floor : List Gate)
  | withdrawProcedure (revision : ProcedureRevision)
  | correctKnowledge
  | advanceUpstream (revision : ProviderRevision)
  | changeProviderState (revision : ProviderRevision) (producer consumer : Nat)
  | completeExternalEffect (event : ExternalEvent) (producer consumer : Nat)
  | loseReply (effect : Id .effect)
  | terminateProcess (resource : Id .resource) (generation processGeneration : Nat)
  deriving DecidableEq, Repr, Inhabited

/-- External completion is independent of the coordinator's lifetime. The exact
conditional-effect and authorization-at-effect contract remains a qualified
boundary; arbitrary outside provider changes need not preserve compatibility. -/
def environmentBody (contracts : Contracts) (s : State) : Environment → Option State
  | .advanceTime time => if s.now ≤ time then some { s with now := time } else none
  | .installGrant authorization =>
      if contracts.grantAuthorizations.contains authorization &&
          authorization.realm == s.realm && authorization.grant.binding.realm == s.realm &&
          authorization.policy == s.authority.policy &&
          s.retained.contains authorization.envelope &&
          !(s.authority.grants.map Grant.id).contains authorization.grant.id &&
          !authorization.grant.revoked &&
          authorization.grant.validFrom ≤ s.now && s.now < authorization.grant.expires then
        some { s with authority := { s.authority with
          grants := authorization.grant :: s.authority.grants } }
      else none
  | .revokeGrant id => some { s with authority := { s.authority with
      grants := s.authority.grants.map (fun grant =>
        if grant.id == id then { grant with revoked := true } else grant) } }
  | .changePolicy id floor =>
      if subset contracts.kernelFloor floor then
        some { s with authority := { s.authority with policy := id, floor := floor } }
      else none
  | .withdrawProcedure revision => some { s with authority := { s.authority with
      withdrawn := revision :: s.authority.withdrawn } }
  | .correctKnowledge => some { s with authority := { s.authority with
      correctionGeneration := s.authority.correctionGeneration + 1 } }
  | .advanceUpstream revision => some { s with world := { s.world with provider := revision } }
  | .changeProviderState revision producer consumer => some { s with world := { s.world with
      provider := revision, producer := producer, consumer := consumer } }
  | .completeExternalEffect event producer consumer =>
      completeExternalEffect contracts s event producer consumer
  | .loseReply effect => do
      let current ← findEffect s effect
      let dispatch ← current.dispatch
      if dispatch.issued then
        let next := if current.outcome == .claimed || current.outcome == .unknown then
          putEffect s { current with outcome := .unknown } else s
        some { next with world := { s.world with
          replies := s.world.replies.filter (·.effect != effect) } }
      else none
  | .terminateProcess resource generation processGeneration =>
      if contracts.quiescent.contains (resource, generation, processGeneration) &&
          s.ownership.any (fun owner => owner.resource == resource && owner.generation == generation &&
            owner.processGeneration == processGeneration) then
        some { s with ownership := s.ownership.map (fun owner =>
          if owner.resource == resource && owner.generation == generation &&
              owner.processGeneration == processGeneration then
            { owner with children := 0 } else owner) }
      else none

/-- Observations can invalidate continuation, but cannot forge a corpus release,
a selected ledger prefix or a reader cut. -/
def environment (contracts : Contracts) (s : State) (action : Environment) : Option State := do
  let next ← environmentBody contracts s action
  some { next with
    realm := s.realm
    controlStream := s.controlStream
    ledger := s.ledger
    cuts := s.cuts
    knowledge := s.knowledge
    knowledgeJournal := s.knowledgeJournal }

end Kctx.Engineering
