import Kctx.Citation.Reuse

set_option autoImplicit false
namespace Kctx.Citation

theorem CIT05_reuse_requires_exact_bindings {store : Store} {access : Access}
    {kb : KB} {context : Context} {contextId : ContextId} {claim : Ref} {current : List Binding}
    {validation : BindingValidation}
    (accepted : citationReusable store access kb context contextId claim current validation = true) :
    contextId = validation.contextId ∧ current = validation.bindings ∧
      bindingsInContext contextId validation current = true ∧
      bindingsExact store access current = true ∧
      reusable kb context claim validation.legacy = true := by
  simpa only [citationReusable, Bool.and_eq_true, decide_eq_true_eq, and_assoc] using accepted

theorem CIT05_changed_binding_refuses {store : Store} {access : Access}
    {kb : KB} {context : Context} {contextId : ContextId} {claim : Ref} {current : List Binding}
    {validation : BindingValidation} (changed : current ≠ validation.bindings) :
    citationReusable store access kb context contextId claim current validation = false := by
  simp [citationReusable, changed]

theorem CIT05_foreign_context_refuses {store : Store} {access : Access}
    {kb : KB} {context : Context} {contextId : ContextId} {claim : Ref} {current : List Binding}
    {validation : BindingValidation} (foreign : contextId ≠ validation.contextId) :
    citationReusable store access kb context contextId claim current validation = false := by
  simp [citationReusable, foreign]

theorem bindingsExact_congr {before after : Store} {readBefore readAfter : Access}
    (bindings : List Binding)
    (same : ∀ binding ∈ bindings,
      resolve before readBefore binding = resolve after readAfter binding) :
    bindingsExact before readBefore bindings = bindingsExact after readAfter bindings := by
  induction bindings with
  | nil => rfl
  | cons binding rest ih =>
      have one := same binding (by simp)
      have others := ih (fun b hb => same b (by simp [hb]))
      change (resolvesExactly before readBefore binding && bindingsExact before readBefore rest) =
        (resolvesExactly after readAfter binding && bindingsExact after readAfter rest)
      rw [others]
      simp only [resolvesExactly, one]

/-- Unrelated capture/alias changes cannot invalidate an otherwise unchanged
binding receipt. Changes to legacy corrections/time/policy still use reusable. -/
theorem CIT05_unaffected_reuse {before after : Store} {readBefore readAfter : Access}
    {kb kb' : KB} {context : Context} {contextId : ContextId} {claim : Ref} {current : List Binding}
    {validation : BindingValidation}
    (same : ∀ binding ∈ current,
      resolve before readBefore binding = resolve after readAfter binding)
    (legacy : reusable kb context claim validation.legacy =
      reusable kb' context claim validation.legacy) :
    citationReusable before readBefore kb context contextId claim current validation =
      citationReusable after readAfter kb' context contextId claim current validation := by
  simp only [citationReusable, bindingsExact_congr current same, legacy]

/-- The actual computed transitive footprint is reused, not a new citation-only
notion of support. Unrelated evidence changes retain this independent witness. -/
theorem CIT05_computed_footprint_transfer {kb kb' : KB} {context : Context}
    {witness : Witness} {checked : Checked}
    (accepted : checkWitness kb context witness = some checked)
    (unchanged : FootprintMatches kb' context checked.dependencies) :
    checkWitness kb' context witness = some checked :=
  T07_computed_footprint_locality accepted unchanged

theorem CIT05_independent_alternative {kb : KB} {context : Context} {claim : Ref}
    {polarity : Polarity} (remaining : supportAlternatives kb context claim polarity ≠ []) :
    supported kb context claim polarity = true := T09_alternative_survives remaining

/-- Exact inherited inclusive expiry, for every integer timestamp and nonnegative
TTL. Acquisition scheduling hints are not inputs to this judgment. -/
theorem CIT06_inclusive_expiry (observed ttl : Int) (nonnegative : 0 ≤ ttl) :
    assessFresh (.expireAfter ttl) observed (observed + ttl) = .fresh := by
  apply (assessFresh_iff_fresh_expire ttl observed (observed + ttl)).mpr
  constructor <;> omega

theorem CIT06_no_refresh_without_observation {policy : TtlPolicy}
    {observed now later : Int} (stale : assessFresh policy observed now = .stale)
    (forward : now ≤ later) : assessFresh policy observed later = .stale :=
  assessFresh_stale_mono stale forward

theorem CIT06_future_not_fresh {policy : TtlPolicy} {observed now : Int}
    (future : now < observed) : assessFresh policy observed now = .unknown :=
  assessFresh_future_not_fresh future
end Kctx.Citation
