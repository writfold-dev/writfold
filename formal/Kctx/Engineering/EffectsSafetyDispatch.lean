import Kctx.Engineering.EffectsSafety
import Kctx.Engineering.ContinuationSafety

set_option autoImplicit false
set_option maxRecDepth 4096
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

structure DispatchChecked (contracts : Contracts) (s : State) (id : Id .effect) where
  effect : Effect
  dispatch : Dispatch
  run : Run
  grant : Grant
  node : Node
  selectedEffect : findEffect s id = some effect
  selectedDispatch : effect.dispatch = some dispatch
  selectedRun : findRun s effect.intent.invocation = some run
  selectedGrant : currentGrant s run = some grant
  selectedNode : findNode run.procedure effect.intent.node = some node
  claimed : effect.outcome = .claimed
  unissued : dispatch.issued = false
  incarnation : dispatch.incarnation = s.incarnation
  intent : dispatch.intent = effect.intent
  grantIdentity : dispatch.grant = grant.id
  policy : dispatch.policy = s.authority.policy
  generation : dispatch.generation = effect.generation
  ownershipAndAuthority : intentReady s run effect.intent dispatch.generation = true
  evidence : evidenceReady s run node = true
  nodeReady : Kctx.Engineering.nodeReady contracts s run node = true
  treesQuiet : ownedTreesQuiet s effect.intent dispatch.generation = true
  inventory : dispatch ∈ s.claims
  retryValid : dispatchRetryValid contracts s dispatch = true
  persisted : ∃ record ∈ Ledger.currentPrefix s.ledger run.stream,
    record.id = dispatch.record ∧
      record.payload = (if dispatch.retryAuthorization.isSome then
        .retryClaim id dispatch.id dispatch.expectedHead dispatch.record
      else .claimDispatch id dispatch.id dispatch.expectedHead dispatch.record)
  providerOrIdempotent : effect.intent.expectedProvider = s.world.provider ∨
    dispatch.retry = some .providerIdempotent

