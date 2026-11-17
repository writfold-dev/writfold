import Writfold.Gate
import Writfold.Plan
import Writfold.Policy

/-!
# About the current subject

Theorems over the Aeneas translation of the comparisons that decide whether
a receipt or a statement is about the present (`writfold_core::gate`:
`about`, `evidence`, `judge`, `stands`, `statements`) and of the advance on
the gates as found (`writfold_core::plan`: `fact_met`, `met_all`,
`advance`). Before 16 November 2026 these comparisons were made in
`writfold-kb` and the kernels received their results as booleans; now the
digests themselves are the kernels' input.

- `about_spec`: a record is about the present exactly when its recorded
  definition and content digests are the present ones, byte for byte.
- `judge_spec`, `judge_satisfied_iff`: a gate's state is `satisfied`
  exactly when its newest receipt about the present definition and the
  present (non-empty) content passed; `judge_other_bytes`: receipts about
  other bytes never satisfy, whatever they record.
- `statements_spec`: an attestation or a waiver stands only when it is
  about the present; `attested_any` counts every attestation.
- `advance_facts_spec`, `advance_facts_ok_iff`: a step is recorded done
  exactly when it is the current step and every gate it names has a usable
  definition and stands met; `advance_undefined_refused`.
- `met_only_when_current`, `done_only_when_met`: composed, a step is
  recorded done only when every gate stands on a passing newest receipt, a
  current attestation or a current waiver, each about the present digests.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Gate

/-! ## `about` -/

theorem about_spec (rd rc d c : Slice U8) :
    gate.about rd rc d c ⦃ b => b = true ↔ rd.val = d.val ∧ rc.val = c.val ⦄ := by
  unfold gate.about
  step with same_spec as ⟨ b, hb ⟩
  split
  · rename_i h
    apply WP.spec_mono (same_spec rc c)
    intro r hr
    have : rd.val = d.val := hb.mp h
    simp [this, hr]
  · rename_i h
    simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_and]
    intro hd
    exact absurd (hb.mpr hd) h

/-! ## `evidence` and `judge` -/

/-- A receipt is about the current subject: the present definition and the
present content in scope, which is not empty; and it is bound to this run
and subject. -/
def About (d c : List U8) (r : gate.Recorded) : Prop :=
  r.definition.val = d ∧ r.content.val = c ∧ c ≠ [] ∧ r.bound = true

/-- `evidence` on the model: the comparison flags of one receipt. -/
def evid (d c : List U8) (r : gate.Recorded) : gate.Evidence :=
  { outcome := r.outcome,
    same_definition := decide (r.definition.val = d),
    same_scope := decide (c ≠ [] ∧ r.content.val = c),
    bound := r.bound }

theorem isCurrent_evid (d c : List U8) (r : gate.Recorded) :
    isCurrent (evid d c r) = true ↔ About d c r := by
  simp only [isCurrent, evid, About, Bool.and_eq_true, decide_eq_true_eq]
  tauto

theorem evidence_spec (r : gate.Recorded) (d c : Slice U8) :
    gate.evidence r d c ⦃ e => e = evid d.val c.val r ⦄ := by
  unfold gate.evidence
  step with same_spec as ⟨ b, hb ⟩
  step with core.slice.Slice.is_empty_spec as ⟨ e, he ⟩
  split
  · rename_i hempty
    simp only [WP.spec_ok, evid, alloc.vec.Vec.deref]
    have hc : c.val = [] := by
      have : c.length = 0 := by simpa [hempty] using he.symm
      simpa [Slice.length] using this
    have hbd : b = decide (r.definition.val = d.val) := by
      cases b <;> simp_all [alloc.vec.Vec.deref, Slice.from_val]
    simp [hc, hbd]
  · rename_i hne
    step with same_spec as ⟨ b2, hb2 ⟩
    simp only [WP.spec_ok, evid, gate.Evidence.mk.injEq, true_and]
    have hc : c.val ≠ [] := by
      intro h0
      apply hne
      rw [he]
      simp [Slice.length, h0]
    constructor
    · cases b <;> simp_all [alloc.vec.Vec.deref, Slice.from_val]
    · cases b2 <;> simp_all [alloc.vec.Vec.deref, Slice.from_val]

