import Writfold.Conform.Words
import Writfold.Gate
import Writfold.Current
import Writfold.Checklist
import Writfold.Delivery

/-!
# Links: the evidence kernels (#210, calculus.md 3.2 to 3.4)

A gate's checked state, its standing, a checklist's and a delivery's
standing, and the records that decide them are shown to be the core's
`stand`, `met` and `deciding`. Each link is derived from the kernel's
existing spec theorem; no kernel is unfolded here.

Where the core is coarser than the kernel the dictionary collapses and the
kernel's own spec keeps the detail: `gate.State`'s `open`, `stale` and
`failed` are all the core's `no` (`stateTri`), and a delivery's or a
checklist's standing is linked through the gate's.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Conform

/-! ## The dictionary -/

/-- A gate's checked state as the core's three answers: `satisfied` is
yes, `unknown` unknown; `open`, `stale` and `failed` are the core's no. -/
def stateTri : gate.State → Calculus.Tri
  | .Satisfied => .yes
  | .Unknown => .unknown
  | _ => .no

theorem stateTri_onto (t : Calculus.Tri) : ∃ s, stateTri s = t := by
  cases t
  · exact ⟨.Satisfied, rfl⟩
  · exact ⟨.Failed, rfl⟩
  · exact ⟨.Unknown, rfl⟩

/-- A checked state is the standing a checked record about the present
reaches (calculus.md 3.3). -/
theorem gate_checked_stand (s : gate.State) :
    standingOf (.Checked s) = Calculus.stand .checked true (stateTri s) := by
  cases s <;> rfl

/-- A policy's verdict on a standing is the core's `met`. -/
theorem pmetB_calculus_met (x : gate.Standing) (a : policy.Accept) :
    Core.Plan.pmetB x a = Calculus.met (standingOf x) (acceptOf a) := by
  rcases x with s | _ | _
  · cases s <;> rfl
  · rfl
  · rfl

/-! ## The deciding record over positions -/

