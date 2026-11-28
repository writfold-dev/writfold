import Writfold.Conform.AdmissionGuard
import Writfold.Plan
import Writfold.Current
import Writfold.Round
import Writfold.Delivery
import Writfold.Handoff
import Writfold.Compose
import Writfold.Inventory
import Writfold.Policy
import Calculus.Account

/-!
# Admission links, part 2: plans, rounds, delivery, handoff, compose and
policy lines (#209)

A plan step advances, a delivery is ready, a handoff is accepted and a
skill's closure is admitted exactly as `Calculus.admit`, `Tri.all` or
`universal` says over the dictionary. Refusals that name the first owed
gate or line are the refusal's name, not its decision, and are declared
out in `proofs/laws.toml`.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Conform.Admit

/-- A gate fact as a word: undefined is unknown; a standing holds when the
policy accepts it (`Calculus.met`). -/
def gateFactTri : plan.GateFact → Calculus.Tri
  | .Undefined => .unknown
  | .Stands s a => ofBool (Calculus.met (standingOf s) (acceptOf a))

/-- A loop exit is met. -/
def exitMet : plan.round.Exit → Bool
  | .Met => true
  | _ => false

/-- A summary that closes the run. -/
def closesB : policy.Summary → Bool
  | .Complete => true
  | .CompleteWithExceptions => true
  | _ => false

/-- A policy pin that moved. -/
def movedB : policy.Pin → Bool
  | .Moved => true
  | _ => false

/-- A reference fact as a word: only a found reference holds. -/
def refTri : compose.RefFact → Calculus.Tri
  | .Found => .yes
  | _ => .no

/-- A required key is granted. -/
noncomputable def keyB (keys : List (alloc.vec.Vec U8)) (key : List U8) : Bool :=
  @decide (Core.Plan.KeyIn keys key) (Classical.propDecidable _)

theorem keyB_iff (keys : List (alloc.vec.Vec U8)) (key : List U8) :
    keyB keys key = true ↔ Core.Plan.KeyIn keys key := by
  simp [keyB]

/-- A line of a policy: a skipped line is declared out of the universe;
another is judged met or not. -/
def lineMember (l : policy.lines.Listed) : Calculus.Member × Calculus.Tri :=
  if !l.skipped then (.judged, ofBool l.met) else (.declaredOut, .yes)

/-- A delivery channel: a withdrawn one is out of the universe; a live one
is judged ready or not. -/
def liveMember (c : delivery.Seen) : Calculus.Member × Calculus.Tri :=
  if c.live then (.judged, ofBool c.ready) else (.declaredOut, .yes)

theorem pmetB_met (s : gate.Standing) (a : policy.Accept) :
    Core.Plan.pmetB s a = Calculus.met (standingOf s) (acceptOf a) := by
  rcases s with st | _ | _
  · cases st <;> simp [Core.Plan.pmetB, standingOf, acceptOf, Calculus.met]
  · simp [Core.Plan.pmetB, standingOf, acceptOf, Calculus.met]
  · simp [Core.Plan.pmetB, standingOf, acceptOf, Calculus.met]

theorem gateFactTri_yes (g : plan.GateFact) :
    gateFactTri g = .yes ↔ ∃ s a, g = .Stands s a ∧ Core.Plan.PMet s a := by
  cases g with
  | Undefined => simp [gateFactTri]
  | Stands s a => simp [gateFactTri, ← pmetB_met, Core.Plan.pmetB_iff]

theorem all_ofBool (l : List Bool) :
    Calculus.Tri.admits (Calculus.Tri.all (l.map ofBool)) = true ↔ ∀ b ∈ l, b = true := by
  rw [Calculus.Tri.admits_iff, Calculus.Tri.all_eq_yes]
  constructor
  · intro h b hb
    have := h (ofBool b) (List.mem_map_of_mem hb)
    cases b
    · exact absurd this (by simp [ofBool])
    · rfl
  · intro h t ht; obtain ⟨b, hb, rfl⟩ := List.mem_map.mp ht; rw [h b hb]; rfl

theorem admit_ofBool (l : List Bool) :
    Calculus.admit .deny (l.map ofBool) = .pass ↔ ∀ b ∈ l, b = true := by
  rw [Calculus.admit_pass_iff, ← Calculus.Tri.all_eq_yes, ← Calculus.Tri.admits_iff, all_ofBool]

/-- The judged members of a universe whose members are judged or declared
out by `p`. -/
theorem judgedOf_mixed {α : Type} (l : List α) (p f : α → Bool) :
    Calculus.judgedOf (l.map fun x => if p x then (Calculus.Member.judged, ofBool (f x))
      else (Calculus.Member.declaredOut, Calculus.Tri.yes)) = (l.filter p).map (fun x => ofBool (f x)) := by
  induction l with
  | nil => rfl
  | cons x l ih =>
    by_cases h : p x = true
    · simp [Calculus.judgedOf, h, List.filter_cons] at ih ⊢; exact ih
    · simp [Calculus.judgedOf, h, List.filter_cons] at ih ⊢; exact ih

theorem whole_mixed {α : Type} (l : List α) (p f : α → Bool) :
    Calculus.whole ((l.map fun x => if p x then (Calculus.Member.judged, ofBool (f x))
      else (Calculus.Member.declaredOut, Calculus.Tri.yes)).map (·.1)) = true := by
  rw [Calculus.whole_iff_no_remainder]
  simp only [List.map_map, List.mem_map, Function.comp, not_exists, not_and]
  intro x _; by_cases h : p x = true <;> simp [h]

theorem universal_mixed {α : Type} (e : Bool) (l : List α) (p f : α → Bool) :
    Calculus.Tri.admits (Calculus.universal e (l.map fun x => if p x then
      (Calculus.Member.judged, ofBool (f x)) else (Calculus.Member.declaredOut, Calculus.Tri.yes))) = true ↔
      (e = true ∨ ∃ x ∈ l, p x = true) ∧ ∀ x ∈ l, p x = true → f x = true := by
  rw [Calculus.Tri.admits_iff, Calculus.universal_yes_iff, judgedOf_mixed, whole_mixed]
  simp only [true_and, List.mem_map, List.mem_filter, ne_eq, List.map_eq_nil_iff,
    List.filter_eq_nil_iff, not_forall, Bool.not_eq_true, exists_prop]
  constructor
  · rintro ⟨h1, h2⟩
    refine ⟨?_, fun x hx hp => ?_⟩
    · rcases h2 with h2 | ⟨x, hx, hp⟩
      · exact Or.inl h2
      · exact Or.inr ⟨x, hx, by simpa using hp⟩
    · have := h1 _ ⟨x, ⟨hx, hp⟩, rfl⟩; simpa using this
  · rintro ⟨h1, h2⟩
    refine ⟨?_, ?_⟩
    · rintro t ⟨x, ⟨hx, hp⟩, rfl⟩; simp [h2 x hx hp]
    · rcases h1 with h1 | ⟨x, hx, hp⟩
      · exact Or.inl h1
      · exact Or.inr ⟨x, hx, by simp [hp]⟩

theorem universal_judged {α : Type} (e : Bool) (l : List α) (f : α → Bool) :
    Calculus.Tri.admits (Calculus.universal e (l.map fun x => (Calculus.Member.judged, ofBool (f x)))) = true ↔
      (e = true ∨ l ≠ []) ∧ ∀ x ∈ l, f x = true := by
  have := universal_mixed e l (fun _ => true) f
  simp only [ite_true] at this
  rw [this]
  have hl : (∃ x ∈ l, True) ↔ l ≠ [] :=
    ⟨fun ⟨_, hx, _⟩ => List.ne_nil_of_mem hx,
     fun h => let ⟨x, hx⟩ := List.exists_mem_of_ne_nil l h; ⟨x, hx, trivial⟩⟩
  rw [hl]; simp only [true_implies]

theorem exitOf_met (mn mx r : Nat) (a : Bool) :
    exitMet (Core.Round.exitOf mn mx r a) = (decide (mn ≤ r) && a) := by
  unfold Core.Round.exitOf
  by_cases h : mn ≤ r ∧ a = true
  · rw [if_pos h]; simp [exitMet, h.1, h.2]
  · rw [if_neg h]
    have : (decide (mn ≤ r) && a) = false := by cases a <;> simp_all
    rw [this]; split <;> rfl

end Writfold.Core.Conform.Admit

namespace Writfold.Core.Conform

open Admit

/-! ## Plans -/

theorem plan_fact_met_implements (g : plan.GateFact) :
    plan.fact_met g ⦃ b => b = match g with
      | .Undefined => false
      | .Stands s a => Calculus.met (standingOf s) (acceptOf a) ⦄ := by
  apply WP.spec_mono (Core.Plan.fact_met_spec g)
  intro b hb; rw [hb]
  cases g <;> simp [Core.Plan.factMetB, pmetB_met]

theorem plan_met_all_implements (gs : Slice plan.GateFact) :
    plan.met_all gs ⦃ r => r.val = gs.val.map (fun g => Calculus.Tri.admits (gateFactTri g)) ⦄ := by
  apply WP.spec_mono (Core.Plan.met_all_spec gs)
  intro r hr; rw [hr]
  apply List.map_congr_left
  intro g _
  cases g with
  | Undefined => rfl
  | Stands s a =>
    simp only [Core.Plan.factMetB, gateFactTri, pmetB_met]
    cases Calculus.met (standingOf s) (acceptOf a) <;> rfl

theorem plan_advance_check_implements (nodes : Slice plan.Node) (pos : Usize) (met : Slice Bool) :
    plan.advance_check nodes pos met ⦃ r => r = .Ok () ↔ Calculus.admit .deny
      (ofBool (decide (Core.Plan.cur nodes.val = some pos.val)) :: met.val.map ofBool) = .pass ⦄ := by
  apply WP.spec_mono (Core.Plan.advance_ok_iff nodes pos met)
  intro r hr; rw [hr, ← List.map_cons, admit_ofBool, List.forall_mem_cons, decide_eq_true_eq]

theorem plan_advance_implements (nodes : Slice plan.Node) (pos : Usize) (gs : Slice plan.GateFact) :
    plan.advance nodes pos gs ⦃ r => r = .Ok () ↔ Calculus.admit .deny
      (ofBool (decide (Core.Plan.cur nodes.val = some pos.val)) :: gs.val.map gateFactTri) = .pass ⦄ := by
  apply WP.spec_mono (Core.Plan.advance_facts_ok_iff nodes pos gs)
  intro r hr; rw [hr, Calculus.admit_pass_iff]
  simp [gateFactTri_yes]

/-- Law 3.5 at the plan, by transfer: an undefined gate never advances. -/
theorem plan_advance_undefined_never_passes (nodes : Slice plan.Node) (pos : Usize)
    (gs : Slice plan.GateFact) (h : plan.GateFact.Undefined ∈ gs.val) :
    plan.advance nodes pos gs ⦃ r => r ≠ .Ok () ⦄ := by
  apply WP.spec_mono (plan_advance_implements nodes pos gs)
  intro r hr hok
  exact Calculus.unknown_never_passes .deny
    (List.mem_cons_of_mem _ (List.mem_map.mpr ⟨_, h, rfl⟩)) (hr.mp hok)

theorem plan_skip_check_implements (nodes : Slice plan.Node) (pos : Usize) (locked : Bool) :
    plan.skip_check nodes pos locked ⦃ r => r = .Ok () ↔ Calculus.admit .deny
      [ofBool (decide (Core.Plan.cur nodes.val = some pos.val)), ofBool (!locked)] = .pass ⦄ := by
  apply WP.spec_mono (Core.Plan.skip_ok_iff nodes pos locked)
  intro r hr; rw [hr, Calculus.admit_pass_iff]; cases locked <;> simp

theorem plan_first_owed_implements (met : Slice Bool) :
    plan.first_owed met ⦃ r => r.isNone = Calculus.Tri.admits (Calculus.Tri.all (met.val.map ofBool)) ⦄ := by
  apply WP.spec_mono (Core.Plan.first_owed_spec met)
  intro r hr
  rw [Bool.eq_iff_iff, all_ofBool, ← Core.Plan.firstOwed_none_iff, ← hr]
  cases r <;> simp

theorem round_all_met_implements (met : Slice Bool) :
    plan.round.all_met met ⦃ a => a = Calculus.Tri.admits (Calculus.Tri.all (met.val.map ofBool)) ⦄ := by
  apply WP.spec_mono (Core.Round.all_met_spec met)
  intro a ha
  rw [ha, Bool.eq_iff_iff, all_ofBool, ← Core.Plan.firstOwed_none_iff]
  simp

theorem round_exit_implements (b : plan.round.Bounds) (r : Usize) (met : Slice Bool) :
    plan.round.exit b r met ⦃ e => exitMet e = Calculus.Tri.admits (Calculus.Tri.all
      (ofBool (decide (b.min.val ≤ r.val)) :: met.val.map ofBool)) ⦄ := by
  apply WP.spec_mono (Core.Round.exit_spec b r met)
  intro e he
  rw [he, exitOf_met, Bool.eq_iff_iff, ← List.map_cons, all_ofBool, List.forall_mem_cons,
    Bool.and_eq_true, decide_eq_true_eq, Option.isNone_iff_eq_none, Core.Plan.firstOwed_none_iff]

theorem round_advance_check_implements (nodes : Slice plan.Node) (pos : Usize)
    (b : plan.round.Bounds) (r : Usize) (met : Slice Bool) :
    plan.round.advance_check nodes pos b r met ⦃ res => res = .Ok () ↔ Calculus.admit .deny
      (ofBool (decide (Core.Plan.cur nodes.val = some pos.val)) ::
        ofBool (decide (b.min.val ≤ r.val)) :: met.val.map ofBool) = .pass ⦄ := by
  apply WP.spec_mono (Core.Round.advance_iff nodes pos b r met)
  intro res hres
  have hn : (Core.Plan.firstOwed met.val).isNone = true ↔ ∀ x ∈ met.val, x = true := by
    rw [Option.isNone_iff_eq_none, Core.Plan.firstOwed_none_iff]
  have hx : Core.Round.exitOf b.min.val b.max.val r.val (Core.Plan.firstOwed met.val).isNone = .Met ↔
      b.min.val ≤ r.val ∧ ∀ x ∈ met.val, x = true := by
    unfold Core.Round.exitOf
    rw [← hn]
    by_cases hm : b.min.val ≤ r.val ∧ (Core.Plan.firstOwed met.val).isNone = true
    · simp [hm]
    · rw [if_neg hm]; constructor
      · intro h; split at h <;> cases h
      · intro h; exact absurd h hm
  rw [hres, ← List.map_cons, ← List.map_cons, admit_ofBool, hx]
  simp only [List.forall_mem_cons, decide_eq_true_eq]

theorem round_again_check_implements (nodes : Slice plan.Node) (pos : Usize) (looped : Bool)
    (b : plan.round.Bounds) (r : Usize) (met : Slice Bool) :
    plan.round.again_check nodes pos looped b r met ⦃ res => (∃ n, res = .Ok n) ↔ Calculus.admit .deny
      [ofBool (decide (Core.Plan.cur nodes.val = some pos.val)), ofBool looped,
        ofBool (!Calculus.Tri.admits (Calculus.Tri.all
          (ofBool (decide (b.min.val ≤ r.val)) :: met.val.map ofBool))),
        ofBool (decide (r.val < b.max.val))] = .pass ⦄ := by
  apply WP.spec_mono (Core.Round.again_admitted_iff nodes pos looped b r met)
  intro res hres
  have hn : (Core.Plan.firstOwed met.val).isNone = true ↔ ∀ x ∈ met.val, x = true := by
    rw [Option.isNone_iff_eq_none, Core.Plan.firstOwed_none_iff]
  have hall : Calculus.Tri.admits (Calculus.Tri.all
      (ofBool (decide (b.min.val ≤ r.val)) :: met.val.map ofBool)) = true ↔
      b.min.val ≤ r.val ∧ ∀ x ∈ met.val, x = true := by
    rw [← List.map_cons, all_ofBool, List.forall_mem_cons, decide_eq_true_eq]
  have hx : Core.Round.exitOf b.min.val b.max.val r.val (Core.Plan.firstOwed met.val).isNone = .Again ↔
      ¬ Calculus.Tri.admits (Calculus.Tri.all
        (ofBool (decide (b.min.val ≤ r.val)) :: met.val.map ofBool)) = true ∧ r.val < b.max.val := by
    unfold Core.Round.exitOf
    rw [hall, ← hn]
    by_cases hm : b.min.val ≤ r.val ∧ (Core.Plan.firstOwed met.val).isNone = true
    · simp [hm]
    · rw [if_neg hm]
      by_cases hr : r.val < b.max.val <;> simp [hr] <;> intro h1 h2 <;> exact hm ⟨h1, by simp [h2]⟩
  rw [hres, hx, Calculus.admit_pass_iff]
  simp only [List.mem_cons, List.not_mem_nil, or_false, forall_eq_or_imp, forall_eq, ofBool_eq_yes,
    decide_eq_true_eq, Bool.not_eq_true']
  simp

/-! ## Delivery -/

theorem delivery_unmet_implements (o : delivery.Owed) :
    delivery.unmet o ⦃ b => b = !Calculus.met (standingOf o.standing) (acceptOf o.accept) ⦄ := by
  apply WP.spec_mono (Core.Delivery.unmet_spec o)
  intro b hb; rw [hb, Core.Delivery.ometB, pmetB_met]

theorem delivery_ready_implements (os : Slice delivery.Owed) :
    delivery.ready os ⦃ b => b = Calculus.Tri.admits (Calculus.universal false (os.val.map fun o =>
      (Calculus.Member.judged, ofBool (Calculus.met (standingOf o.standing) (acceptOf o.accept))))) ⦄ := by
  apply WP.spec_mono (Core.Delivery.ready_iff os)
  intro b hb
  rw [Bool.eq_iff_iff, hb, universal_judged]
  simp [← pmetB_met, Core.Plan.pmetB_iff]

/-- 3.5 by transfer: an empty delivery is never ready. -/
theorem delivery_empty_never_ready (os : Slice delivery.Owed) (h : os.val = []) :
    delivery.ready os ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (delivery_ready_implements os)
  intro b hb; rw [hb]
  have := Calculus.empty_never_yes (os.val.map fun o =>
    (Calculus.Member.judged, ofBool (Calculus.met (standingOf o.standing) (acceptOf o.accept))))
    (by simp [h, Calculus.judgedOf])
  cases hu : Calculus.universal false (os.val.map fun o =>
    (Calculus.Member.judged, ofBool (Calculus.met (standingOf o.standing) (acceptOf o.accept)))) <;>
    simp_all [Calculus.Tri.admits]

theorem delivery_postable_implements (c : delivery.Seen) :
    delivery.postable c ⦃ b => b = Calculus.Tri.admits (Calculus.Tri.all
      [ofBool c.same, ofBool c.ready, ofBool c.live, ofBool c.routed, ofBool (!c.spent)]) ⦄ := by
  apply WP.spec_mono (Core.Delivery.postable_spec c)
  intro b hb; rw [hb]
  rcases c with ⟨a, r, l, o, s⟩
  cases a <;> cases r <;> cases l <;> cases o <;> cases s <;> rfl

theorem delivery_guard_all_implements (cs : Slice delivery.Seen) :
    delivery.guard_all cs ⦃ b => b = Calculus.Tri.admits (Calculus.universal false (cs.val.map liveMember)) ⦄ := by
  apply WP.spec_mono (Core.Delivery.guard_all_spec cs)
  intro b hb
  rw [Bool.eq_iff_iff, hb]
  have := universal_mixed false cs.val (fun c => c.live) (fun c => c.ready)
  rw [show cs.val.map liveMember = cs.val.map (fun c => if c.live then
    (Calculus.Member.judged, ofBool c.ready) else (Calculus.Member.declaredOut, Calculus.Tri.yes)) from rfl,
    this, Core.Delivery.AllLiveReady]
  simp

theorem delivery_guard_any_implements (cs : Slice delivery.Seen) :
    delivery.guard_any cs ⦃ b => b = Calculus.Tri.admits
      (Calculus.search true (cs.val.any Core.Delivery.postableB)) ⦄ := by
  apply WP.spec_mono (Core.Delivery.guard_any_spec cs)
  intro b hb
  rw [Bool.eq_iff_iff, hb, Calculus.Tri.admits_iff, Calculus.search_yes_iff]
  simp [Core.Delivery.postableB_iff]

theorem delivery_guard_implements (body : delivery.Body) (u : delivery.Unread) (cs : Slice delivery.Seen) :
    delivery.guard body u cs ⦃ b => b = match body, u with
      | .Known, _ => Calculus.Tri.admits (Calculus.search true (cs.val.any Core.Delivery.postableB))
      | .Unknown, .AllReady => Calculus.Tri.admits (Calculus.universal false (cs.val.map liveMember))
      | .Unknown, .Deny => false ⦄ := by
  apply WP.spec_mono (Core.Delivery.guard_allow_iff body u cs)
  intro b hb
  rw [Bool.eq_iff_iff, hb]
  have hu := universal_mixed false cs.val (fun c => c.live) (fun c => c.ready)
  rw [show cs.val.map liveMember = cs.val.map (fun c => if c.live then
    (Calculus.Member.judged, ofBool c.ready) else (Calculus.Member.declaredOut, Calculus.Tri.yes)) from rfl]
  cases body <;> cases u <;>
    simp [hu, Core.Delivery.AllLiveReady, Calculus.Tri.admits_iff, Calculus.search_yes_iff,
      Core.Delivery.postableB_iff]

/-! ## Handoff -/

theorem handoff_strict_implements (c : Bool) (xs : Slice handoff.Item) :
    handoff.strict c xs ⦃ v => v = .Accepted ↔ Calculus.Tri.admits (Calculus.Tri.and (ofBool c)
      (Calculus.universal false (xs.val.map fun x =>
        (Calculus.Member.judged, ofBool (Core.Handoff.itemOf x).isNone)))) = true ⦄ := by
  apply WP.spec_mono (Core.Handoff.strict_accepted_iff c xs)
  intro v hv
  rw [hv, Calculus.Tri.admits_iff, Calculus.Tri.and_eq_yes, ← Calculus.Tri.admits_iff
    (Calculus.universal _ _), universal_judged]
  simp [Option.isNone_iff_eq_none, Core.Handoff.itemOf_none_iff]

theorem handoff_checked_implements (e c : Bool) (xs : Slice handoff.Item) :
    handoff.checked e c xs ⦃ v => v = .Accepted ↔ Calculus.Tri.admits (Calculus.Tri.and (ofBool e)
      (Calculus.Tri.and (ofBool c) (Calculus.universal false (xs.val.map fun x =>
        (Calculus.Member.judged, ofBool (Core.Handoff.itemOf x).isNone))))) = true ⦄ := by
  cases e
  · apply WP.spec_mono (Core.Handoff.incomplete_evidence_never_accepted c xs)
    intro v hv; rw [hv]; simp [Calculus.Tri.and, Calculus.Tri.admits]
  · rw [Core.Handoff.complete_evidence_unchanged]
    apply WP.spec_mono (handoff_strict_implements c xs)
    intro v hv; rw [hv]
    cases Calculus.Tri.and (ofBool c) (Calculus.universal false (xs.val.map fun x =>
        (Calculus.Member.judged, ofBool (Core.Handoff.itemOf x).isNone))) <;> rfl

/-! ## Compose -/

theorem covered_iff (req granted : List (alloc.vec.Vec U8)) :
    Core.Compose.Covered req granted ↔
      Calculus.Tri.admits (Calculus.Tri.all (req.map fun k => ofBool (keyB granted k.val))) = true := by
  rw [show req.map (fun k => ofBool (keyB granted k.val)) =
      (req.map fun k => keyB granted k.val).map ofBool by rw [List.map_map]; rfl,
    all_ofBool, List.forall_mem_map]
  simp only [keyB_iff]
  unfold Core.Compose.Covered
  exact (List.forall_mem_iff_forall_getElem (P := fun k : alloc.vec.Vec U8 =>
    Core.Plan.KeyIn granted k.val)).symm

theorem compose_covers_implements (req granted : Slice (alloc.vec.Vec U8)) :
    compose.covers req granted ⦃ r => r = none ↔ Calculus.Tri.admits (Calculus.Tri.all
      (req.val.map fun k => ofBool (keyB granted.val k.val))) = true ⦄ := by
  apply WP.spec_mono (Core.Compose.covers_spec req granted)
  intro r hr
  rw [Core.Compose.covers_none_iff _ _ r hr, covered_iff]

theorem compose_closure_implements (refs : Slice compose.RefFact) (req granted : Slice (alloc.vec.Vec U8)) :
    compose.closure refs req granted ⦃ r => r = .Ok () ↔ Calculus.admit .deny
      (refs.val.map refTri ++ req.val.map fun k => ofBool (keyB granted.val k.val)) = .pass ⦄ := by
  apply WP.spec_mono (Core.Compose.closure_spec refs req granted)
  intro r hr
  rw [Core.Compose.closure_ok_iff _ _ _ r hr, Calculus.admit_pass_iff, List.forall_mem_append,
    covered_iff, Calculus.Tri.admits_iff, Calculus.Tri.all_eq_yes]
  apply and_congr_left'
  unfold Core.Compose.AllFound
  rw [← List.forall_mem_iff_forall_getElem (P := fun x => x = compose.RefFact.Found)]
  constructor
  · intro h t ht
    obtain ⟨x, hx, rfl⟩ := List.mem_map.mp ht
    rw [h x hx]; rfl
  · intro h x hx
    have := h (refTri x) (List.mem_map_of_mem hx)
    cases x <;> simp_all [refTri]

/-! ## Inventory and policy lines -/

theorem unplanned_clean_implements (c : Bool) (met : Slice Bool) (v : Bool) :
    inventory.unplanned_clean c met v ⦃ b => b = Calculus.Tri.admits (Calculus.Tri.and (ofBool c)
      (Calculus.Tri.and (Calculus.universal false (met.val.map fun m => (Calculus.Member.judged, ofBool m)))
        (ofBool v))) ⦄ := by
  apply WP.spec_mono (Core.Inventory.unplanned_clean_iff c met v)
  intro b hb
  have hu := universal_judged false met.val id
  simp only [id] at hu
  rw [Bool.eq_iff_iff, hb, Calculus.Tri.admits_iff, Calculus.Tri.and_eq_yes, Calculus.Tri.and_eq_yes,
    ← Calculus.Tri.admits_iff (Calculus.universal _ _), hu]
  cases c <;> cases v <;> simp

theorem owed_met_implements (xs : Slice policy.lines.Listed) :
    policy.lines.owed_met xs ⦃ b => b = Calculus.Tri.admits (Calculus.universal true (xs.val.map lineMember)) ⦄ := by
  apply WP.spec_mono (Core.Policy.owed_met_spec xs)
  intro b hb
  rw [Bool.eq_iff_iff, hb]
  have := universal_mixed true xs.val (fun l => !l.skipped) (fun l => l.met)
  beta_reduce at this
  rw [show xs.val.map lineMember = xs.val.map (fun l => if !l.skipped then
    (Calculus.Member.judged, ofBool l.met) else (Calculus.Member.declaredOut, Calculus.Tri.yes)) from rfl]
  rw [this]
  simp [Core.Policy.Owed]

theorem lines_clean_implements (s : policy.Summary) (xs : Slice policy.lines.Listed) (v : Bool) :
    policy.lines.lines_clean s xs v ⦃ b => b = Calculus.Tri.admits (Calculus.Tri.all
      [ofBool (closesB s), Calculus.universal true (xs.val.map lineMember), ofBool v]) ⦄ := by
  apply WP.spec_mono (Core.Policy.lines_clean_iff s xs v)
  intro b hb
  have := universal_mixed true xs.val (fun l => !l.skipped) (fun l => l.met)
  rw [Bool.eq_iff_iff, hb, Calculus.Tri.admits_iff, Calculus.Tri.all_eq_yes]
  simp only [List.mem_cons, List.not_mem_nil, or_false, forall_eq_or_imp, forall_eq]
  rw [← Calculus.Tri.admits_iff (Calculus.universal _ _)]
  beta_reduce at this
  rw [show xs.val.map lineMember = xs.val.map (fun l => if !l.skipped then
    (Calculus.Member.judged, ofBool l.met) else (Calculus.Member.declaredOut, Calculus.Tri.yes)) from rfl]
  rw [this]
  cases s <;> cases v <;> simp [closesB, Core.Policy.Owed]

theorem lines_clean_over_implements (c : Bool) (s : policy.Summary) (xs : Slice policy.lines.Listed)
    (v : Bool) :
    inventory.lines_clean_over c s xs v ⦃ b => b = Calculus.Tri.admits (Calculus.Tri.all
      [ofBool c, ofBool (closesB s), Calculus.universal true (xs.val.map lineMember), ofBool v]) ⦄ := by
  apply WP.spec_mono (Core.Inventory.lines_clean_over_iff c s xs v)
  intro b hb
  have := universal_mixed true xs.val (fun l => !l.skipped) (fun l => l.met)
  rw [Bool.eq_iff_iff, hb, Calculus.Tri.admits_iff, Calculus.Tri.all_eq_yes]
  simp only [List.mem_cons, List.not_mem_nil, or_false, forall_eq_or_imp, forall_eq]
  rw [← Calculus.Tri.admits_iff (Calculus.universal _ _)]
  beta_reduce at this
  rw [show xs.val.map lineMember = xs.val.map (fun l => if !l.skipped then
    (Calculus.Member.judged, ofBool l.met) else (Calculus.Member.declaredOut, Calculus.Tri.yes)) from rfl]
  rw [this]
  cases c <;> cases s <;> cases v <;> simp [closesB, Core.Policy.Owed]

theorem repin_check_implements (w v : Bool) (p : policy.Pin) (l k : Bool) :
    policy.repin.repin_check w v p l k ⦃ r => r = .Ok () ↔ Calculus.admit .deny
      [ofBool w, ofBool v, ofBool (movedB p), ofBool l, ofBool k] = .pass ⦄ := by
  apply WP.spec_mono (Core.Policy.repin_ok_iff w v p l k)
  intro r hr; rw [hr, Calculus.admit_pass_iff]
  cases p <;> simp [movedB]

end Writfold.Core.Conform