theorem judge_loop_spec (rs : Slice gate.Recorded) (d c : Slice U8)
    (out : alloc.vec.Vec gate.Evidence) (i : Usize) (hi : i.val ≤ rs.val.length)
    (hout : out.val = (rs.val.take i.val).map (evid d.val c.val)) :
    gate.judge_loop rs d c out i ⦃ r => r.val = rs.val.map (evid d.val c.val) ⦄ := by
  unfold gate.judge_loop
  apply loop.spec_decr_nat (fun p : alloc.vec.Vec gate.Evidence × Usize => rs.val.length - p.2.val)
    (fun p : alloc.vec.Vec gate.Evidence × Usize => p.2.val ≤ rs.val.length ∧
      p.1.val = (rs.val.take p.2.val).map (evid d.val c.val))
  · rintro ⟨out, i⟩ ⟨hi, hout⟩
    unfold gate.judge_loop.body
    dsimp only at hi hout ⊢
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with evidence_spec as ⟨ e, he ⟩
      have hlen : out.val.length < Usize.max := by
        rw [hout]; simp; scalar_tac
      apply WP.spec_bind (alloc.vec.Vec.push_spec _ _ hlen)
      intro out1 hout1
      apply WP.spec_bind (Usize.add_spec (by scalar_tac))
      intro i' hi'
      simp only [WP.spec_ok]
      have hi1 : i'.val = i.val + 1 := by scalar_tac
      and_intros
      all_goals try scalar_tac
      rw [hout1, hout, hi1, he, hx, List.take_add_one,
        List.getElem?_eq_getElem (show i.val < rs.val.length by scalar_tac)]
      simp only [Option.toList, List.map_append, List.map_cons, List.map_nil]
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = rs.val.length := by scalar_tac
      rw [hout, this, List.take_length]
  · exact ⟨hi, hout⟩

/-- `judge` is `state` over each receipt compared with the present digests. -/
theorem judge_spec (rs : Slice gate.Recorded) (d c : Slice U8) :
    gate.judge rs d c ⦃ r => StateSpec (rs.val.map (evid d.val c.val)) r ⦄ := by
  unfold gate.judge
  apply WP.spec_bind (judge_loop_spec rs d c (alloc.vec.Vec.new gate.Evidence) 0#usize
    (by simp) (by simp))
  intro all hall
  apply WP.spec_mono (state_spec (alloc.vec.Vec.deref all))
  intro r hr
  simpa [alloc.vec.Vec.deref, Slice.from_val, hall] using hr