theorem E07_issue_checks_selected_claim {contracts : Contracts} {s t : State} {id : Id .effect}
    (h : issueEffect contracts s id = some t) : Nonempty (DispatchChecked contracts s id) := by
  cases effectFound : findEffect s id with
  | none => simp [issueEffect, effectFound] at h
  | some effect =>
      cases dispatchFound : effect.dispatch with
      | none => simp [issueEffect, effectFound, dispatchFound] at h
      | some dispatch =>
          cases runFound : findRun s effect.intent.invocation with
          | none => simp [issueEffect, effectFound, dispatchFound, runFound] at h
          | some run =>
              cases grantFound : currentGrant s run with
              | none => simp [issueEffect, effectFound, dispatchFound, runFound, grantFound] at h
              | some grant =>
                  cases nodeFound : findNode run.procedure effect.intent.node with
                  | none => simp [issueEffect, effectFound, dispatchFound, runFound, grantFound, nodeFound] at h
                  | some node =>
                      simp only [issueEffect, effectFound, dispatchFound, runFound, grantFound,
                        nodeFound, bind, Option.bind_some] at h
                      obtain ⟨guard, _⟩ := Option.ite_none_right_eq_some.mp h
                      have checked : effect.outcome = .claimed ∧ dispatch.issued = false ∧
                          dispatch.incarnation = s.incarnation ∧ dispatch.intent = effect.intent ∧
                          dispatch.grant = grant.id ∧ dispatch.policy = s.authority.policy ∧
                          dispatch.generation = effect.generation ∧
                          intentReady s run effect.intent dispatch.generation = true ∧
                          evidenceReady s run node = true ∧ nodeReady contracts s run node = true ∧
                          ownedTreesQuiet s effect.intent dispatch.generation = true ∧
                          dispatch ∈ s.claims ∧
                          dispatchRetryValid contracts s dispatch = true ∧
                          (∃ record ∈ Ledger.currentPrefix s.ledger run.stream,
                            record.id = dispatch.record ∧ record.payload =
                              (if dispatch.retryAuthorization.isSome then
                                .retryClaim id dispatch.id dispatch.expectedHead dispatch.record
                              else .claimDispatch id dispatch.id dispatch.expectedHead dispatch.record)) ∧
                          (effect.intent.expectedProvider = s.world.provider ∨
                            dispatch.retry = some .providerIdempotent) := by
                        simpa [Bool.and_eq_true, Bool.or_eq_true, Bool.not_eq_true', List.any_eq_true, and_assoc] using guard
                      rcases checked with ⟨claimed, unissued, incarnation, intent, grantIdentity,
                        policy, generation, authority, evidence, ready, quiet, inventory, retry, persisted, provider⟩
                      exact ⟨⟨effect, dispatch, run, grant, node, effectFound, dispatchFound,
                        runFound, grantFound, nodeFound, claimed, unissued, incarnation, intent,
                        grantIdentity, policy, generation, authority, evidence, ready, quiet,
                        inventory, retry, persisted, provider⟩⟩

theorem E07_dispatch_request_checks_selected_claim {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {id : Id .effect}
    (h : step contracts s (.dispatch stream expected id) = some t) : Nonempty (DispatchChecked contracts s id) := by
  simp only [step, dispatchSelected] at h
  split at h
  · cases body : domainBody contracts s (.issueEffect id) with
    | none => simp [domain, body] at h
    | some next => exact E07_issue_checks_selected_claim body
  · contradiction

/-- Ledger identity uniqueness upgrades the selected-record existence guard to
one exact persisted claim; no unselected orphan is a send authorization. -/
theorem E07_selected_claim_unique {contracts : Contracts} {s : State} {id : Id .effect}
    (checked : DispatchChecked contracts s id) (ledger : Ledger.WellFormed historicalValid s.ledger) :
    ∃ record ∈ Ledger.currentPrefix s.ledger checked.run.stream,
      record.id = checked.dispatch.record ∧
      record.payload = (if checked.dispatch.retryAuthorization.isSome then
        .retryClaim id checked.dispatch.id checked.dispatch.expectedHead checked.dispatch.record
      else .claimDispatch id checked.dispatch.id checked.dispatch.expectedHead checked.dispatch.record) ∧
      ∀ other ∈ Ledger.currentPrefix s.ledger checked.run.stream,
        other.id = checked.dispatch.record → other = record := by
  obtain ⟨record, selected, identity, payload⟩ := checked.persisted
  refine ⟨record, selected, identity, payload, ?_⟩
  intro other member same
  have retained := Ledger.currentPrefix_retained historicalValid s.ledger checked.run.stream ledger
  exact ledger.1 other (retained other member).1 record (retained record selected).1 (same.trans identity.symm)

theorem E06_issue_increments_owned_process_generation {contracts : Contracts} {s t : State} {id : Id .effect}
    (h : issueEffect contracts s id = some t) :
    ∃ effect dispatch, findEffect s id = some effect ∧ effect.dispatch = some dispatch ∧
      t.ownership = s.ownership.map (fun owner =>
        if effect.intent.resources.contains owner.resource && owner.owner == effect.intent.run &&
            owner.holder == effect.intent.invocation && owner.generation == dispatch.generation then
          { owner with children := owner.children + 1, processGeneration := owner.processGeneration + 1 }
        else owner) := by
  simp only [issueEffect, bind, Option.bind] at h
  repeat first
    | contradiction
    | (split at h <;> try dsimp only at h)
    | (cases h; simp_all)

theorem E06_terminate_requires_current_generations {contracts : Contracts} {s t : State}
    {resource : Id .resource} {generation processGeneration : Nat}
    (h : environment contracts s (.terminateProcess resource generation processGeneration) = some t) :
    (resource, generation, processGeneration) ∈ contracts.quiescent ∧
      ∃ owner ∈ s.ownership, owner.resource = resource ∧ owner.generation = generation ∧
        owner.processGeneration = processGeneration := by
  cases body : environmentBody contracts s (.terminateProcess resource generation processGeneration) with
  | none => simp [environment, body] at h
  | some next =>
      simp only [environmentBody] at body
      split at body
      next guard =>
        simpa [Bool.and_eq_true, List.any_eq_true, and_assoc] using guard
      next => contradiction

/-- This impossibility is about the actual environment transition, not merely
the shape of a certificate. Issuing a new tree invalidates every older number. -/
theorem E06_stale_process_certificate_refused {contracts : Contracts} {s : State}
    {resource : Id .resource} {generation oldProcessGeneration : Nat}
    (stale : ∀ owner ∈ s.ownership, owner.resource = resource → owner.generation = generation →
      owner.processGeneration ≠ oldProcessGeneration) :
    environment contracts s (.terminateProcess resource generation oldProcessGeneration) = none := by
  cases result : environment contracts s (.terminateProcess resource generation oldProcessGeneration) with
  | none => rfl
  | some next =>
      obtain ⟨_, owner, member, sameResource, sameGeneration, sameProcess⟩ :=
        E06_terminate_requires_current_generations result
      exact False.elim (stale owner member sameResource sameGeneration sameProcess)

/-- Neither observing external completion nor reconciling its classification
requires a live mutation grant. They cannot themselves send a pending operation. -/
theorem E07_reconcile_preserves_mutation_world {s t : State} {event : ExternalEvent}
    (h : reconcile s event = some t) : t.world = s.world ∧ t.ownership = s.ownership := by
  simp only [reconcile, bind, Option.bind] at h
  split at h <;> try dsimp only at h
  · contradiction
  · split at h <;> try dsimp only at h
    · cases h
      exact ⟨rfl, rfl⟩
    · contradiction

theorem E07_external_completion_independent_of_coordinator {contracts : Contracts} {s : State}
    {event : ExternalEvent} {producer consumer : Nat} (alive : Bool) :
    (environmentBody contracts { s with alive := alive } (.completeExternalEffect event producer consumer)).isSome =
      (environmentBody contracts s (.completeExternalEffect event producer consumer)).isSome := by
  cases claimFound : event.claim with
  | none =>
      simp only [environmentBody, completeExternalEffect, claimFound, bind,
        Option.bind_none, Option.isSome_none]
  | some claim =>
      cases pendingFound : s.world.pending.find? (fun pending =>
          pending.intent.effect == event.effect && pending.claim == claim) with
      | none =>
          simp only [environmentBody, completeExternalEffect, claimFound, pendingFound,
            bind, Option.bind_some, Option.bind_none, Option.isSome_none]
      | some pending =>
          simp only [environmentBody, completeExternalEffect, claimFound, pendingFound,
            bind, Option.bind_some]
          by_cases matching : (event.binding == pending.intent.binding &&
              event.expected == pending.intent.expectedProvider) = true
          · simp only [matching, ↓reduceIte]
            cases applicationFound : s.world.applications.find? (fun application =>
                application.intent.effect == event.effect) with
            | none =>
                by_cases rejected : (event.result == .rejected) = true
                · simp only [applicationFound, rejected, ↓reduceIte, Option.isSome_some]
                · simp only [applicationFound, rejected, Bool.false_eq_true, ↓reduceIte]
                  by_cases authorized : (contracts.effectAuthorizations.contains pending.intent &&
                      s.world.provider == pending.intent.expectedProvider &&
                      contracts.compatibility.contains (event.providerAfter, producer, consumer)) = true
                  · simp only [authorized, ↓reduceIte, Option.isSome_some]
                  · simp only [authorized, Bool.false_eq_true, ↓reduceIte, Option.isSome_none]
            | some application =>
                by_cases replayed : (application.intent == pending.intent && application.result != .rejected &&
                    event.result == application.result && event.providerAfter == application.providerAfter) = true
                · simp only [applicationFound, replayed, ↓reduceIte, Option.isSome_some]
                · simp only [applicationFound, replayed, Bool.false_eq_true, ↓reduceIte, Option.isSome_none]
          · simp only [matching, Bool.false_eq_true, ↓reduceIte, Option.isSome_none]

end Kctx.Engineering
