import Writfold.Conform.Evidence
import Writfold.Ground
import Writfold.Pin
import Writfold.Policy

/-!
# Links: grounding, pins and a policy's classes (#210, calculus.md 3.3)

A grounding is the standing a piece of evidence gives an answer: the
core's `stand` of the evidence's class, never a stronger one (law 3.3, a
class is never promoted; `waiver_never_grounds`). Pins are requirements
that all hold (`Tri.all`), and "nothing found" is an absence that needs
the whole of what was looked at (`search`, law U).

Too coarse a core, recorded here: a grounding has no `waived` word (a
waiver grounds nothing) and no `unknown` word (what cannot be judged does
not ground), so `groundingOf` reaches only `satisfied`, `attested` and
`unmet`.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core.Conform

/-! ## The dictionary -/

/-- A grounding as the standing it gives an answer. -/
def groundingOf : grounding.Grounding → Calculus.Standing
  | .Grounded => .satisfied
  | .Attested => .attested
  | .Ungrounded => .unmet

/-- Onto the standings evidence can give: a waiver never grounds and an
unjudged look grounds nothing, so `waived`, `reported` and `unknown` have
no grounding word. -/
theorem groundingOf_onto (s : Calculus.Standing)
    (h : s = .satisfied ∨ s = .attested ∨ s = .unmet) : ∃ g, groundingOf g = s := by
  rcases h with rfl | rfl | rfl
  · exact ⟨.Grounded, rfl⟩
  · exact ⟨.Attested, rfl⟩
  · exact ⟨.Ungrounded, rfl⟩

/-- How a done step's gate was met, as the class it stood on. -/
def howClass : policy.How → Calculus.Class
  | .Passed => .checked
  | .Attested => .attested
  | .Waived => .waived

/-- Onto every class a policy takes: no policy takes `reported` (law S). -/
theorem howClass_onto (c : Calculus.Class) (h : c ≠ .reported) : ∃ x, howClass x = c := by
  cases c
  · exact ⟨.Passed, rfl⟩
  · exact ⟨.Attested, rfl⟩
  · exact ⟨.Waived, rfl⟩
  · exact absurd rfl h

/-! ## Grounding is `stand`, never a stronger class -/

/-- 3.3 at a gate reference: the grounding has the gate's class, or none;
a waiver's class never grounds. -/
theorem ground_of_gate_implements (found : Bool) (s : gate.Standing) :
    ground.of_gate found s ⦃ g => Calculus.Standing.cls (groundingOf g) =
      if found then
        (match Calculus.Standing.cls (standingOf s) with
         | some .waived => none
         | c => c)
      else none ⦄ := by
  apply WP.spec_mono (Core.Ground.of_gate_spec found s)
  intro g hg
  subst hg
  unfold Core.Ground.ofGate
  cases found
  · rfl
  · rcases s with st | _ | _
    · cases st <;> rfl
    · rfl
    · rfl

/-- A receipt is a checked record: it grounds when about the present and
passed. -/
theorem ground_of_receipt_implements (f : ground.ReceiptFacts) :
    ground.of_receipt f ⦃ g => groundingOf g = Calculus.stand .checked
      (f.same_subject && f.same_definition && f.same_content && f.gate_met)
      (if f.passed then .yes else .no) ⦄ := by
  have h := WP.spec_and (Core.Ground.of_receipt_grounded_iff f)
    (Core.Ground.of_receipt_never_attested f)
  apply WP.spec_mono h
  rintro g ⟨hg, hna⟩
  rcases f with ⟨a, b, c, d, e⟩
  cases g
  · have := hg.mp rfl
    simp only at this
    obtain ⟨rfl, rfl, rfl, rfl, rfl⟩ := this
    rfl
  · exact absurd rfl hna
  · have hn : ¬ (a = true ∧ b = true ∧ c = true ∧ d = true ∧ e = true) := fun h' =>
      by have := hg.mpr h'; cases this
    cases a <;> cases b <;> cases c <;> cases d <;> cases e <;> simp_all <;> rfl