/-- The gate is `satisfied` exactly when the newest receipt about the present
definition and the present, non-empty content passed. -/
theorem judge_satisfied_iff (rs : Slice gate.Recorded) (d c : Slice U8) :
    gate.judge rs d c ⦃ r => r.1 = .Satisfied ↔
      ∃ i, ∃ h : i < rs.val.length, About d.val c.val rs.val[i] ∧
        (∀ j (hj : j < rs.val.length), i < j → ¬ About d.val c.val rs.val[j]) ∧
        rs.val[i].outcome = .Pass ⦄ := by
  unfold gate.judge
  apply WP.spec_bind (judge_loop_spec rs d c (alloc.vec.Vec.new gate.Evidence) 0#usize
    (by simp) (by simp))
  intro all hall
  apply WP.spec_mono (state_satisfied_iff (alloc.vec.Vec.deref all))
  intro r hr
  have hv : (alloc.vec.Vec.deref all).val = rs.val.map (evid d.val c.val) := by
    simpa [alloc.vec.Vec.deref, Slice.from_val] using hall
  rw [hr]
  constructor
  · rintro ⟨i, h, hcur, hlater, hpass⟩
    have hi : i < rs.val.length := by simpa [hv] using h
    refine ⟨i, hi, ?_, ?_, ?_⟩
    · have := hcur; simp only [hv, List.getElem_map] at this
      exact (isCurrent_evid _ _ _).mp this
    · intro j hj hij habout
      have := hlater j (by simpa [hv] using hj) hij
      simp only [hv, List.getElem_map] at this
      rw [(isCurrent_evid _ _ _).mpr habout] at this
      simp at this
    · have := hpass; simp only [hv, List.getElem_map] at this
      simpa [evid] using this
  · rintro ⟨i, hi, habout, hlater, hpass⟩
    refine ⟨i, by simpa [hv] using hi, ?_, ?_, ?_⟩
    · simp only [hv, List.getElem_map]
      exact (isCurrent_evid _ _ _).mpr habout
    · intro j hj hij
      have hj' : j < rs.val.length := by simpa [hv] using hj
      simp only [hv, List.getElem_map]
      cases hb : isCurrent (evid d.val c.val rs.val[j])
      · rfl
      · exact absurd ((isCurrent_evid _ _ _).mp hb) (hlater j hj' hij)
    · simp only [hv, List.getElem_map]
      simpa [evid] using hpass

/-- Receipts about other bytes never satisfy, whatever outcome they record:
with no receipt about the present content the state is `stale` (or `open`). -/
theorem judge_other_bytes (rs : Slice gate.Recorded) (d c : Slice U8)
    (h : ∀ r ∈ rs.val, r.content.val ≠ c.val) :
    gate.judge rs d c ⦃ r => r.1 ≠ .Satisfied ⦄ := by
  apply WP.spec_mono (judge_satisfied_iff rs d c)
  intro r hr hs
  obtain ⟨i, hi, ⟨_, hc, _⟩, _, _⟩ := hr.mp hs
  exact h _ (List.getElem_mem hi) hc

/-- With no readable content (an empty digest), no receipt satisfies. -/
theorem judge_empty_content (rs : Slice gate.Recorded) (d c : Slice U8) (h : c.val = []) :
    gate.judge rs d c ⦃ r => r.1 ≠ .Satisfied ⦄ := by
  apply WP.spec_mono (judge_satisfied_iff rs d c)
  intro r hr hs
  obtain ⟨i, hi, ⟨_, _, hc, _⟩, _, _⟩ := hr.mp hs
  exact hc h

/-! ## `stands` and `statements` -/

/-- A statement is about the present definition and content, and bound to
this run and subject. -/
def AboutS (d c : List U8) (f : gate.Said) : Prop :=
  f.definition.val = d ∧ f.content.val = c ∧ f.bound = true

theorem stands_spec (f : gate.Said) (a : Bool) (d c : Slice U8) :
    gate.stands f a d c ⦃ b => b = true ↔ f.attested = a ∧ AboutS d.val c.val f ⦄ := by
  unfold gate.stands
  split
  · rename_i hne
    simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_and]
    intro heq
    simp [heq] at hne
  · rename_i heq
    have ha : f.attested = a := by simpa using heq
    split
    · rename_i hb
      apply WP.spec_mono (about_spec _ _ d c)
      intro b hab
      simp [hab, ha, hb, AboutS, alloc.vec.Vec.deref, Slice.from_val]
    · rename_i hb
      simp [AboutS, hb]

/-- An attestation counts as some attestation only when it is bound. -/
theorem attests_spec (f : gate.Said) :
    gate.attests f ⦃ b => b = (f.attested && f.bound) ⦄ := by
  unfold gate.attests
  cases f.attested <;> simp

/-- The three flags over the first `i` statements. -/
def Flags (fs : List gate.Said) (d c : List U8) (i : Nat) (ac aa wc : Bool) : Prop :=
  (ac = true ↔ ∃ j, ∃ hj : j < fs.length, j < i ∧ fs[j].attested = true ∧ AboutS d c fs[j]) ∧
  (aa = true ↔ ∃ j, ∃ hj : j < fs.length, j < i ∧ fs[j].attested = true ∧ fs[j].bound = true) ∧
  (wc = true ↔ ∃ j, ∃ hj : j < fs.length, j < i ∧ fs[j].attested = false ∧ AboutS d c fs[j])

