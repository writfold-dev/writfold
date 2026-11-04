import Kctx.Engineering.EffectsSafetyStep

set_option autoImplicit false
set_option maxRecDepth 4096
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

/-- Observations, cancellation and reconciliation are deliberately not mutation
continuations. They remain possible after authority has been withdrawn. -/
def ContinuationChecked (contracts : Contracts) (s : State) : Action → Prop
  | .activate run _ => live s run = true
  | .loadResource id _ | .advanceNode id _ _ | .failNode id _ _ |
    .selectRoute id _ _ _ | .call id _ _ _ _ | .reserve id _ _ |
    .reuse id _ | .reopen id _ =>
      ∃ run, findRun s id = some run ∧ live s run = true
  | .prepare intent _ => ∃ run, findRun s intent.invocation = some run ∧ live s run = true
  | .claimDispatch id _ _ _ | .retryClaim id _ _ _ | .issueEffect id =>
      ∃ effect run, findEffect s id = some effect ∧
        findRun s effect.intent.invocation = some run ∧ live s run = true
  | .returnChild parent child _ =>
      ∃ p c, findRun s parent = some p ∧ findRun s child = some c ∧
        live s p = true ∧ live s c = true
  | .rebind id procedure binding invocation =>
      ∃ run grant, findRun s id = some run ∧ currentGrant s { run with binding := binding } = some grant ∧
        s.alive = true ∧
        ⟨run.id, run.binding, binding, s.incarnation, s.world.provider⟩ ∈ contracts.migrations ∧
        procedure ∈ s.catalog ∧ procedure.revision ∉ s.authority.withdrawn ∧
        ¬ s.runs.any (·.invocation == invocation) = true
  | .resume id =>
      ∃ run grant, findRun s id = some run ∧ currentGrant s run = some grant ∧
        s.alive = true ∧ run.status = .active ∧ run.incarnation ≠ s.incarnation ∧
        contracts.resumptions.any (resumeAuthorizationValid s run) = true ∧
        (run.parent.isNone || (run.parent.bind (findRun s)).any
          (fun parent => live s parent && parent.id == run.id && parent.binding == run.binding)) = true ∧
        s.ownership.all (fun owner => owner.owner != run.id ||
          (owner.children == 0 && contracts.quiescent.contains
            (owner.resource, owner.generation, owner.processGeneration))) = true ∧
        run.procedure.revision ∉ s.authority.withdrawn
  | _ => True

