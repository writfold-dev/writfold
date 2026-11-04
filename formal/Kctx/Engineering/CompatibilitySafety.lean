import Kctx.Engineering.EffectsSafetyStep

set_option autoImplicit false
set_option maxRecDepth 4096
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

/-- Compatibility is about the actual completed revision and both repository
states. It is not inferred from a previous provider head or an exception. -/
def CompatibleState (compatible : ProviderRevision → Nat → Nat → Prop)
    (s : OperationalState) : Prop := compatible s.world.provider s.world.producer s.world.consumer

/-- Rejection and replay of an earlier canonical application leave the current
provider vector alone. Only a first logical application changes it, under the
actual-result compatibility observation and the expected-provider CAS. -/
theorem E11_completion_uses_actual_result {contracts : Contracts} {s t : State}
    {event : ExternalEvent} {producer consumer : Nat}
    (executed : environmentBody contracts s (.completeExternalEffect event producer consumer) = some t) :
    (t.world.provider = s.world.provider ∧ t.world.producer = s.world.producer ∧
      t.world.consumer = s.world.consumer) ∨
    ∃ pending ∈ s.world.pending, event.binding = pending.intent.binding ∧
      event.expected = pending.intent.expectedProvider ∧ pending.intent ∈ contracts.effectAuthorizations ∧
      s.world.provider = pending.intent.expectedProvider ∧
      (t.world.provider, t.world.producer, t.world.consumer) ∈ contracts.compatibility := by
  simp only [environmentBody, completeExternalEffect, responseWorld, bind, Option.bind] at executed
  repeat first
    | contradiction
    | (split at executed <;> try dsimp only at executed)
    | (cases executed; exact Or.inl ⟨rfl, rfl, rfl⟩)
    | (cases executed
       right
       refine ⟨_, List.mem_of_find?_eq_some (by assumption), ?_⟩
       simp_all [Bool.and_eq_true, Bool.or_eq_true])

theorem E11_completion_body_preserves {contracts : Contracts}
    {truth : ExternalEvent → Prop} {compatible : ProviderRevision → Nat → Nat → Prop}
    (sound : ExternalSound contracts truth compatible) {s t : State}
    (before : CompatibleState compatible s.toOperationalState)
    {event : ExternalEvent} {producer consumer : Nat}
    (h : environmentBody contracts s (.completeExternalEffect event producer consumer) = some t) :
    CompatibleState compatible t.toOperationalState := by
  rcases E11_completion_uses_actual_result h with frame | qualified
  · simpa only [CompatibleState, frame.1, frame.2.1, frame.2.2] using before
  · obtain ⟨pending, _, _, _, _, _, compatibleResult⟩ := qualified
    exact sound.compatibility _ _ _ compatibleResult

theorem E11_completion_preserves {contracts : Contracts}
    {truth : ExternalEvent → Prop} {compatible : ProviderRevision → Nat → Nat → Prop}
    (sound : ExternalSound contracts truth compatible) {s t : State}
    (before : CompatibleState compatible s.toOperationalState)
    {event : ExternalEvent} {producer consumer : Nat}
    (h : environment contracts s (.completeExternalEffect event producer consumer) = some t) :
    CompatibleState compatible t.toOperationalState := by
  cases body : environmentBody contracts s (.completeExternalEffect event producer consumer) with
  | none => simp [environment, body] at h
  | some next =>
      have preserved := E11_completion_body_preserves sound before body
      simp only [environment, body, bind, Option.bind, Option.some.injEq] at h
      subst t
      exact preserved

private theorem domain_provider_frame {contracts : Contracts} {s t : State} {action : Action}
    (h : domain contracts s action = some t) :
    t.world.provider = s.world.provider ∧ t.world.producer = s.world.producer ∧
      t.world.consumer = s.world.consumer := by
  cases body : domainBody contracts s action with
  | none => simp [domain, body] at h
  | some next =>
      have frame : next.world.provider = s.world.provider ∧ next.world.producer = s.world.producer ∧
          next.world.consumer = s.world.consumer := by
        cases action <;> simp only [domainBody, resolve, activate, loadResource, advanceNode,
          failNode, selectRoute, call, prepare, reserve, claimDispatch, retryClaim, issueEffect, observeEffect,
          observeMaterial, reconcile, reuse, returnChild, rebind, resume, reopen, cancel, quiesce,
          propose, evaluate, review, prepareAdmission, publishAdmission, acknowledgeAdmission,
          retire, putRun, putEffect] at body
        all_goals operational_success body
        all_goals exact ⟨rfl, rfl, rfl⟩
      simp only [domain, body, Option.map_some, Option.some.injEq] at h
      subst t
      exact frame

private theorem local_provider_frame {contracts : Contracts} {s t : State} {action : Action}
    (h : localAction contracts s action = some t) :
    t.world.provider = s.world.provider ∧ t.world.producer = s.world.producer ∧
      t.world.consumer = s.world.consumer := by
  cases action <;> simp only [localAction, project, acquireCut, releaseReadCut, crash,
    recover] at h
  all_goals operational_success h
  all_goals exact ⟨rfl, rfl, rfl⟩