theorem ground_of_token_never_attested (n nf : Usize) :
    ground.of_token n nf ⦃ g => g ≠ .Attested ⦄ := by
  unfold ground.of_token
  split
  · split <;> simp
  · simp

/-- A citation token is a checked record: about the present when it names
exactly one source, passed when that source is fresh. -/
theorem ground_of_token_implements (n nf : Usize) :
    ground.of_token n nf ⦃ g => groundingOf g =
      Calculus.stand .checked (decide (n.val = 1)) (if nf.val = 0 then .yes else .no) ⦄ := by
  have h := WP.spec_and (Core.Ground.of_token_grounded_iff n nf) (ground_of_token_never_attested n nf)
  apply WP.spec_mono h
  rintro g ⟨hg, hna⟩
  cases g
  · obtain ⟨h1, h2⟩ := hg.mp rfl
    simp [h1, h2, groundingOf, Calculus.stand]
  · exact absurd rfl hna
  · have hn : ¬ (n.val = 1 ∧ nf.val = 0) := fun h' => by have := hg.mpr h'; cases this
    by_cases h1 : n.val = 1
    · have h2 : nf.val ≠ 0 := fun h2 => hn ⟨h1, h2⟩
      simp [h1, h2, groundingOf, Calculus.stand]
    · simp [h1, groundingOf, Calculus.stand]

theorem ground_of_observation_never_attested (s : observe.Status) (n nf : Usize) :
    ground.of_observation s n nf ⦃ g => g ≠ .Attested ⦄ := by
  unfold ground.of_observation
  split
  all_goals
    step with Core.Ground.grounds_iff as ⟨ b1, _ ⟩
    split <;> simp

/-- An `ok` look. -/
def statusOk : observe.Status → Bool
  | .Ok => true
  | _ => false

/-- An observation is a checked record: about the present when the look
was `ok` and embedded a citation, passed when every citation is fresh. -/
theorem ground_of_observation_implements (s : observe.Status) (n nf : Usize) :
    ground.of_observation s n nf ⦃ g => groundingOf g =
      Calculus.stand .checked (statusOk s && decide (0 < n.val)) (if nf.val = 0 then .yes else .no) ⦄ := by
  have h := WP.spec_and (Core.Ground.of_observation_grounded_iff s n nf)
    (ground_of_observation_never_attested s n nf)
  apply WP.spec_mono h
  rintro g ⟨hg, hna⟩
  cases g
  · obtain ⟨rfl, h1, h2⟩ := hg.mp rfl
    simp [h1, h2, statusOk, groundingOf, Calculus.stand]
  · exact absurd rfl hna
  · have hn : ¬ (s = .Ok ∧ 0 < n.val ∧ nf.val = 0) := fun h' => by have := hg.mpr h'; cases this
    by_cases hs : s = .Ok
    · subst hs
      by_cases h1 : 0 < n.val
      · have h2 : nf.val ≠ 0 := fun h2 => hn ⟨rfl, h1, h2⟩
        simp [h1, h2, statusOk, groundingOf, Calculus.stand]
      · simp [h1, statusOk, groundingOf, Calculus.stand]
    · have : statusOk s = false := by cases s <;> simp_all [statusOk]
      simp [this, groundingOf, Calculus.stand]

/-- An attestation is an attested record: it grounds only as an
attestation, and only when current. -/
theorem ground_of_statement_implements (c : Bool) :
    ground.of_statement c ⦃ g => groundingOf g = Calculus.stand .attested c .yes ⦄ := by
  apply WP.spec_mono (Core.Ground.of_statement_spec c)
  intro g hg
  subst hg
  cases c <;> rfl