/-- The newest member satisfying `c` is the core's deciding record over
the members numbered by position. -/
theorem evDeciding_last {α : Type} (c : α → Bool) (l : List α) (i : Nat) (hi : i < l.length)
    (hc : c l[i] = true) (hl : ∀ j (hj : j < l.length), i < j → c l[j] = false) :
    Calculus.deciding Prod.snd (fun p => c p.1) l.zipIdx = some (l[i], i) := by
  have hmem : (l[i], i) ∈ l.zipIdx := List.mem_zipIdx_iff_getElem?.mpr (by simp [hi])
  cases hd : Calculus.deciding Prod.snd (fun p => c p.1) l.zipIdx with
  | none =>
    have := (Calculus.deciding_none_iff _ _ _).mp hd _ hmem
    simp [hc] at this
  | some p =>
    obtain ⟨y, j⟩ := p
    obtain ⟨hm, hcy⟩ := Calculus.deciding_mem _ _ _ _ hd
    obtain ⟨_, hjl, hy⟩ := List.mem_zipIdx hm
    simp only [Nat.zero_add, Nat.sub_zero] at hjl hy
    have hnew := Calculus.deciding_is_newest _ _ _ _ hd _ hmem (by simpa using hc)
    simp only at hnew
    have hcy' : c l[j] = true := by rw [← hy]; simpa using hcy
    have hji : j = i := by
      by_contra hne
      have := hl j hjl (by omega)
      rw [hcy'] at this
      exact Bool.noConfusion this
    subst hji
    rw [hy]

/-- No member counts: nothing decides. -/
theorem evDeciding_none {α : Type} (c : α → Bool) (l : List α)
    (h : ∀ x ∈ l, c x = false) :
    Calculus.deciding Prod.snd (fun p => c p.1) l.zipIdx = none := by
  rw [Calculus.deciding_none_iff]
  intro r hr
  obtain ⟨_, hjl, hy⟩ := List.mem_zipIdx hr
  simp only [Nat.zero_add, Nat.sub_zero] at hjl hy
  rw [hy]
  exact h _ (List.getElem_mem _)

/-- What the deciding receipt says, or the core's no when none decides. -/
def gateDecidedTri (l : List gate.Evidence) : Calculus.Tri :=
  ((Calculus.deciding Prod.snd (fun p => Core.Gate.isCurrent p.1) l.zipIdx).map
    (fun p => outcomeTri p.1.outcome)).getD .no

theorem gate_stateSpec_decided (l : List gate.Evidence) (s : gate.State) (o : Option Usize)
    (h : Core.Gate.StateSpec l (s, o)) : stateTri s = gateDecidedTri l := by
  unfold gateDecidedTri
  cases o with
  | none =>
    obtain ⟨hnone, rfl⟩ := h
    rw [evDeciding_none]
    · rfl
    · intro e he
      cases hc : Core.Gate.isCurrent e
      · rfl
      · exact absurd (Core.Gate.isCurrent_bound hc) (by simp [hnone e he])
  | some i =>
    rcases h with ⟨hi, hcur, hlater, rfl⟩ | ⟨hall, _, rfl⟩
    · rw [evDeciding_last _ _ _ hi hcur hlater]
      simp only [Option.map_some, Option.getD_some]
      cases l[i.val].outcome <;> rfl
    · rw [evDeciding_none _ _ hall]
      rfl

/-! ## `gate`: the deciding receipt, the standing, `met` -/

/-- 3.2 at the gate: the newest current receipt decides; with none, the
answer is the core's no (`open` or `stale`). -/
theorem gate_state_implements (rs : Slice gate.Evidence) :
    gate.state rs ⦃ r => stateTri r.1 =
      ((Calculus.deciding Prod.snd (fun p => Core.Gate.isCurrent p.1) rs.val.zipIdx).map
        (fun p => outcomeTri p.1.outcome)).getD .no ⦄ := by
  apply WP.spec_mono (Core.Gate.state_spec rs)
  rintro ⟨s, o⟩ hr
  exact gate_stateSpec_decided _ s o hr

/-- `judge` is `state` over each receipt's evidence about the present
definition and content. -/
theorem gate_judge_implements (rs : Slice gate.Recorded) (d c : Slice U8) :
    gate.judge rs d c ⦃ r => stateTri r.1 =
      ((Calculus.deciding Prod.snd (fun p => Core.Gate.isCurrent p.1)
        (rs.val.map (Core.Gate.evid d.val c.val)).zipIdx).map
        (fun p => outcomeTri p.1.outcome)).getD .no ⦄ := by
  apply WP.spec_mono (Core.Gate.judge_spec rs d c)
  rintro ⟨s, o⟩ hr
  exact gate_stateSpec_decided _ s o hr

/-- The newest bound receipt is the deciding record of the bound ones. -/
theorem gate_newest_bound_implements (rs : Slice gate.Evidence) :
    gate.newest_bound rs ⦃ r => r.map (·.val) =
      (Calculus.deciding Prod.snd (fun p => p.1.bound) rs.val.zipIdx).map Prod.snd ⦄ := by
  apply WP.spec_mono (Core.Gate.newest_bound_spec rs)
  intro r hr
  cases r with
  | none =>
    rw [evDeciding_none _ _ hr]
    rfl
  | some i =>
    obtain ⟨hi, hb, hlater⟩ := hr
    rw [evDeciding_last (fun e : gate.Evidence => e.bound) _ _ hi hb hlater]
    rfl

/-- `met` is the core's `met` under the widest policy. -/
theorem gate_met_implements (x : gate.Standing) :
    gate.Standing.met x ⦃ b => b = Calculus.met (standingOf x) Calculus.Accept.top ⦄ := by
  apply WP.spec_mono (Core.Gate.met_spec x)
  intro b hb
  rw [Bool.eq_iff_iff, hb]
  rcases x with s | _ | _
  · cases s <;> simp [standingOf, Calculus.met, Calculus.Accept.top]
  · simp [standingOf, Calculus.met, Calculus.Accept.top]
  · simp [standingOf, Calculus.met, Calculus.Accept.top]

/-- 3.3 at the gate: an obligation stands on one record, and the standing
is the core's `stand` of that record's class. An attested-class obligation
stands on a current attestation, else a current waiver, else on nothing
about the present; any other on a satisfied check, else a current waiver,
else its checked state. -/
theorem gate_standing_implements (ac : Bool) (s : gate.State) (st : gate.Statements) :
    gate.standing ac s st ⦃ x => standingOf x =
      if ac then
        (if st.attested_current then Calculus.stand .attested true .yes
         else if st.waived_current then Calculus.stand .waived true .yes
         else Calculus.stand .attested false .yes)
      else if stateTri s = .yes then Calculus.stand .checked true .yes
      else if st.waived_current then Calculus.stand .waived true .yes
      else Calculus.stand .checked true (stateTri s) ⦄ := by
  apply WP.spec_mono (Core.Gate.standing_spec ac s st)
  intro x hx
  subst hx
  rcases st with ⟨a, b, c⟩
  cases ac <;> cases s <;> cases a <;> cases b <;> cases c <;> rfl

/-! ## `checklist` -/

/-- A checklist's checked state never stands: a checklist is judged by
attestation, never by a check. -/
theorem checklist_state_implements (r : checklist.Rollup) :
    checklist.state r ⦃ s => Calculus.stand .checked true (stateTri s) = .unmet ⦄ := by
  apply WP.spec_mono (Core.Checklist.state_spec r)
  intro s hs
  subst hs
  cases r.stale <;> rfl

/-- A checklist stands on its cover (an attestation about the present),
else a current waiver, else on nothing about the present. -/
theorem checklist_standing_implements (r : checklist.Rollup) (w : Bool) :
    checklist.standing r w ⦃ s => standingOf s =
      if r.covered then Calculus.stand .attested true .yes
      else if w then Calculus.stand .waived true .yes
      else Calculus.stand .attested false .yes ⦄ := by
  apply WP.spec_mono (Core.Checklist.standing_spec r w)
  intro s hs
  subst hs
  rcases r with ⟨c, st, f⟩
  cases c <;> cases st <;> cases w <;> rfl

/-! ## `delivery` -/

/-- A candidate counts unless withdrawn. -/
def placeLive : delivery.Place → Bool
  | .Withdrawn => false
  | _ => true

/-- The deciding candidate, when it is open. -/
def placeOpenAt : Option (delivery.Place × Nat) → Option Nat
  | some (.Open, i) => some i
  | _ => none

/-- 3.2 at a delivery: the newest candidate not withdrawn decides; the
kernel names it when it is open. -/
theorem delivery_deciding_implements (ps : Slice delivery.Place) :
    delivery.deciding ps ⦃ r => r.map (·.val) =
      placeOpenAt (Calculus.deciding Prod.snd (fun p => placeLive p.1) ps.val.zipIdx) ⦄ := by
  apply WP.spec_mono (Core.Delivery.deciding_spec ps)
  intro r hr
  cases r with
  | some i =>
    obtain ⟨hi, ho, hlater⟩ := hr
    rw [evDeciding_last placeLive _ _ hi (by rw [ho]; rfl)
      (fun j hj hij => by rw [hlater j hj hij]; rfl)]
    simp [placeOpenAt, ho]
  | none =>
    simp only [Option.map_none]
    cases hd : Calculus.deciding Prod.snd (fun p => placeLive p.1) ps.val.zipIdx with
    | none => rfl
    | some p =>
      obtain ⟨y, j⟩ := p
      obtain ⟨hm, hl⟩ := Calculus.deciding_mem _ _ _ _ hd
      obtain ⟨_, hjl, hy⟩ := List.mem_zipIdx hm
      simp only [Nat.zero_add, Nat.sub_zero] at hjl hy
      rw [hy] at hl ⊢
      cases hpj : ps.val[j] with
      | Open =>
        obtain ⟨k, hk, hjk, hkw⟩ := hr j hjl hpj
        have hmk : (ps.val[k], k) ∈ ps.val.zipIdx :=
          List.mem_zipIdx_iff_getElem?.mpr (by simp [hk])
        have hlk : placeLive ps.val[k] = true := by
          cases hpk : ps.val[k] <;> simp_all [placeLive]
        have := Calculus.deciding_is_newest _ _ _ _ hd _ hmk hlk
        simp only at this
        omega
      | Bound => simp [placeOpenAt]
      | Withdrawn => simp [hpj, placeLive] at hl

/-- A delivery's standing: with nothing owed, nothing about the present;
else the standing of the first owed that its policy does not meet; else,
every owed met under its own policy, a standing met under any policy that
takes its class. -/
theorem delivery_standing_implements (os : Slice delivery.Owed) :
    delivery.standing os ⦃ s =>
      (os.val = [] ∧ standingOf s = Calculus.stand .checked false .yes) ∨
      (∃ o ∈ os.val, Calculus.met (standingOf o.standing) (acceptOf o.accept) = false ∧
        s = o.standing) ∨
      (os.val ≠ [] ∧ (∀ o ∈ os.val, Calculus.met (standingOf o.standing) (acceptOf o.accept) = true) ∧
        Calculus.met (standingOf s) Calculus.Accept.top = true) ⦄ := by
  apply WP.spec_mono (Core.Delivery.standing_spec os)
  intro s hs
  rcases hs with ⟨he, rfl⟩ | ⟨i, hi, hu, _, rfl⟩ | ⟨hne, hall, rfl⟩
  · exact Or.inl ⟨he, rfl⟩
  · refine Or.inr (Or.inl ⟨_, List.getElem_mem hi, ?_, rfl⟩)
    rw [← pmetB_calculus_met]; exact hu
  · refine Or.inr (Or.inr ⟨hne, fun o ho => ?_, ?_⟩)
    · rw [← pmetB_calculus_met]; exact hall o ho
    · unfold Core.Delivery.allMet
      split
      · rfl
      · split <;> rfl

/-- A post record is an attestation: it stands, about the present, only
when recorded. -/
theorem delivery_posted_implements (r : Bool) :
    delivery.posted r ⦃ s => standingOf s = Calculus.stand .attested r .yes ⦄ := by
  apply WP.spec_mono (Core.Delivery.posted_spec r)
  intro s hs
  subst hs
  cases r <;> rfl

/-- A satisfied check whose cited evidence moved is no longer about the
present; nothing else changes. -/
theorem delivery_rechecked_implements (c : gate.State) (moved : Usize) :
    delivery.rechecked c moved ⦃ s => standingOf (.Checked s) =
      Calculus.stand .checked (moved.val = 0 || stateTri c != .yes) (stateTri c) ⦄ := by
  apply WP.spec_mono (Core.Delivery.rechecked_spec c moved)
  intro s hs
  subst hs
  unfold Core.Delivery.recheckedOf
  by_cases h : moved.val = 0
  · cases c <;> simp [h, stateTri, standingOf, Calculus.stand]
  · have hp : 0 < moved.val := Nat.pos_of_ne_zero h
    cases c <;> simp [h, hp, stateTri, standingOf, Calculus.stand]

/-! ## The assurance sentence, on the core's words -/

/-- `done_only_when_met` with the core in its conclusion: a step is
recorded done only when every gate it names stands, about the present, on
a class no policy refuses (never `reported`), and its policy meets that
standing. -/
theorem done_only_when_met (nodes : Slice plan.Node) (pos : Usize) (gs : Slice plan.GateFact) :
    plan.advance nodes pos gs ⦃ r => r = .Ok () → ∀ g ∈ gs.val, ∃ s a, g = .Stands s a ∧
      Calculus.met (standingOf s) (acceptOf a) = true ∧
      ∃ c, c ≠ .reported ∧ standingOf s = Calculus.stand c true .yes ⦄ := by
  apply WP.spec_mono (Core.Plan.advance_facts_ok_iff nodes pos gs)
  intro r hr hok g hg
  obtain ⟨s, a, hs, hm⟩ := (hr.mp hok).2 g hg
  refine ⟨s, a, hs, ?_, ?_⟩
  · rw [← pmetB_calculus_met]; exact (Core.Plan.pmetB_iff s a).mpr hm
  · rcases hm with ⟨rfl, _⟩ | ⟨rfl, _⟩ | ⟨rfl, _⟩
    · exact ⟨.checked, by decide, rfl⟩
    · exact ⟨.attested, by decide, rfl⟩
    · exact ⟨.waived, by decide, rfl⟩

end Writfold.Core.Conform
