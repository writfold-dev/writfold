import Calculus.Word
/-!
# Evidence: classes, policy, one record decides (calculus.md 3.2 to 3.4, 3.13)
-/
namespace Calculus

/-- How a fact came to be recorded. `reported` is what a harness said
happened (law S); it is the weakest class and no policy takes it. -/
inductive Class where
  | checked | attested | waived | reported
  deriving DecidableEq, Repr

/-- What the deciding record of an obligation says now. -/
inductive Standing where
  | satisfied            -- a mechanical pass about the present
  | attested | waived | reported
  | unmet                -- open, stale or failed
  | unknown
  deriving DecidableEq, Repr

/-- The class a standing stands on, when it stands. -/
def Standing.cls : Standing → Option Class
  | .satisfied => some .checked
  | .attested => some .attested
  | .waived => some .waived
  | .reported => some .reported
  | _ => none

/-- 3.3 in one definition: the standing a record of class `c` can reach.
A record that is not about the present reaches none. -/
def stand (c : Class) (aboutPresent : Bool) (outcome : Tri) : Standing :=
  if !aboutPresent then .unmet
  else match c, outcome with
    | .checked, .yes => .satisfied
    | .checked, .no => .unmet
    | .checked, .unknown => .unknown
    | .attested, _ => .attested
    | .waived, _ => .waived
    | .reported, _ => .reported