theorem statements_loop_spec (fs : Slice gate.Said) (d c : Slice U8) (ac aa wc : Bool)
    (i : Usize) (hi : i.val ≤ fs.val.length) (hf : Flags fs.val d.val c.val i.val ac aa wc) :
    gate.statements_loop fs d c ac aa wc i ⦃ r =>
      Flags fs.val d.val c.val fs.val.length r.1 r.2.1 r.2.2 ⦄ := by
  unfold gate.statements_loop
  apply loop.spec_decr_nat (fun p : Bool × Bool × Bool × Usize => fs.val.length - p.2.2.2.val)
    (fun p : Bool × Bool × Bool × Usize => p.2.2.2.val ≤ fs.val.length ∧
      Flags fs.val d.val c.val p.2.2.2.val p.1 p.2.1 p.2.2.1)
  · rintro ⟨ac, aa, wc, i⟩ ⟨hi, hac, haa, hwc⟩
    unfold gate.statements_loop.body
    dsimp only at hi hac haa hwc ⊢
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      have hil : i.val < fs.val.length := by scalar_tac
      subst hx
      have hif : ∀ (c v : Bool), (if c = true then ok true else ok v : Result Bool) = ok (c || v) := by
        intro c v; cases c <;> rfl
      step with attests_spec as ⟨ ba, hba ⟩
      simp only [hif, bind_tc_ok]
      step with stands_spec as ⟨ b, hb ⟩
      step with stands_spec as ⟨ b1, hb1 ⟩
      apply WP.spec_bind (Usize.add_spec (by scalar_tac))
      intro i' hi'
      simp only [WP.spec_ok]
      have hi1 : i'.val = i.val + 1 := by scalar_tac
      refine ⟨by scalar_tac, ⟨?_, ?_, ?_⟩, by scalar_tac⟩
      · rw [hi1, Bool.or_eq_true]
        constructor
        · rintro (h | h)
          · exact ⟨i.val, hil, by omega, (hb.mp h).1, (hb.mp h).2⟩
          · obtain ⟨j, hj, hji, hja, hjab⟩ := hac.mp h
            exact ⟨j, hj, by omega, hja, hjab⟩
        · rintro ⟨j, hj, hji, hja, hjab⟩
          by_cases hjeq : j = i.val
          · subst hjeq; exact Or.inl (hb.mpr ⟨hja, hjab⟩)
          · exact Or.inr (hac.mpr ⟨j, hj, by omega, hja, hjab⟩)
      · rw [hi1, Bool.or_eq_true, hba]
        constructor
        · rintro (h | h)
          · simp only [Bool.and_eq_true] at h
            exact ⟨i.val, hil, by omega, h.1, h.2⟩
          · obtain ⟨j, hj, hji, hja, hjb⟩ := haa.mp h
            exact ⟨j, hj, by omega, hja, hjb⟩
        · rintro ⟨j, hj, hji, hja, hjb⟩
          by_cases hjeq : j = i.val
          · subst hjeq; exact Or.inl (by simp [hja, hjb])
          · exact Or.inr (haa.mpr ⟨j, hj, by omega, hja, hjb⟩)
      · rw [hi1, Bool.or_eq_true]
        constructor
        · rintro (h | h)
          · exact ⟨i.val, hil, by omega, (hb1.mp h).1, (hb1.mp h).2⟩
          · obtain ⟨j, hj, hji, hja, hjab⟩ := hwc.mp h
            exact ⟨j, hj, by omega, hja, hjab⟩
        · rintro ⟨j, hj, hji, hja, hjab⟩
          by_cases hjeq : j = i.val
          · subst hjeq; exact Or.inl (hb1.mpr ⟨hja, hjab⟩)
          · exact Or.inr (hwc.mpr ⟨j, hj, by omega, hja, hjab⟩)
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = fs.val.length := by scalar_tac
      rw [← this]
      exact ⟨hac, haa, hwc⟩
  · exact ⟨hi, hf⟩

/-- An attestation or a waiver stands exactly when one is bound and about the
present definition and content; `attested_any` is whether any bound
attestation exists. -/
theorem statements_spec (fs : Slice gate.Said) (d c : Slice U8) :
    gate.statements fs d c ⦃ s =>
      (s.attested_current = true ↔ ∃ f ∈ fs.val, f.attested = true ∧ AboutS d.val c.val f) ∧
      (s.waived_current = true ↔ ∃ f ∈ fs.val, f.attested = false ∧ AboutS d.val c.val f) ∧
      (s.attested_any = true ↔ ∃ f ∈ fs.val, f.attested = true ∧ f.bound = true) ⦄ := by
  unfold gate.statements
  apply WP.spec_bind (statements_loop_spec fs d c false false false 0#usize (by simp)
    (by simp [Flags]))
  rintro ⟨ac, aa, wc⟩ ⟨hac, haa, hwc⟩
  have mem : ∀ (P : gate.Said → Prop),
      (∃ j, ∃ hj : j < fs.val.length, j < fs.val.length ∧ P fs.val[j]) ↔ ∃ f ∈ fs.val, P f := by
    intro P
    constructor
    · rintro ⟨j, hj, _, hp⟩; exact ⟨_, List.getElem_mem hj, hp⟩
    · rintro ⟨f, hf, hp⟩
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hf
      exact ⟨j, hj, hj, hp⟩
  dsimp only at hac haa hwc
  show (ok ({ attested_current := ac, attested_any := aa, waived_current := wc } : gate.Statements)) ⦃ _ ⦄
  simp only [WP.spec_ok]
  refine ⟨?_, ?_, ?_⟩
  · rw [hac]; exact mem (fun f => f.attested = true ∧ AboutS d.val c.val f)
  · rw [hwc]; exact mem (fun f => f.attested = false ∧ AboutS d.val c.val f)
  · rw [haa]; exact mem (fun f => f.attested = true ∧ f.bound = true)

