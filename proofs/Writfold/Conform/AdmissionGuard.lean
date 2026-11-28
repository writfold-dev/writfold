import Writfold.Conform.Links
import Writfold.Guard
import Writfold.Gate
import Writfold.Prose
import Writfold.Vale
import Writfold.Invoke
import Calculus.Site
import Calculus.Referent

/-!
# Admission links, part 1: the guard, outcomes and invoke (#209)

Every shipped kernel that answers "may this proceed?" or "is this
requirement met?" is shown to be `Calculus.admit`, `Tri.all`,
`Decision.join`, `Site.decide` or `settle` over the dictionary, each
derived from the kernel's existing spec theorem (`WP.spec_mono`), so the
kernel mutants calibrate unchanged.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Conform.Admit

/-- A two-valued answer as a word of the calculus. -/
def ofBool : Bool → Calculus.Tri
  | true => .yes
  | false => .no

@[simp] theorem ofBool_true : ofBool true = .yes := rfl
@[simp] theorem ofBool_false : ofBool false = .no := rfl

theorem ofBool_admits (b : Bool) : (ofBool b).admits = b := by cases b <;> rfl

@[simp] theorem ofBool_eq_yes (b : Bool) : ofBool b = .yes ↔ b = true := by cases b <;> simp [ofBool]
@[simp] theorem ofBool_ne_unknown (b : Bool) : ofBool b ≠ .unknown := by cases b <;> simp [ofBool]

/-- A rule in audit mode is no rule at the site; an enforced rule is judged. -/
def modeBasis : guard.Mode → Calculus.Decision → Calculus.Basis
  | .Enforce, d => .judged d
  | .Audit, _ => .noRules

/-- One guard rule as the calculus decides it: admitted on its requirement,
then read at the site under its mode. -/
def ruledD (r : guard.call.Ruled) : Calculus.Decision :=
  Calculus.Site.decide (modeBasis r.mode (Calculus.admit (actionOf r.action) [ofBool r.met])) .open

/-- A stop rule's repeat: once per session or once per turn. -/
def isSession : guard.Repeat → Bool
  | .Session => true
  | .Turn => false

/-- Kept outputs: missing ones leave a pass undecided. -/
def keptTri : gate.Kept → Calculus.Tri
  | .Missing => .unknown
  | _ => .yes

/-- A `for:` item: only one that could not be read is unknown. -/
def forTri : invoke.ForItem → Calculus.Tri
  | .Unknown => .unknown
  | _ => .yes

@[simp] theorem forTri_eq_yes (i : invoke.ForItem) : forTri i = .yes ↔ i ≠ .Unknown := by
  cases i <;> simp [forTri]

theorem metF_false (x : guard.Fact) : Core.Guard.metF x = false ↔ x ≠ .Met := by
  cases x <;> simp [Core.Guard.metF]

/-- `invoke prepare`'s own requirements, in the kernel's order. -/
def prepReqs (f : invoke.Prepare) : List Calculus.Tri :=
  [ofBool f.run_open, ofBool f.current_step, ofBool f.closure_admitted, ofBool f.plans_in_run,
    ofBool f.input_read, ofBool (!f.packet_required || f.packet_complete), ofBool f.out_outside,
    forTri f.for_item]

theorem foldr_join_pass (L : List Calculus.Decision) :
    L.foldr Calculus.Decision.join .pass = .pass ↔ ∀ x ∈ L, x = .pass := by
  induction L with
  | nil => simp
  | cons x L ih => simp [Calculus.Decision.join_eq_pass, ih]

theorem foldr_join_deny (L : List Calculus.Decision) :
    L.foldr Calculus.Decision.join .pass = .deny ↔ .deny ∈ L := by
  induction L with
  | nil => simp
  | cons x L ih =>
    simp only [List.foldr_cons, List.mem_cons]
    generalize L.foldr Calculus.Decision.join .pass = y at ih
    rw [← ih]
    cases x <;> cases y <;> simp [Calculus.Decision.join]

theorem ruledD_pass (r : guard.call.Ruled) : ruledD r = .pass ↔ r.mode = .Audit ∨ r.met = true := by
  rcases r with ⟨m, a, b⟩
  cases m <;> cases a <;> cases b <;> simp [Core.Guard.ruledOf, Core.Guard.effectOf, Core.Guard.actionOf, decisionOf, ruledD, modeBasis, Calculus.admit, Calculus.Site.decide, Calculus.Tri.all, Calculus.Tri.and, Calculus.Tri.admits, actionOf, Calculus.Action.decision, ofBool]

