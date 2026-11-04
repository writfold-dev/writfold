import Kctx.Engineering.OperationalProofsRunsFrames
import Kctx.Engineering.OperationalProofsState

set_option autoImplicit false
set_option maxRecDepth 4096
namespace Kctx.Engineering


private theorem charge_account_survives {s t : State} {root : RunId} {cost : Nat}
    (h : charge s root cost = some t) :
    ∀ old ∈ s.budgets, ∃ next ∈ t.budgets, next.root = old.root := by
  rw [charge_exact_state h]
  intro old member
  refine ⟨if old.root == root then { old with spent := old.spent + cost } else old,
    List.mem_map.mpr ⟨old, member, rfl⟩, ?_⟩
  split <;> rfl

theorem activate_preserves_run_representation {contracts : Contracts} {s t : State}
    {run : Run} {budget : Nat} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : activate contracts s run budget = some t) :
    ∀ r ∈ t.runs, RunRepresentation contracts t.toOperationalState r := by
  unfold activate at h
  obtain ⟨checked, activated⟩ := Option.ite_none_right_eq_some.mp h
  · cases activated
    have liveRun : live s run = true := by
      simp only [Bool.and_eq_true] at checked
      grind only
    have floor := runs_live_floor liveRun
    simp only [Bool.and_eq_true, beq_iff_eq, List.contains_iff_mem,
      List.isEmpty_iff, Option.isNone_iff_eq_none] at checked
    intro r member
    rcases List.mem_cons.mp member with same | oldMember
    · subst r
      cases grant : currentGrant s run with
      | none => simp [grant] at checked
      | some g =>
        have grantBound : subset run.capabilities g.capabilities = true := by
          simp only [grant, Bool.and_eq_true] at checked
          grind only
        have gMember := runs_grant grant
        have emptyControl : run.control = ⟨[], [], []⟩ := by grind only
        have emptyLoaded : run.loaded = [] := by grind only
        have root : run.parent = none := by grind only
        refine {
          catalog := by grind only
          realm := by grind only
          input_type := by grind only
          input_retained := runs_retained (s := s) (by grind only)
          closure_retained := runs_retained (s := s) (by grind only)
          capabilities := runs_subset (by grind only)
          grant_bound := ⟨g, gMember.1, gMember.2,
            (runs_grant_scope grant).2.2.1, runs_subset grantBound⟩
          procedure_floor := runs_subset (by grind only)
          kernel_floor := fun gate member => floor gate (wf.governing_floor gate member)
          account := ⟨⟨run.id, budget, 0⟩, by simp, rfl⟩
          loaded := by simp [emptyLoaded]
          results_unique := by simp [emptyControl]
          results := by simp [emptyControl]
          routes := by simp [emptyControl]
          parent := by simp [root]
          root_delegation := by intro _; grind only
          call_origin := by simp [root]
          pending_nodes_unique := by simp [emptyControl]
          pending_children_unique := by simp [emptyControl]
          pending_results_disjoint := by simp [emptyControl]
          pending := by simp [emptyControl] }
    · exact (wf.runs r oldMember).transport
        (fun _ h => h) rfl (fun _ h => h)
        (fun g hg => ⟨g, hg, rfl, rfl, fun _ h => h⟩)
        (fun b hb => ⟨b, by simp [hb], rfl⟩)
        (fun old ho => ⟨old, by simp [ho], rfl⟩)

private theorem run_representation_status {contracts : Contracts} {s : OperationalState}
    {run : Run} (rep : RunRepresentation contracts s run) (status : RunStatus) :
    RunRepresentation contracts s { run with status := status } := by
  cases rep
  constructor <;> assumption