private theorem resume_live_not_withdrawn {s : State} {run : Run}
    (h : live s run = true) : run.procedure.revision ∉ s.authority.withdrawn := by
  simp only [live, ancestryLive, Bool.and_eq_true] at h
  have localGuard := h.1
  simp only [locallyLive, Bool.and_eq_true] at localGuard
  simpa only [Bool.not_eq_true', List.contains_eq_mem, decide_eq_false_iff_not] using
    localGuard.1.1.1.1.2

theorem resume_checks_continuation {contracts : Contracts} {s t : State} {activation : ActivationId}
    (h : resume contracts s activation = some t) :
    ContinuationChecked contracts s (.resume activation) := by
  cases found : findRun s activation with
  | none => simp [resume, found] at h
  | some run =>
      cases grantFound : currentGrant s run with
      | none => simp [resume, found, grantFound] at h
      | some grant =>
          simp only [resume, found, grantFound, bind, Option.bind_some] at h
          split at h
          next guard =>
            simp only [Bool.and_eq_true, beq_iff_eq, bne_iff_ne, and_assoc] at guard
            rcases guard with ⟨alive, active, stale, _, _, _, authorized, parent, quiet, nextLive, _⟩
            have notWithdrawn := resume_live_not_withdrawn nextLive
            exact ⟨run, grant, by simp only [found], by simp only [grantFound],
              alive, active, stale, authorized, parent, quiet, notWithdrawn⟩
          next => contradiction

theorem domain_body_checks_continuation {contracts : Contracts} {s t : State} {action : Action}
    (h : domainBody contracts s action = some t) : ContinuationChecked contracts s action := by
  cases action <;> try trivial
  case resume activation => exact resume_checks_continuation h
  all_goals simp only [domainBody, activate, loadResource, advanceNode, failNode,
    selectRoute, call, prepare, reserve, claimDispatch, retryClaim, issueEffect, reuse, returnChild,
    rebind, reopen, bind, Option.bind] at h
  all_goals repeat first
    | contradiction
    | (split at h <;> try dsimp only at h)
    | (cases h; simp_all [ContinuationChecked, nodeReady, enabled, intentReady,
        receiptValid, Bool.and_eq_true, Bool.or_eq_true, Bool.not_eq_true', List.contains_iff])

theorem E12_domain_checks_continuation {contracts : Contracts} {s t : State} {action : Action}
    (h : domain contracts s action = some t) : ContinuationChecked contracts s action := by
  cases body : domainBody contracts s action with
  | none => simp [domain, body] at h
  | some next => exact domain_body_checks_continuation body

theorem ancestryLive_ledger_frame (s : State) (ledger : Ledger.Store Action HistoricalPremises)
    (cuts : List ContextCut) (fuel : Nat) (run : Run) :
    ancestryLive { s with ledger := ledger, cuts := cuts } fuel run = ancestryLive s fuel run := by
  induction fuel generalizing run with
  | zero => rfl
  | succ fuel ih =>
      simp only [ancestryLive, locallyLive, currentGrant, findRun, ih]

theorem live_ledger_frame (s : State) (ledger : Ledger.Store Action HistoricalPremises)
    (cuts : List ContextCut) (run : Run) :
    live { s with ledger := ledger, cuts := cuts } run = live s run := by
  simpa only [live] using ancestryLive_ledger_frame s ledger cuts (s.runs.length + 1) run

theorem continuationChecked_ledger_frame (contracts : Contracts) (s : State) (action : Action)
    (ledger : Ledger.Store Action HistoricalPremises) (cuts : List ContextCut) :
    ContinuationChecked contracts { s with ledger := ledger, cuts := cuts } action ↔
      ContinuationChecked contracts s action := by
  have findFrame : findRun { s with ledger := ledger, cuts := cuts } = findRun s := by
    funext activation
    rfl
  have resumeFrame : resumeAuthorizationValid { s with ledger := ledger, cuts := cuts } =
      resumeAuthorizationValid s := by
    funext run authorization
    rfl
  cases action <;> simp only [ContinuationChecked, live_ledger_frame,
    findFrame, resumeFrame, findRun, findEffect, currentGrant]

/-- An append's replay checks the same operational continuation premises as the
live source. Captured ledger prefix reconstruction cannot refresh a grant. -/
theorem E12_append_checks_continuation {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {id : LedgerRecordId} {action : Action}
    (h : step contracts s (.append stream expected id action) = some t) :
    ContinuationChecked contracts s action := by
  obtain ⟨next, checked, _⟩ := append_domain_witness h
  have guarded := E12_domain_checks_continuation checked
  exact (continuationChecked_ledger_frame contracts s action
    (Ledger.fromPrefix stream (Ledger.currentPrefix s.ledger stream) s.retained) []).mp guarded

theorem E12_dispatch_checks_continuation {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {effect : Id .effect}
    (h : step contracts s (.dispatch stream expected effect) = some t) :
    ContinuationChecked contracts s (.issueEffect effect) := by
  simp only [step, dispatchSelected] at h
  split at h
  · exact E12_domain_checks_continuation h
  · contradiction

theorem live_checks_local_authority {s : State} {run : Run} (h : live s run = true) :
    locallyLive s run = true := by
  simp only [live, ancestryLive, Bool.and_eq_true] at h
  exact h.1

/-- All six current premises are checked at each actual continuation, not merely
when the root was admitted. The grant is the exact selected unexpired grant. -/
theorem E12_live_current_authority {s : State} {run : Run} (h : live s run = true) :
    s.alive = true ∧ run.status = .active ∧ run.binding.realm = s.realm ∧
    run.incarnation = s.incarnation ∧ run.procedure.revision ∉ s.authority.withdrawn ∧
    run.correctionGeneration = s.authority.correctionGeneration ∧ run.policy = s.authority.policy ∧
    subset s.authority.floor run.floor = true ∧
    ∃ grant, currentGrant s run = some grant ∧ grant.run = run.id ∧ grant.binding = run.binding ∧
      grant.revoked = false ∧ grant.validFrom ≤ s.now ∧ s.now < grant.expires := by
  have locallyCurrent := live_checks_local_authority h
  simp only [locallyLive, Bool.and_eq_true, Bool.not_eq_true', beq_iff_eq,
    List.contains_eq_mem, decide_eq_false_iff_not, and_assoc] at locallyCurrent
  rcases locallyCurrent with ⟨alive, active, realm, incarnation, withdrawn, correction, policy, floor, grant⟩
  refine ⟨alive, active, realm, incarnation, withdrawn, correction, policy, floor, ?_⟩
  cases found : currentGrant s run with
  | none => simp [found] at grant
  | some current =>
      have selected := List.find?_some (show s.authority.grants.find? (fun g =>
        g.run == run.id && g.binding == run.binding && !g.revoked &&
          g.validFrom ≤ s.now && s.now < g.expires) = some current from found)
      have exactness : current.run = run.id ∧ current.binding = run.binding ∧
          current.revoked = false ∧ current.validFrom ≤ s.now ∧ s.now < current.expires := by
        simpa only [Bool.and_eq_true, Bool.not_eq_true', beq_iff_eq, decide_eq_true_eq,
          and_assoc] using selected
      exact ⟨current, rfl, exactness⟩

theorem E12_live_checks_parent {s : State} {run : Run} {parent : ActivationId}
    (liveNow : live s run = true) (parentOf : run.parent = some parent) :
    ∃ ancestor, findRun s parent = some ancestor ∧ ancestor.id = run.id ∧
      ancestor.binding = run.binding ∧ subset run.capabilities ancestor.capabilities = true ∧
      subset ancestor.floor run.floor = true ∧ ancestryLive s s.runs.length ancestor = true := by
  simp only [live, ancestryLive, parentOf, Bool.and_eq_true] at liveNow
  cases found : findRun s parent with
  | none => simp [found] at liveNow
  | some ancestor =>
      refine ⟨ancestor, rfl, ?_⟩
      simpa only [found, Option.any_some, Bool.and_eq_true, beq_iff_eq, and_assoc] using liveNow.2

/-- Recovery changes the coordinator incarnation, never the incarnation carried
by an old activation, its input, its receipts or its ledger records. -/
theorem E12_recovery_preserves_historical_inputs {s t : State} (h : recover s = some t) :
    t.runs = s.runs ∧ t.effects = s.effects ∧ t.receipts = s.receipts ∧
      t.ledger = s.ledger ∧ t.incarnation = s.incarnation + 1 := by
  unfold recover at h
  split at h
  · cases h
    exact ⟨rfl, rfl, rfl, rfl, rfl⟩
  · contradiction

theorem E12_recovered_old_activation_refused {s t : State} {run : Run}
    (recovered : recover s = some t) (old : run.incarnation = s.incarnation) :
    live t run = false := by
  have incarnation := (E12_recovery_preserves_historical_inputs recovered).2.2.2.2
  have stale : run.incarnation ≠ t.incarnation := by omega
  simp [live, ancestryLive, locallyLive, stale]

theorem E12_return_refuses_stale_child {contracts : Contracts} {s : State}
    {parent child : ActivationId} {run : Run} {receipt : Receipt}
    (found : findRun s child = some run) (stale : run.incarnation ≠ s.incarnation) :
    returnChild contracts s parent child receipt = none := by
  have dead : live s run = false := by simp [live, ancestryLive, locallyLive, stale]
  cases result : returnChild contracts s parent child receipt with
  | none => rfl
  | some next =>
      have checked : ContinuationChecked contracts s (.returnChild parent child receipt) :=
        domain_body_checks_continuation result
      obtain ⟨parentRun, childRun, _, childFound, _, childLive⟩ := checked
      have same : childRun = run := Option.some.inj (childFound.symm.trans found)
      subst childRun
      rw [dead] at childLive
      contradiction

/-- Every recorded migration has the full qualified old/new binding and current
incarnation/provider, not a digest comparison or an implicit resume. -/
theorem E12_append_migration_exact {contracts : Contracts} {s t : State}
    {stream : StreamId} {expected : Option Ledger.Head} {id : LedgerRecordId}
    {run invocation : ActivationId} {procedure : Procedure} {binding : Binding}
    (h : step contracts s (.append stream expected id (.rebind run procedure binding invocation)) = some t) :
    ∃ old grant, findRun s run = some old ∧ currentGrant s { old with binding := binding } = some grant ∧
      s.alive = true ∧
      ⟨old.id, old.binding, binding, s.incarnation, s.world.provider⟩ ∈ contracts.migrations ∧
      procedure ∈ s.catalog ∧ procedure.revision ∉ s.authority.withdrawn ∧
      ¬ s.runs.any (·.invocation == invocation) = true :=
  E12_append_checks_continuation h

def RequestContinuationChecked (contracts : Contracts) (s : State) : Request → Prop
  | .append _ _ _ action => ContinuationChecked contracts s action
  | .dispatch _ _ effect => ContinuationChecked contracts s (.issueEffect effect)
  | .publish _ _ => True
  | .local action => match action with
    | .project _ _ _ | .acquireReadCut _ | .releaseReadCut _ | .crash | .recover => True
    | _ => False

theorem E12_step_checks_continuation {contracts : Contracts} {s t : State} {request : Request}
    (h : step contracts s request = some t) : RequestContinuationChecked contracts s request := by
  cases request with
  | append stream expected id action => exact E12_append_checks_continuation h
  | dispatch stream expected effect => exact E12_dispatch_checks_continuation h
  | publish stream expected => trivial
  | «local» action =>
      cases action <;> try trivial
      all_goals simp only [step, localAction] at h
      all_goals contradiction

/-- This audit retains each actual request and its source-state checks. It is
derived by induction over Reachable, not supplied as a premise of execution. -/
inductive ContinuationAudited (contracts : Contracts) (initial : State) : State → Prop where
  | initial : ContinuationAudited contracts initial initial
  | operational {s t : State} {request : Request} :
      ContinuationAudited contracts initial s → step contracts s request = some t →
      RequestContinuationChecked contracts s request → ContinuationAudited contracts initial t
  | external {s t : State} {action : Environment} :
      ContinuationAudited contracts initial s → environment contracts s action = some t →
      ContinuationAudited contracts initial t
  | refused {s : State} : ContinuationAudited contracts initial s → ContinuationAudited contracts initial s

theorem E12_reachable_continuation_audited {contracts : Contracts} {initial s : State}
    (h : Reachable contracts initial s) : ContinuationAudited contracts initial s := by
  induction h with
  | initial => exact ContinuationAudited.initial
  | next _ transition ih =>
      cases transition with
      | operational executed =>
          exact ContinuationAudited.operational ih executed (E12_step_checks_continuation executed)
      | external executed => exact ContinuationAudited.external ih executed
      | refused => exact ContinuationAudited.refused ih

end Kctx.Engineering