/-- A worker's report is a checked record: about the present when found,
live and bound to its invocation, passed when ready; never an
attestation. -/
theorem ground_of_delivery_implements (found live ready bound : Bool) :
    ground.of_delivery found live ready bound ⦃ g => groundingOf g =
      Calculus.stand .checked (found && live && bound) (if ready then .yes else .no) ⦄ := by
  apply WP.spec_mono (Core.Ground.of_delivery_grounded_iff found live ready bound)
  rintro g ⟨hg, hna⟩
  cases g
  · obtain ⟨rfl, rfl, rfl, rfl⟩ := hg.mp rfl
    rfl
  · exact absurd rfl hna
  · have hn : ¬ (found = true ∧ live = true ∧ ready = true ∧ bound = true) := fun h' =>
      by have := hg.mpr h'; cases this
    cases found <;> cases live <;> cases ready <;> cases bound <;> simp_all <;> rfl

/-! ## Pins: requirements that all hold -/

/-- A pin as a requirement: held, or not. -/
def pinHeldTri (p : pin.Pin) : Calculus.Tri := if Core.Pin.heldB p then .yes else .no

theorem pin_all_held (ps : List pin.Pin) :
    (Calculus.Tri.all (ps.map pinHeldTri)).admits = true ↔ ∀ p ∈ ps, p = .Held := by
  rw [Calculus.Tri.admits_iff, Calculus.Tri.all_eq_yes]
  constructor
  · intro h p hp
    have := h _ (List.mem_map_of_mem hp)
    unfold pinHeldTri at this
    cases hb : Core.Pin.heldB p
    · simp [hb] at this
    · exact (Core.Pin.heldB_iff p).mp hb
  · intro h t ht
    obtain ⟨p, hp, rfl⟩ := List.mem_map.mp ht
    simp [pinHeldTri, (Core.Pin.heldB_iff p).mpr (h p hp)]

/-- Evidence holds exactly when every pin holds: `Tri.all` of the pins. -/
theorem pin_holds_implements (pins : Slice pin.Pin) :
    pin.holds pins ⦃ b => b = (Calculus.Tri.all (pins.val.map pinHeldTri)).admits ⦄ := by
  apply WP.spec_mono (Core.Pin.holds_spec pins)
  intro b hb
  rw [Bool.eq_iff_iff, hb, pin_all_held]

/-- Currency under pins: aboutness and every pin, as one `Tri.all`. -/
theorem pin_current_under_implements (about : Bool) (pins : Slice pin.Pin) :
    pin.current_under about pins ⦃ b => b =
      (Calculus.Tri.all ((if about then .yes else .no) :: pins.val.map pinHeldTri)).admits ⦄ := by
  apply WP.spec_mono (Core.Pin.current_under_spec about pins)
  intro b hb
  rw [Bool.eq_iff_iff, hb]
  show _ ↔ (Calculus.Tri.and (if about then .yes else .no)
    (Calculus.Tri.all (pins.val.map pinHeldTri))).admits = true
  rw [Calculus.Tri.admits_iff, Calculus.Tri.and_eq_yes, ← Calculus.Tri.admits_iff (Calculus.Tri.all _),
    pin_all_held]
  cases about <;> simp

/-- A look that reports no data. -/
def statusEmpty : observe.Status → Bool
  | .NoData => true
  | .NotFound => true
  | _ => false

/-- "Nothing found" is the core's absence, which needs the whole of what
was looked at: every pin held (law U). -/
theorem pin_nothing_found_implements (status : observe.Status) (pins : Slice pin.Pin) :
    pin.nothing_found status pins ⦃ b =>
      b = (Calculus.search (pins.val.all Core.Pin.heldB) (!statusEmpty status) == .no) ⦄ := by
  apply WP.spec_mono (Core.Pin.nothing_found_spec status pins)
  intro b hb
  rw [Bool.eq_iff_iff, hb]
  have he : (status = .NoData ∨ status = .NotFound) ↔ statusEmpty status = true := by
    cases status <;> simp [statusEmpty]
  have ha : (∀ p ∈ pins.val, p = .Held) ↔ pins.val.all Core.Pin.heldB = true := by
    simp [List.all_eq_true, Core.Pin.heldB_iff]
  rw [he, ha]
  cases statusEmpty status <;> cases pins.val.all Core.Pin.heldB <;> simp [Calculus.search]