theorem ruledD_deny (r : guard.call.Ruled) :
    ruledD r = .deny ↔ r.mode = .Enforce ∧ r.action = .Deny ∧ r.met = false := by
  rcases r with ⟨m, a, b⟩
  cases m <;> cases a <;> cases b <;> simp [Core.Guard.ruledOf, Core.Guard.effectOf, Core.Guard.actionOf, decisionOf, ruledD, modeBasis, Calculus.admit, Calculus.Site.decide, Calculus.Tri.all, Calculus.Tri.and, Calculus.Tri.admits, actionOf, Calculus.Action.decision, ofBool]

end Writfold.Core.Conform.Admit

namespace Writfold.Core.Conform

open Admit

/-- A process's ending as a word: exit 0 holds, another exit code fails,
a signal, a timeout or an error could not be told. -/
def endingTri : gate.Ending → Calculus.Tri
  | .Exited i => if i.val = 0 then .yes else .no
  | _ => .unknown

theorem endingTri_onto (t : Calculus.Tri) : ∃ e, endingTri e = t := by
  cases t
  · exact ⟨.Exited 0#i32, rfl⟩
  · exact ⟨.Exited 1#i32, rfl⟩
  · exact ⟨.Signaled, rfl⟩

/-! ## The guard -/

theorem guard_met_implements (f : guard.Fact) :
    guard.met f ⦃ b => b = Calculus.Tri.admits (factTri f) ⦄ := by
  apply WP.spec_mono (Core.Guard.met_spec f)
  intro b hb; rw [hb]; cases f <;> rfl

theorem guard_all_met_implements (fs : Slice guard.Fact) :
    guard.all_met fs ⦃ b => b = Calculus.Tri.admits (Calculus.Tri.all (fs.val.map factTri)) ⦄ := by
  apply WP.spec_mono (Core.Guard.all_met_spec fs)
  intro b hb
  rw [all_metF, Bool.eq_iff_iff, hb, Core.Guard.allMetF_iff]

theorem guard_verdict_implements (m : Bool) (a : guard.Action) :
    guard.verdict m a ⦃ d => decisionOf d = Calculus.admit (actionOf a) [ofBool m] ⦄ := by
  apply WP.spec_mono (Core.Guard.verdict_spec m a)
  intro d hd; rw [hd]
  cases m <;> cases a <;> rfl

theorem guard_effect_implements (m : guard.Mode) (d : guard.Decision) :
    guard.effect m d ⦃ r => ∀ p, decisionOf r = Calculus.Site.decide (modeBasis m (decisionOf d)) p ⦄ := by
  apply WP.spec_mono (Core.Guard.effect_spec m d)
  intro r hr p; rw [hr]
  cases m <;> rfl

theorem guard_ruled_implements (r : guard.call.Ruled) :
    guard.call.ruled r ⦃ d => decisionOf d =
      Calculus.Site.decide (modeBasis r.mode (Calculus.admit (actionOf r.action) [ofBool r.met])) .open ⦄ := by
  apply WP.spec_mono (Core.Guard.ruled_spec r)
  intro d hd; rw [hd]
  rcases r with ⟨m, a, b⟩
  cases m <;> cases a <;> cases b <;> simp [Core.Guard.ruledOf, Core.Guard.effectOf, Core.Guard.actionOf, decisionOf, ruledD, modeBasis, Calculus.admit, Calculus.Site.decide, Calculus.Tri.all, Calculus.Tri.and, Calculus.Tri.admits, actionOf, Calculus.Action.decision, ofBool]

theorem guard_decide_all_implements (rs : Slice guard.call.Ruled) :
    guard.call.decide_all rs ⦃ d =>
      decisionOf d = (rs.val.map ruledD).foldr Calculus.Decision.join .pass ⦄ := by
  apply WP.spec_mono (WP.spec_and (Core.Guard.decide_all_pass_iff rs) (Core.Guard.decide_all_deny_iff rs))
  rintro d ⟨hp, hd⟩
  have hp' : (rs.val.map ruledD).foldr Calculus.Decision.join .pass = .pass ↔
      ∀ r ∈ rs.val, r.mode = .Audit ∨ r.met = true := by
    rw [foldr_join_pass]; simp [ruledD_pass]
  have hd' : (rs.val.map ruledD).foldr Calculus.Decision.join .pass = .deny ↔
      ∃ r ∈ rs.val, r.mode = .Enforce ∧ r.action = .Deny ∧ r.met = false := by
    rw [foldr_join_deny]
    simp only [List.mem_map]
    constructor
    · rintro ⟨r, hr, he⟩; exact ⟨r, hr, (ruledD_deny r).mp he⟩
    · rintro ⟨r, hr, he⟩; exact ⟨r, hr, (ruledD_deny r).mpr he⟩
  generalize (rs.val.map ruledD).foldr Calculus.Decision.join .pass = y at hp' hd'
  have e1 : d = .Pass ↔ y = .pass := hp.trans hp'.symm
  have e2 : d = .Deny ↔ y = .deny := hd.trans hd'.symm
  clear hp hd hp' hd'
  cases d <;> cases y <;> simp_all [decisionOf]