/-! ## The composed claim for one gate -/

/-- A standing is met: satisfied by a check, attested or waived (`met_spec`). -/
def Met (x : gate.Standing) : Prop :=
  x = .Checked .Satisfied ∨ x = .Attested ∨ x = .Waived

/-- `Met` as a function. -/
def metB : gate.Standing → Bool
  | .Checked .Satisfied => true
  | .Attested => true
  | .Waived => true
  | _ => false

theorem metB_iff (x : gate.Standing) : metB x = true ↔ Met x := by
  rcases x with s | _ | _
  · cases s <;> simp [metB, Met]
  · simp [metB, Met]
  · simp [metB, Met]

theorem standingOf_met (ac : Bool) (s : gate.State) (st : gate.Statements) :
    Met (standingOf ac s st) ↔
      if ac then st.attested_current = true ∨ st.waived_current = true
      else s = .Satisfied ∨ st.waived_current = true := by
  unfold standingOf Met
  rcases st with ⟨a, b, c⟩
  cases ac <;> cases s <;> cases a <;> cases b <;> cases c <;> simp

/-- Whatever receipts and statements exist, the standing of an obligation is
met only on evidence about the present digests: a checked obligation's
newest receipt about the present definition and (non-empty) content passed;
or an attestation about them stands for an attested-class obligation; or a
waiver about them stands. -/
theorem met_only_when_current (rs : Slice gate.Recorded) (fs : Slice gate.Said)
    (ac : Bool) (d c : Slice U8) :
    (do let r ← gate.judge rs d c
        let st ← gate.statements fs d c
        gate.standing ac r.1 st) ⦃ x => Met x →
      (ac = false ∧ ∃ i, ∃ h : i < rs.val.length, About d.val c.val rs.val[i] ∧
        (∀ j (hj : j < rs.val.length), i < j → ¬ About d.val c.val rs.val[j]) ∧
        rs.val[i].outcome = .Pass) ∨
      (ac = true ∧ ∃ f ∈ fs.val, f.attested = true ∧ AboutS d.val c.val f) ∨
      (∃ f ∈ fs.val, f.attested = false ∧ AboutS d.val c.val f) ⦄ := by
  step with judge_satisfied_iff as ⟨ r, hr ⟩
  step with statements_spec as ⟨ st, hac, hwc, _ ⟩
  apply WP.spec_mono (standing_spec ac r.1 st)
  intro x hx hmet
  rw [hx, standingOf_met] at hmet
  cases ac
  · simp only [Bool.false_eq_true, ↓reduceIte] at hmet
    rcases hmet with hs | hw
    · exact Or.inl ⟨rfl, hr.mp hs⟩
    · exact Or.inr (Or.inr (hwc.mp hw))
  · simp only [↓reduceIte] at hmet
    rcases hmet with ha | hw
    · exact Or.inr (Or.inl ⟨rfl, hac.mp ha⟩)
    · exact Or.inr (Or.inr (hwc.mp hw))

end Writfold.Core.Gate

namespace Writfold.Core.Plan

/-! ## `fact_met`, `met_all`, `advance` -/

/-- Met under a policy: a met standing the policy accepts (`Policy.met_spec`). -/
def PMet (x : gate.Standing) (a : policy.Accept) : Prop :=
  (x = .Checked .Satisfied ∧ a.satisfied = true) ∨
  (x = .Attested ∧ a.attested = true) ∨ (x = .Waived ∧ a.waived = true)