/-- A class is never promoted. -/
theorem class_never_promoted (c : Class) (p : Bool) (o : Tri) (c' : Class)
    (h : (stand c p o).cls = some c') : c' = c := by
  revert h; cases c <;> cases p <;> cases o <;> simp [stand, Standing.cls] <;> exact fun h => h.symm

/-- Only a current mechanical pass is satisfied. -/
theorem satisfied_only_by_current_pass (c : Class) (p : Bool) (o : Tri)
    (h : stand c p o = .satisfied) : c = .checked ∧ p = true ∧ o = .yes := by
  revert h; cases c <;> cases p <;> cases o <;> simp [stand]

/-- A policy: the standings an obligation takes. There is no field for
`reported`, so no policy can take it. -/
structure Accept where
  satisfied : Bool
  attested : Bool
  waived : Bool
  deriving DecidableEq, Repr

def Accept.top : Accept := ⟨true, true, true⟩
def Accept.inf (a b : Accept) : Accept :=
  ⟨a.satisfied && b.satisfied, a.attested && b.attested, a.waived && b.waived⟩
def Accept.le (a b : Accept) : Prop :=
  (a.satisfied → b.satisfied) ∧ (a.attested → b.attested) ∧ (a.waived → b.waived)

/-- L2: does this standing meet the obligation? -/
def met (s : Standing) (a : Accept) : Bool :=
  match s with
  | .satisfied => a.satisfied
  | .attested => a.attested
  | .waived => a.waived
  | _ => false

/-- 3.4: narrowing is a meet, exactly. -/
theorem met_inf (s : Standing) (a b : Accept) : met s (a.inf b) = (met s a && met s b) := by
  cases s <;> simp [met, Accept.inf]

/-- A narrower policy meets no more than a wider one. -/
theorem met_narrows (s : Standing) {a b : Accept} (h : a.le b) : met s a = true → met s b = true := by
  cases s <;> simp [met] <;> first | exact h.1 | exact h.2.1 | exact h.2.2

/-- Law S: a reported fact meets no policy. -/
theorem reported_never_met (a : Accept) : met .reported a = false := rfl
/-- Unknown meets no policy. -/
theorem unknown_never_met (a : Accept) : met .unknown a = false := rfl

/-- Narrowing does not depend on order. -/
theorem inf_comm (a b : Accept) : a.inf b = b.inf a := by
  cases a; cases b; simp [Accept.inf, Bool.and_comm]
/-- Narrowing groups freely. -/
theorem inf_assoc (a b c : Accept) : (a.inf b).inf c = a.inf (b.inf c) := by
  cases a; cases b; cases c; simp [Accept.inf, Bool.and_assoc]
/-- Narrowing by the same policy changes nothing. -/
theorem inf_idem (a : Accept) : a.inf a = a := by cases a; simp [Accept.inf]
/-- Narrowing by the widest policy changes nothing. -/
theorem inf_top (a : Accept) : a.inf .top = a := by cases a; simp [Accept.inf, Accept.top]

/-- 3.2: of the records that count (bound, intact, of this subject), the
one with the greatest `seq` decides; a later one wins a tie. -/
def deciding {α : Type} (seq : α → Nat) (counts : α → Bool) (rs : List α) : Option α :=
  (rs.filter counts).foldl
    (fun best r => match best with
      | none => some r
      | some b => if seq b ≤ seq r then some r else some b) none

/-- No record decides exactly when none counts. -/
theorem deciding_none_iff {α : Type} (seq : α → Nat) (counts : α → Bool) (rs : List α) :
    deciding seq counts rs = none ↔ ∀ r ∈ rs, counts r = false := by
  unfold deciding
  have key : ∀ (l : List α) (init : Option α),
      l.foldl (fun best r => match best with
        | none => some r
        | some b => if seq b ≤ seq r then some r else some b) init = none ↔ init = none ∧ l = [] := by
    intro l
    induction l with
    | nil => intro init; simp
    | cons x xs ih =>
      intro init
      rw [List.foldl_cons, ih]
      cases init <;> simp <;> split <;> simp
  rw [key]
  simp [List.filter_eq_nil_iff]

/-- A record that does not count never changes which record decides. -/
theorem uncounted_never_decides {α : Type} (seq : α → Nat) (counts : α → Bool)
    (rs : List α) (r : α) (h : counts r = false) :
    deciding seq counts (r :: rs) = deciding seq counts rs := by
  simp [deciding, h]

/-- The deciding record is one of the records that count. -/
theorem deciding_mem {α : Type} (seq : α → Nat) (counts : α → Bool) (rs : List α) (r : α)
    (h : deciding seq counts rs = some r) : r ∈ rs ∧ counts r = true := by
  unfold deciding at h
  have key : ∀ (l : List α) (init : Option α) (x : α),
      l.foldl (fun best r => match best with
        | none => some r
        | some b => if seq b ≤ seq r then some r else some b) init = some x →
      x ∈ l ∨ init = some x := by
    intro l
    induction l with
    | nil => intro init x h; exact Or.inr h
    | cons y ys ih =>
      intro init x h
      rw [List.foldl_cons] at h
      rcases ih _ x h with hx | hx
      · exact Or.inl (List.mem_cons_of_mem _ hx)
      · cases init with
        | none => simp at hx; exact Or.inl (by simp [hx])
        | some b =>
          simp only at hx
          split at hx
          · simp at hx; exact Or.inl (by simp [hx])
          · exact Or.inr hx
  rcases key _ none r h with hm | hm
  · exact ⟨(List.mem_filter.mp hm).1, by simpa using (List.mem_filter.mp hm).2⟩
  · cases hm

/-- 3.2: the deciding record is the newest of those that count. -/
theorem deciding_is_newest {α : Type} (seq : α → Nat) (counts : α → Bool) (rs : List α) (r : α)
    (h : deciding seq counts rs = some r) : ∀ r' ∈ rs, counts r' = true → seq r' ≤ seq r := by
  unfold deciding at h
  have key : ∀ (l : List α) (init : Option α) (x : α),
      l.foldl (fun best r => match best with
        | none => some r
        | some b => if seq b ≤ seq r then some r else some b) init = some x →
      (∀ y ∈ l, seq y ≤ seq x) ∧ (∀ b, init = some b → seq b ≤ seq x) := by
    intro l
    induction l with
    | nil => intro init x h; simp at h; subst h; exact ⟨by simp, fun b hb => by simp_all⟩
    | cons y ys ih =>
      intro init x h
      rw [List.foldl_cons] at h
      obtain ⟨hl, hi⟩ := ih _ x h
      cases init with
      | none =>
        simp only at hi
        refine ⟨?_, by simp⟩
        intro z hz
        rcases List.mem_cons.mp hz with rfl | hz
        · exact hi z rfl
        · exact hl z hz
      | some b =>
        simp only at hi
        split at hi
        · rename_i hle
          refine ⟨?_, fun b' hb' => ?_⟩
          · intro z hz
            rcases List.mem_cons.mp hz with rfl | hz
            · exact hi z rfl
            · exact hl z hz
          · simp at hb'; subst hb'; exact Nat.le_trans hle (hi _ rfl)
        · rename_i hle
          refine ⟨?_, fun b' hb' => ?_⟩
          · intro z hz
            rcases List.mem_cons.mp hz with rfl | hz
            · have := hi b rfl; omega
            · exact hl z hz
          · simp at hb'; subst hb'; exact hi _ rfl
  intro r' hr' hc
  exact (key _ none r h).1 r' (List.mem_filter.mpr ⟨hr', by simpa using hc⟩)

/-- 3.2: a record that counts never loses to an older one. -/
theorem older_never_outvotes {α : Type} (seq : α → Nat) (counts : α → Bool) (rs : List α)
    (newer older : α) (hn : newer ∈ rs) (hc : counts newer = true) (hlt : seq older < seq newer) :
    deciding seq counts rs ≠ some older := by
  intro h
  have := deciding_is_newest seq counts rs older h newer hn hc
  omega

end Calculus