theorem gates_fact_implements (o u i : Usize) :
    guard.gates_fact o u i ⦃ f => factTri f =
      Calculus.Tri.all [ofBool (decide (0 < o.val)), ofBool (decide (u.val = 0)), ofBool (decide (i.val = 0))] ⦄ := by
  apply WP.spec_mono (WP.spec_and (Core.Guard.gates_fact_met_iff o u i)
    (Core.Guard.gates_fact_never_unknown o u i))
  rintro f ⟨hm, hu⟩
  by_cases h0 : 0 < o.val <;> by_cases h1 : u.val = 0 <;> by_cases h2 : i.val = 0 <;>
    cases f <;> simp_all [factTri, Calculus.Tri.all, Calculus.Tri.and, ofBool]

/-- `run.current` can never be unknown: both of its inputs are read. -/
theorem run_current_never_unknown (found open_ : Bool) :
    guard.require.run_current found open_ ⦃ f => f ≠ .Unknown ⦄ := by
  unfold guard.require.run_current
  cases found <;> cases open_ <;> simp

theorem run_current_implements (found open_ : Bool) :
    guard.require.run_current found open_ ⦃ f =>
      factTri f = Calculus.Tri.all [ofBool found, ofBool open_] ⦄ := by
  apply WP.spec_mono (WP.spec_and (Core.Guard.run_current_met_iff found open_)
    (run_current_never_unknown found open_))
  rintro f ⟨hm, hu⟩
  cases found <;> cases open_ <;> cases f <;> simp_all [factTri, Calculus.Tri.all, Calculus.Tri.and, ofBool]

theorem step_fact_implements (r : guard.require.Looked) (planned cur : Bool) (owed : Usize) :
    guard.require.step_fact r planned cur owed ⦃ f => factTri f = match r with
      | .Read => Calculus.Tri.all [ofBool (!planned || !cur || decide (owed.val = 0))]
      | .Failed => .unknown
      | .No => .no ⦄ := by
  apply WP.spec_mono (WP.spec_and (Core.Guard.step_fact_met_iff r planned cur owed)
    (Core.Guard.step_fact_unknown_iff r planned cur owed))
  rintro f ⟨hm, hu⟩
  by_cases h : owed.val = 0 <;>
    cases r <;> cases planned <;> cases cur <;> cases f <;>
      simp_all [factTri, Calculus.Tri.all, Calculus.Tri.and, ofBool]

theorem settled_fact_implements (r : guard.require.Looked) (planned cur : Bool) (owed : Usize)
    (spent : Bool) :
    guard.require.settled_fact r planned cur owed spent ⦃ f => factTri f = match r with
      | .Read => Calculus.Tri.all [ofBool (spent || !planned || !cur || decide (owed.val = 0))]
      | .Failed => .unknown
      | .No => .no ⦄ := by
  apply WP.spec_mono (WP.spec_and (Core.Guard.settled_met_iff r planned cur owed spent)
    (Core.Guard.settled_unknown_iff_unread r planned cur owed spent))
  rintro f ⟨hm, hu⟩
  by_cases h : owed.val = 0 <;>
    cases r <;> cases spent <;> cases planned <;> cases cur <;> cases f <;>
      simp_all [factTri, Calculus.Tri.all, Calculus.Tri.and, ofBool]

theorem gate_fact_implements (r : guard.require.Looked) (defined met : Bool) :
    guard.require.gate_fact r defined met ⦃ f => factTri f = match r with
      | .Read => Calculus.Tri.all [ofBool defined, ofBool met]
      | _ => .unknown ⦄ := by
  apply WP.spec_mono (WP.spec_and (Core.Guard.gate_fact_met_iff r defined met)
    (Core.Guard.gate_fact_unknown_iff r defined met))
  rintro f ⟨hm, hu⟩
  cases r <;> cases defined <;> cases met <;> cases f <;>
    simp_all [factTri, Calculus.Tri.all, Calculus.Tri.and, ofBool]