/-- `PMet` as a function. -/
def pmetB (x : gate.Standing) (a : policy.Accept) : Bool :=
  match x with
  | .Checked .Satisfied => a.satisfied
  | .Attested => a.attested
  | .Waived => a.waived
  | _ => false

theorem pmetB_iff (x : gate.Standing) (a : policy.Accept) : pmetB x a = true ↔ PMet x a := by
  rcases x with st | _ | _
  · cases st <;> simp [pmetB, PMet]
  · simp [pmetB, PMet]
  · simp [pmetB, PMet]

/-- Met under a policy is met: a policy only narrows. -/
theorem PMet.met {x : gate.Standing} {a : policy.Accept} (h : PMet x a) : Gate.Met x := by
  rcases h with ⟨h, _⟩ | ⟨h, _⟩ | ⟨h, _⟩ <;> simp [Gate.Met, h]

/-- `fact_met` on the model. -/
def factMetB : plan.GateFact → Bool
  | .Undefined => false
  | .Stands s a => pmetB s a

theorem fact_met_spec (g : plan.GateFact) :
    plan.fact_met g ⦃ b => b = factMetB g ⦄ := by
  unfold plan.fact_met
  cases g with
  | Undefined => simp [factMetB]
  | Stands s a =>
    simp only
    apply WP.spec_mono (Policy.met_spec s a)
    intro b hb
    have hm := pmetB_iff s a
    simp only [factMetB]
    have hb' : b = true ↔ PMet s a := hb
    cases hmb : pmetB s a <;> cases b <;> simp_all

theorem factMetB_iff (g : plan.GateFact) :
    factMetB g = true ↔ ∃ s a, g = .Stands s a ∧ PMet s a := by
  cases g <;> simp [factMetB, pmetB_iff]

theorem met_all_loop_spec (gs : Slice plan.GateFact) (out : alloc.vec.Vec Bool) (i : Usize)
    (hi : i.val ≤ gs.val.length) (hout : out.val = (gs.val.take i.val).map factMetB) :
    plan.met_all_loop gs out i ⦃ r => r.val = gs.val.map factMetB ⦄ := by
  unfold plan.met_all_loop
  apply loop.spec_decr_nat (fun p : alloc.vec.Vec Bool × Usize => gs.val.length - p.2.val)
    (fun p : alloc.vec.Vec Bool × Usize => p.2.val ≤ gs.val.length ∧
      p.1.val = (gs.val.take p.2.val).map factMetB)
  · rintro ⟨out, i⟩ ⟨hi, hout⟩
    unfold plan.met_all_loop.body
    dsimp only at hi hout ⊢
    split
    · rename_i hlt
      step as ⟨ x, hx ⟩
      step with fact_met_spec as ⟨ b, hb ⟩
      have hlen : out.val.length < Usize.max := by
        rw [hout]; simp; scalar_tac
      apply WP.spec_bind (alloc.vec.Vec.push_spec _ _ hlen)
      intro out1 hout1
      apply WP.spec_bind (Usize.add_spec (by scalar_tac))
      intro i' hi'
      simp only [WP.spec_ok]
      have hi1 : i'.val = i.val + 1 := by scalar_tac
      and_intros
      all_goals try scalar_tac
      rw [hout1, hout, hi1, hb, hx, List.take_add_one,
        List.getElem?_eq_getElem (show i.val < gs.val.length by scalar_tac)]
      simp only [Option.toList, List.map_append, List.map_cons, List.map_nil]
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = gs.val.length := by scalar_tac
      rw [hout, this, List.take_length]
  · exact ⟨hi, hout⟩

theorem met_all_spec (gs : Slice plan.GateFact) :
    plan.met_all gs ⦃ r => r.val = gs.val.map factMetB ⦄ :=
  met_all_loop_spec gs (alloc.vec.Vec.new Bool) 0#usize (by simp) (by simp)