theorem E11_step_preserves_compatibility {contracts : Contracts}
    {compatible : ProviderRevision → Nat → Nat → Prop} {s t : State} {request : Request}
    (before : CompatibleState compatible s.toOperationalState)
    (h : step contracts s request = some t) : CompatibleState compatible t.toOperationalState := by
  apply step_preserves_operational_property (P := CompatibleState compatible) ?_ ?_ before h
  · intro s t action compatibleNow executed
    obtain ⟨provider, producer, consumer⟩ := domain_provider_frame executed
    simpa only [CompatibleState, provider, producer, consumer] using compatibleNow
  · intro s t action compatibleNow executed
    obtain ⟨provider, producer, consumer⟩ := local_provider_frame executed
    simpa only [CompatibleState, provider, producer, consumer] using compatibleNow

/-- These are precisely the unconstrained outside writers. They are modeled,
not claimed to be fenced by a local interpreter. -/
def Environment.uncontrolledProviderChange : Environment → Bool
  | .advanceUpstream _ | .changeProviderState _ _ _ => true
  | _ => false

theorem E11_environment_preserves_or_external_change {contracts : Contracts}
    {truth : ExternalEvent → Prop} {compatible : ProviderRevision → Nat → Nat → Prop}
    (sound : ExternalSound contracts truth compatible) {s t : State} {action : Environment}
    (before : CompatibleState compatible s.toOperationalState)
    (h : environment contracts s action = some t) :
    CompatibleState compatible t.toOperationalState ∨ action.uncontrolledProviderChange = true := by
  cases body : environmentBody contracts s action with
  | none => simp [environment, body] at h
  | some next =>
      have checked : CompatibleState compatible next.toOperationalState ∨
          action.uncontrolledProviderChange = true := by
        cases action with
        | advanceUpstream revision => exact Or.inr rfl
        | changeProviderState revision producer consumer => exact Or.inr rfl
        | completeExternalEffect event producer consumer =>
            exact Or.inl (E11_completion_body_preserves sound before body)
        | _ =>
            left
            simp only [environmentBody, putEffect] at body
            operational_success body
            all_goals exact before
      simp only [environment, body, bind, Option.bind, Option.some.injEq] at h
      subst t
      exact checked

/-- Actual Transition consequences identify the only possible compatibility
breakers. No premise assumes preservation of an arbitrary external update. -/
theorem E11_transition_preserves_or_external_change {contracts : Contracts}
    {truth : ExternalEvent → Prop} {compatible : ProviderRevision → Nat → Nat → Prop}
    (sound : ExternalSound contracts truth compatible) {s t : State}
    (before : CompatibleState compatible s.toOperationalState) (h : Transition contracts s t) :
    CompatibleState compatible t.toOperationalState ∨
      ∃ action, action.uncontrolledProviderChange = true ∧ environment contracts s action = some t := by
  cases h with
  | operational executed => exact Or.inl (E11_step_preserves_compatibility before executed)
  | external executed =>
      rcases E11_environment_preserves_or_external_change sound before executed with compatibleNow | outside
      · exact Or.inl compatibleNow
      · exact Or.inr ⟨_, outside, executed⟩
  | refused => exact Or.inl before

/-- A permitted execution is an actual interpreter execution with no arbitrary
outside provider writer. Rejections, crashes and policy changes are unrestricted. -/
inductive CompatibleExecution (contracts : Contracts) (initial : State) : State → Prop where
  | initial : CompatibleExecution contracts initial initial
  | operational {s t : State} {request : Request} : CompatibleExecution contracts initial s →
      step contracts s request = some t → CompatibleExecution contracts initial t
  | external {s t : State} {action : Environment} : CompatibleExecution contracts initial s →
      action.uncontrolledProviderChange = false → environment contracts s action = some t →
      CompatibleExecution contracts initial t
  | refused {s : State} : CompatibleExecution contracts initial s → CompatibleExecution contracts initial s

theorem compatible_execution_reachable {contracts : Contracts} {initial s : State}
    (execution : CompatibleExecution contracts initial s) : Reachable contracts initial s := by
  induction execution with
  | initial => exact Reachable.initial
  | operational _ executed ih => exact Reachable.next ih (.operational executed)
  | external _ _ executed ih => exact Reachable.next ih (.external executed)
  | refused _ ih => exact ih

theorem E11_permitted_execution_compatible {contracts : Contracts}
    {truth : ExternalEvent → Prop} {compatible : ProviderRevision → Nat → Nat → Prop}
    (sound : ExternalSound contracts truth compatible) {initial s : State}
    (base : CompatibleState compatible initial.toOperationalState)
    (execution : CompatibleExecution contracts initial s) : CompatibleState compatible s.toOperationalState := by
  induction execution with
  | initial => exact base
  | operational _ executed ih => exact E11_step_preserves_compatibility ih executed
  | external _ controlled executed ih =>
      rcases E11_environment_preserves_or_external_change sound ih executed with preserved | outside
      · exact preserved
      · rw [controlled] at outside
        contradiction
  | refused _ ih => exact ih

theorem E11_external_change_can_break_compatibility {contracts : Contracts} {s : State}
    {compatible : ProviderRevision → Nat → Nat → Prop} {revision : ProviderRevision} {producer consumer : Nat}
    (incompatible : ¬ compatible revision producer consumer) :
    ∃ t, Transition contracts s t ∧ ¬ CompatibleState compatible t.toOperationalState := by
  refine ⟨{ s with world := { s.world with provider := revision, producer := producer, consumer := consumer } },
    Transition.external (action := .changeProviderState revision producer consumer) rfl, incompatible⟩

end Kctx.Engineering