/-- `gates.met` is unknown unless the gates were judged. -/
theorem gates_read_unread (r : guard.require.Looked) (o u i : Usize) (h : r ≠ .Read) :
    guard.require.gates_read r o u i ⦃ f => f = .Unknown ⦄ := by
  unfold guard.require.gates_read
  cases r <;> simp_all

/-- Judged gates read as `gates_fact`. -/
theorem gates_read_read (o u i : Usize) :
    guard.require.gates_read .Read o u i ⦃ f => f ≠ .Unknown ⦄ := by
  unfold guard.require.gates_read
  exact Core.Guard.gates_fact_never_unknown o u i

theorem gates_read_implements (r : guard.require.Looked) (o u i : Usize) :
    guard.require.gates_read r o u i ⦃ f => factTri f = match r with
      | .Read => Calculus.Tri.all [ofBool (decide (0 < o.val)), ofBool (decide (u.val = 0)), ofBool (decide (i.val = 0))]
      | _ => .unknown ⦄ := by
  cases r
  case Read =>
    apply WP.spec_mono (WP.spec_and (Core.Guard.gates_read_met_iff .Read o u i) (gates_read_read o u i))
    rintro f ⟨hm, hu⟩
    by_cases h0 : 0 < o.val <;> by_cases h1 : u.val = 0 <;> by_cases h2 : i.val = 0 <;>
      cases f <;> simp_all [factTri, Calculus.Tri.all, Calculus.Tri.and, ofBool]
  all_goals
    apply WP.spec_mono (gates_read_unread _ o u i (by simp))
    intro f hf; rw [hf]; rfl

theorem nudge_implements (active nudged : Bool) (fs : Slice guard.Fact) :
    guard.nudge active nudged fs ⦃ b =>
      b = (!active && !nudged && !Calculus.Tri.admits (Calculus.Tri.all (fs.val.map factTri))) ⦄ := by
  apply WP.spec_mono (Core.Guard.nudge_spec active nudged fs)
  intro b hb
  rw [all_metF, Bool.eq_iff_iff, hb]
  cases active <;> cases nudged <;> simp [metF_false]

theorem nudge_with_implements (rep : guard.Repeat) (active nudged : Bool) (fs : Slice guard.Fact) :
    guard.nudge_with rep active nudged fs ⦃ b =>
      b = (!active && !(nudged && isSession rep) &&
        !Calculus.Tri.admits (Calculus.Tri.all (fs.val.map factTri))) ⦄ := by
  apply WP.spec_mono (Core.Guard.nudge_with_iff rep active nudged fs)
  intro b hb
  rw [all_metF, Bool.eq_iff_iff, hb]
  cases rep <;> cases active <;> cases nudged <;> simp [metF_false, isSession]

/-! ## Outcomes: a moved source or a missing output withholds a verdict -/

theorem outcomeOf_settle (e : gate.Ending) (moved : Bool) :
    outcomeTri (Core.Gate.outcomeOf e moved) =
      Calculus.settle Calculus.Tri.words (!moved) true (endingTri e) := by
  cases moved
  · cases e with
    | Exited i =>
      by_cases h : i.val = 0 <;>
        simp [h, Core.Gate.outcomeOf, endingTri, outcomeTri, Calculus.settle, Calculus.Tri.words]
    | _ => rfl
  · rfl

theorem gate_outcome_implements (e : gate.Ending) (moved : Bool) :
    gate.outcome e moved ⦃ o => outcomeTri o =
      Calculus.settle Calculus.Tri.words (!moved) true (endingTri e) ⦄ := by
  apply WP.spec_mono (Core.Gate.outcome_spec e moved)
  intro o ho; rw [ho, outcomeOf_settle]

theorem gate_outcome_kept_implements (e : gate.Ending) (moved : Bool) (k : gate.Kept) :
    gate.outcome_kept e moved k ⦃ o => outcomeTri o = Calculus.Tri.and
      (Calculus.settle Calculus.Tri.words (!moved) true (endingTri e)) (keptTri k) ⦄ := by
  apply WP.spec_mono (Core.Gate.outcome_kept_spec e moved k)
  intro o ho; rw [ho, ← outcomeOf_settle]
  cases k <;> cases Core.Gate.outcomeOf e moved <;> rfl