/-- `advance` is `advance_check` on each gate's `fact_met`. -/
theorem advance_facts_spec (nodes : Slice plan.Node) (pos : Usize) (gs : Slice plan.GateFact) :
    plan.advance nodes pos gs ⦃ r => AdvanceSpec nodes.val pos.val (gs.val.map factMetB) r ⦄ := by
  unfold plan.advance
  step with met_all_spec as ⟨ v, hv ⟩
  apply WP.spec_mono (advance_check_spec nodes pos (alloc.vec.Vec.deref v))
  intro r hr
  simpa [alloc.vec.Vec.deref, Slice.from_val, hv] using hr

/-- A step is recorded done exactly when it is the current step and every
gate it names has a usable definition and stands met under its policy
(satisfied, attested or waived, each only where the gate accepts it). A
step with no gates owes nothing. -/
theorem advance_facts_ok_iff (nodes : Slice plan.Node) (pos : Usize) (gs : Slice plan.GateFact) :
    plan.advance nodes pos gs ⦃ r => r = .Ok () ↔
      cur nodes.val = some pos.val ∧ ∀ g ∈ gs.val, ∃ s a, g = .Stands s a ∧ PMet s a ⦄ := by
  unfold plan.advance
  step with met_all_spec as ⟨ v, hv ⟩
  apply WP.spec_mono (advance_ok_iff nodes pos (alloc.vec.Vec.deref v))
  intro r hr
  rw [hr]
  have hv' : (alloc.vec.Vec.deref v).val = gs.val.map factMetB := by
    simpa [alloc.vec.Vec.deref, Slice.from_val] using hv
  rw [hv']
  simp only [List.mem_map, forall_exists_index, and_imp, forall_apply_eq_imp_iff₂]
  simp only [factMetB_iff]

/-- A gate with no usable definition is never met, so the step is refused. -/
theorem advance_undefined_refused (nodes : Slice plan.Node) (pos : Usize)
    (gs : Slice plan.GateFact) (h : plan.GateFact.Undefined ∈ gs.val) :
    plan.advance nodes pos gs ⦃ r => r ≠ .Ok () ⦄ := by
  apply WP.spec_mono (advance_facts_ok_iff nodes pos gs)
  intro r hr hok
  obtain ⟨s, a, hs, _⟩ := (hr.mp hok).2 _ h
  cases hs

/-! ## The assurance sentence, end to end -/

/-- A step is recorded as done only when every gate it names has a usable
definition and stands met on evidence about the present digests (and, by
`PMet.met`, under a policy that only narrows). `judged k` is what the caller
computed for gate `k`: when gate `k` stands, its standing is the kernels'
(`judge`, `statements`, `standing`) on that gate's receipts, statements and
present digests. -/
theorem done_only_when_met (nodes : Slice plan.Node) (pos : Usize) (gs : Slice plan.GateFact)
    (rs : Nat → Slice gate.Recorded) (fs : Nat → Slice gate.Said) (ac : Nat → Bool)
    (d c : Nat → Slice U8)
    (judged : ∀ k (hk : k < gs.val.length) s a, gs.val[k]'hk = .Stands s a →
      (do let r ← gate.judge (rs k) (d k) (c k)
          let st ← gate.statements (fs k) (d k) (c k)
          gate.standing (ac k) r.1 st) = ok s) :
    plan.advance nodes pos gs ⦃ r => r = .Ok () → ∀ k (hk : k < gs.val.length),
      (ac k = false ∧ ∃ i, ∃ h : i < (rs k).val.length,
        Gate.About (d k).val (c k).val (rs k).val[i] ∧
        (∀ j (hj : j < (rs k).val.length), i < j → ¬ Gate.About (d k).val (c k).val (rs k).val[j]) ∧
        (rs k).val[i].outcome = .Pass) ∨
      (ac k = true ∧ ∃ f ∈ (fs k).val, f.attested = true ∧ Gate.AboutS (d k).val (c k).val f) ∨
      (∃ f ∈ (fs k).val, f.attested = false ∧ Gate.AboutS (d k).val (c k).val f) ⦄ := by
  apply WP.spec_mono (advance_facts_ok_iff nodes pos gs)
  intro r hr hok k hk
  obtain ⟨s, a, hs, hmet⟩ := (hr.mp hok).2 _ (List.getElem_mem hk)
  have hcomp := Gate.met_only_when_current (rs k) (fs k) (ac k) (d k) (c k)
  rw [judged k hk s a hs, WP.spec_ok] at hcomp
  exact hcomp hmet.met

end Writfold.Core.Plan