/-! ## A plan's close: `complete` is the core's universal word -/

/-- A step's close as a member of the plan's universe: an open step is
left over; a closed or skipped one is judged, and holds only when passed
(skipped, attested and waived steps are exceptions). -/
def closeMember : policy.Close → Calculus.Member × Calculus.Tri
  | .Open => (.remainder, .unknown)
  | .Passed => (.judged, .yes)
  | _ => (.judged, .no)

theorem closes_universal (cs : List policy.Close) :
    Calculus.universal false (cs.map closeMember) = .yes ↔ cs ≠ [] ∧ ∀ c ∈ cs, c = .Passed := by
  rw [Calculus.universal_yes_iff, Calculus.whole_iff_no_remainder]
  have hj : ∀ t, t ∈ Calculus.judgedOf (cs.map closeMember) ↔
      ∃ c ∈ cs, c ≠ .Open ∧ (closeMember c).2 = t := by
    intro t
    simp only [Calculus.judgedOf, List.mem_filterMap, List.mem_map]
    constructor
    · rintro ⟨m, ⟨c, hc, rfl⟩, hm⟩
      split at hm
      · cases hm
        refine ⟨c, hc, ?_, rfl⟩
        intro ho; subst ho; simp [closeMember] at *
      · cases hm
    · rintro ⟨c, hc, ho, rfl⟩
      refine ⟨_, ⟨c, hc, rfl⟩, ?_⟩
      cases c <;> simp_all [closeMember]
  constructor
  · rintro ⟨hw, hall, hne⟩
    have hno : ∀ c ∈ cs, c ≠ .Open := by
      intro c hc ho; subst ho
      exact hw (List.mem_map.mpr ⟨_, List.mem_map.mpr ⟨_, hc, rfl⟩, rfl⟩)
    refine ⟨?_, fun c hc => ?_⟩
    · rcases hne with h | h
      · cases h
      · intro he; subst he; simp [Calculus.judgedOf] at h
    · have := hall _ ((hj _).mpr ⟨c, hc, hno c hc, rfl⟩)
      cases c <;> simp_all [closeMember]
  · rintro ⟨hne, hall⟩
    refine ⟨?_, ?_, Or.inr ?_⟩
    · intro hm
      obtain ⟨m, hm, he⟩ := List.mem_map.mp hm
      obtain ⟨c, hc, rfl⟩ := List.mem_map.mp hm
      rw [hall c hc] at he
      cases he
    · intro t ht
      obtain ⟨c, hc, _, rfl⟩ := (hj t).mp ht
      rw [hall c hc]; rfl
    · obtain ⟨c, hc⟩ := List.exists_mem_of_ne_nil cs hne
      intro he
      have := (hj _).mpr ⟨c, hc, by rw [hall c hc]; simp, rfl⟩
      rw [he] at this
      cases this

/-- Law U at a plan's close: `complete` is the core's universal word over
the steps (no step left open, every step passed, at least one step), and
the further facts a plan needs (its templates present, its policy unmoved,
no repin, nothing stale). -/
theorem policy_summary_implements (cs : Slice policy.Close) (f : policy.Facts) :
    policy.summary cs f ⦃ r => (r = .Complete ↔
      Calculus.universal false (cs.val.map closeMember) = .yes ∧ f.required_present = true ∧
        f.pin ≠ .Moved ∧ f.repinned = false ∧ f.stale = false) ⦄ := by
  apply WP.spec_mono (Core.Policy.summary_complete_iff cs f)
  intro r hr
  rw [hr, closes_universal]
  constructor
  · rintro ⟨h1, h2, h3, h4, h5, h6⟩; exact ⟨⟨h1, h2⟩, h3, h4, h5, h6⟩
  · rintro ⟨⟨h1, h2⟩, h3, h4, h5, h6⟩; exact ⟨h1, h2, h3, h4, h5, h6⟩

end Writfold.Core.Conform