theorem rebind_preserves_run_representation {contracts : Contracts} {s t : State}
    {activation invocation : ActivationId} {procedure : Procedure} {binding : Binding}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : rebind contracts s activation procedure binding invocation = some t) :
    ∀ r ∈ t.runs, RunRepresentation contracts t.toOperationalState r := by
  unfold rebind at h
  cases found : findRun s activation with
  | none => simp [found] at h
  | some run =>
    simp only [found, bind, Option.bind] at h
    cases granted : currentGrant s { run with binding := binding } with
    | none => simp [granted] at h
    | some grant =>
      simp only [granted, bind, Option.bind] at h
      obtain ⟨checked, accepted⟩ := Option.ite_none_right_eq_some.mp h
      · obtain ⟨preconditions, resumed⟩ := Option.ite_none_right_eq_some.mp accepted
        ·
          cases debited : charge s run.id 1 with
          | none => simp [debited] at resumed
          | some charged =>
            simp only [debited, bind, Option.bind] at resumed
            cases resumed
            let next : Run := { run with
              procedure := procedure, binding := binding, invocation := invocation,
              incarnation := s.incarnation, status := .active,
              capabilities := childCapabilities grant.capabilities run.capabilities procedure.ceiling,
              floor := inheritedFloor run.floor procedure.floor s.authority.floor,
              correctionGeneration := s.authority.correctionGeneration,
              policy := s.authority.policy, design := none, loaded := [],
              control := ⟨[], [], []⟩ }
            let old := putRun charged { run with status := .retired }
            let final : State := { old with
              runs := next :: old.runs
              effects := old.effects.map (fun effect =>
                if effect.intent.run == run.id && effect.outcome == .prepared then
                  { effect with outcome := .cancelledBeforeDispatch } else effect) }
            change ∀ r ∈ final.runs, RunRepresentation contracts final.toOperationalState r
            have runMember := runs_find_member found
            have original := wf.runs run runMember
            have sameRuns : charged.runs = s.runs := by rw [charge_exact_state debited]
            have anchors : ∀ r ∈ s.runs, ∃ r' ∈ old.runs, r'.anchor = r.anchor := by
              rw [← sameRuns]
              exact runs_anchor_put (s := charged) (old := run) (next := { run with status := .retired })
                (by simpa [sameRuns] using wf.activations_unique)
                (by simpa [sameRuns] using runMember) rfl
            have ambient : ∀ r, RunRepresentation contracts s.toOperationalState r →
                RunRepresentation contracts final.toOperationalState r := by
              intro r rep
              apply rep.transport
              · intro p hp
                simpa [final, old, putRun, charge_exact_state debited] using hp
              · simp [final, old, putRun, charge_exact_state debited]
              · intro object member
                simpa [final, old, putRun, charge_exact_state debited] using member
              · intro g member
                exact ⟨g, by simpa [final, old, putRun, charge_exact_state debited] using member,
                  rfl, rfl, fun _ h => h⟩
              · intro account member
                obtain ⟨account', member', same⟩ := charge_account_survives debited account member
                exact ⟨account', member', same⟩
              · intro before member
                obtain ⟨after, afterMember, same⟩ := anchors before member
                exact ⟨after, by simp [final, afterMember], same⟩
            have base := ambient run original
            have grantMember := runs_grant granted
            simp only [Bool.and_eq_true, beq_iff_eq, List.contains_iff_mem,
              Option.isNone_iff_eq_none] at checked
            have noParent : run.parent = none := by grind only
            have nextRep : RunRepresentation contracts final.toOperationalState next := by
              refine {
                catalog := by
                  change procedure ∈ final.catalog
                  simpa [final, old, putRun, charge_exact_state debited] using
                    (show procedure ∈ s.catalog by grind only)
                realm := by
                  change binding.realm = final.realm
                  simpa [final, old, putRun, charge_exact_state debited] using
                    (show binding.realm = s.realm by grind only)
                input_type := by change run.input.type = procedure.inputType; grind only
                input_retained := base.input_retained
                closure_retained := by
                  change ∀ object ∈ procedure.closure, object ∈ final.retained
                  have closure : retained s procedure.closure = true := by grind only
                  simpa [final, old, putRun, charge_exact_state debited] using runs_retained closure
                capabilities := by
                  intro cap member
                  exact (show cap ∈ run.capabilities ∧ cap ∈ grant.capabilities ∧
                    cap ∈ procedure.ceiling from by
                      simpa [next, childCapabilities, Bool.and_eq_true] using member).2.2
                grant_bound := by
                  refine ⟨grant, ?_, grantMember.2,
                    (runs_grant_scope granted).2.2.1, ?_⟩
                  · simpa [final, old, putRun, charge_exact_state debited] using grantMember.1
                  · intro cap member
                    exact (show cap ∈ run.capabilities ∧ cap ∈ grant.capabilities ∧
                      cap ∈ procedure.ceiling from by
                        simpa [next, childCapabilities, Bool.and_eq_true] using member).2.1
                procedure_floor := by
                  intro gate member
                  simp [next, inheritedFloor, member]
                kernel_floor := by
                  intro gate member
                  have oldFloor := base.kernel_floor gate member
                  simp [next, inheritedFloor, oldFloor]
                account := base.account
                loaded := by simp [next]
                results_unique := by simp [next]
                results := by simp [next]
                routes := by simp [next]
                parent := by simp [next, noParent]
                root_delegation := by
                  intro _
                  exact original.root_delegation noParent
                call_origin := by simp [next, noParent]
                pending_nodes_unique := by simp [next]
                pending_children_unique := by simp [next]
                pending_results_disjoint := by simp [next]
                pending := by simp [next] }
            intro r member
            rcases List.mem_cons.mp member with same | oldMember
            · subst r
              exact nextRep
            · obtain ⟨before, beforeMember, rfl⟩ := List.mem_map.mp oldMember
              split
              · exact run_representation_status base .retired
              · exact ambient before (wf.runs before (by simpa [sameRuns] using beforeMember))

end Kctx.Engineering