theorem prose_verdict_implements (rules : Usize) (found : Slice prose.Weight) :
    prose.verdict rules found ⦃ o => outcomeTri o = Calculus.settle Calculus.Tri.words
      (decide (rules.val ≠ 0)) true (ofBool (decide (Core.Prose.countB found.val = 0))) ⦄ := by
  apply WP.spec_mono (Core.Prose.verdict_spec rules found)
  intro o ho; rw [ho]
  by_cases h : rules.val = 0 <;> by_cases h' : Core.Prose.countB found.val = 0 <;>
    simp [h, h', Nat.pos_iff_ne_zero, Core.Prose.verdictOf, outcomeTri, Calculus.settle, Calculus.Tri.words, ofBool]

theorem vale_settle_implements (r : vale.Ran) (found : Slice prose.Weight) :
    vale.settle r found ⦃ o => outcomeTri o = Calculus.settle Calculus.Tri.words
      (Core.Vale.usableOf r && decide (Core.Vale.rulesOf r.enabled.val r.canary.val ≠ 0)) true
      (ofBool (decide (Core.Prose.countB found.val = 0))) ⦄ := by
  apply WP.spec_mono (Core.Vale.settle_spec r found)
  intro o ho; rw [ho]
  unfold Core.Vale.settleOf
  cases Core.Vale.usableOf r
  · rfl
  · by_cases h : Core.Vale.rulesOf r.enabled.val r.canary.val = 0 <;>
      by_cases h' : Core.Prose.countB found.val = 0 <;>
        simp [h, h', Nat.pos_iff_ne_zero, Core.Prose.verdictOf, outcomeTri, Calculus.settle, Calculus.Tri.words, ofBool]

/-! ## Invoke -/

theorem invoke_bound_implements (n : invoke.Named) :
    invoke.bound n ⦃ o => outcomeTri o = Calculus.Tri.and (ofBool n.names)
      (Calculus.settle Calculus.Tri.words n.readable true (Calculus.Tri.all
        [ofBool n.this_run, ofBool n.this_step, ofBool n.this_agent, ofBool n.closure_current,
          ofBool n.single])) ⦄ := by
  apply WP.spec_mono (Core.Invoke.bound_spec n)
  intro o ho; rw [ho]
  rcases n with ⟨a, b, c, d, e, g, h⟩
  cases a <;> cases b <;> cases c <;> cases d <;> cases e <;> cases g <;> cases h <;> rfl

theorem admitted_iff (f : invoke.Prepare) :
    Core.Invoke.Admitted f ↔ Calculus.admit .deny (prepReqs f) = .pass := by
  rw [Calculus.admit_pass_iff]
  simp only [prepReqs, List.mem_cons, List.not_mem_nil, forall_eq_or_imp, ofBool_eq_yes,
    forTri_eq_yes, Core.Invoke.Admitted, or_false, forall_eq, Bool.or_eq_true, Bool.not_eq_true']
  cases f.packet_required <;> simp

theorem prepare_check_implements (f : invoke.Prepare) :
    invoke.prepare_check f ⦃ r => r = .Ok () ↔ Calculus.admit .deny (prepReqs f) = .pass ⦄ := by
  apply WP.spec_mono (Core.Invoke.prepare_ok_iff f)
  intro r hr; rw [hr, admitted_iff]

theorem prepare_implements (f : invoke.Prepare) (needs : Slice guard.Fact) :
    invoke.prepare f needs ⦃ r => r = .Ok () ↔
      Calculus.admit .deny (prepReqs f ++ needs.val.map factTri) = .pass ⦄ := by
  apply WP.spec_mono (Core.Invoke.prepare_needs_ok_iff f needs)
  intro r hr
  rw [hr, admitted_iff, Calculus.admit_pass_iff, Calculus.admit_pass_iff, List.forall_mem_append]
  apply and_congr_right'
  unfold Core.Invoke.AllMet
  rw [← List.forall_mem_iff_forall_getElem (P := fun x => x = guard.Fact.Met)]
  constructor
  · intro h t ht
    obtain ⟨x, hx, rfl⟩ := List.mem_map.mp ht
    rw [h x hx]; rfl
  · intro h x hx
    have := h (factTri x) (List.mem_map_of_mem hx)
    cases x <;> simp_all [factTri]

end Writfold.Core.Conform
