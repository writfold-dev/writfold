import Writfold.Bytes

/-!
# Grant authority (E12)

Theorems over the Aeneas translation of `writfold_core::authority`, stated
against spec predicates mirroring the calculus's `installGrant` guards,
`revokeGrant`, `advanceTime`, `currentGrant` and the grant conjunct of
`intentReady` (`formal/Kctx/Engineering/Environment.lean`, `Control.lean`,
`Core.lean`). Grants are stored oldest first; "newest" is the highest index.
-/

open Aeneas Aeneas.Std Result
open writfold_core

namespace Writfold.Core

/-- Some installed grant, revoked or not, has the id. -/
def HasId (grants : Slice authority.Installed) (id : Slice U8) : Prop :=
  ∃ g ∈ grants.val, g.grant.id.val = id.val

theorem has_id_loop_spec (grants : Slice authority.Installed) (id : Slice U8) (i : Usize)
    (hi : i.val ≤ grants.val.length)
    (hmiss : ∀ k, k < i.val → ∀ g, grants.val[k]? = some g → g.grant.id.val ≠ id.val) :
    authority.has_id_loop grants id i ⦃ r => r ↔ HasId grants id ⦄ := by
  unfold authority.has_id_loop
  apply loop.spec_decr_nat (fun i : Usize => grants.val.length - i.val)
    (fun i : Usize => i.val ≤ grants.val.length ∧
      ∀ k, k < i.val → ∀ g, grants.val[k]? = some g → g.grant.id.val ≠ id.val)
  · intro i ⟨hi, hmiss⟩
    unfold authority.has_id_loop.body
    dsimp only
    split
    · step as ⟨ g, hg ⟩
      step with same_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, true_iff]
        refine ⟨g, ?_, by simpa [alloc.vec.Vec.deref, Slice.from_val] using hb.mp hbt⟩
        rw [hg]; exact List.getElem_mem _
      · rename_i hbf
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        and_intros
        all_goals try scalar_tac
        intro k hk g' hg'
        by_cases hki : k < i.val
        · exact hmiss k hki g' hg'
        · have : k = i.val := by scalar_tac
          subst this
          rw [List.getElem?_eq_getElem (by scalar_tac)] at hg'
          simp only [Option.some.injEq] at hg'
          rw [← hg', ← hg]
          intro heq
          exact hbf (hb.mpr (by simpa [alloc.vec.Vec.deref, Slice.from_val] using heq))
    · simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      rintro ⟨g, hmem, hid⟩
      obtain ⟨k, hk, hkg⟩ := List.getElem_of_mem hmem
      exact hmiss k (by scalar_tac) g (by rw [List.getElem?_eq_getElem hk, hkg]) hid
  · exact ⟨hi, hmiss⟩

theorem has_id_spec (grants : Slice authority.Installed) (id : Slice U8) :
    authority.has_id grants id ⦃ r => r ↔ HasId grants id ⦄ := by
  unfold authority.has_id
  apply has_id_loop_spec
  all_goals simp

open Classical in
/-- Lean `installGrant`'s guards, in order: the first failing guard names the
refusal. A revoked id is still installed, so it can never be reinstalled. -/
noncomputable def installOutcome (realm policy : Slice U8) (now : U64)
    (grants : Slice authority.Installed) (a : authority.Authorization)
    (retained : Slice Digest) : core.result.Result Unit authority.Refusal :=
  if a.realm.val ≠ realm.val then .Err .ForeignRealm
  else if a.grant.scope_realm.val ≠ realm.val then .Err .ForeignBindingRealm
  else if a.policy.val ≠ policy.val then .Err .StalePolicy
  else if a.envelope ∉ retained.val then .Err .UnretainedEnvelope
  else if HasId grants (alloc.vec.Vec.deref a.grant.id) then .Err .DuplicateGrant
  else if now.val < a.grant.valid_from.val then .Err .NotYetValid
  else if a.grant.expires.val ≤ now.val then .Err .Expired
  else .Ok ()

theorem install_check_spec (realm policy : Slice U8) (now : U64)
    (grants : Slice authority.Installed) (a : authority.Authorization) (retained : Slice Digest) :
    authority.install_check realm policy now grants a retained ⦃ r =>
      r = installOutcome realm policy now grants a retained ⦄ := by
  unfold authority.install_check
  have deref : ∀ v : alloc.vec.Vec U8, (alloc.vec.Vec.deref v).val = v.val := by
    intro v; simp [alloc.vec.Vec.deref, Slice.from_val]
  step with same_spec as ⟨ b, hb ⟩
  rw [deref] at hb
  split
  · rename_i h1
    step with same_spec as ⟨ b1, hb1 ⟩
    rw [deref] at hb1
    split
    · rename_i h2
      step with same_spec as ⟨ b2, hb2 ⟩
      rw [deref] at hb2
      split
      · rename_i h3
        step with contains_digest_spec as ⟨ b3, hb3 ⟩
        split
        · rename_i h4
          step with has_id_spec as ⟨ b4, hb4 ⟩
          split
          · rename_i h5
            simp only [WP.spec_ok, installOutcome]
            simp [hb.mp h1, hb1.mp h2, hb2.mp h3, hb3.mp h4, hb4.mp h5]
          · rename_i h5
            have h5' : ¬ HasId grants (alloc.vec.Vec.deref a.grant.id) := fun h => h5 (hb4.mpr h)
            split
            · rename_i h6
              simp only [WP.spec_ok, installOutcome]
              have : now.val < a.grant.valid_from.val := by scalar_tac
              simp [hb.mp h1, hb1.mp h2, hb2.mp h3, hb3.mp h4, h5', this]
            · rename_i h6
              have h6' : ¬ now.val < a.grant.valid_from.val := by scalar_tac
              split
              · rename_i h7
                have : a.grant.expires.val ≤ now.val := by scalar_tac
                simp only [WP.spec_ok, installOutcome]
                simp [hb.mp h1, hb1.mp h2, hb2.mp h3, hb3.mp h4, h5', h6', this]
              · rename_i h7
                have : ¬ a.grant.expires.val ≤ now.val := by scalar_tac
                simp only [WP.spec_ok, installOutcome]
                simp [hb.mp h1, hb1.mp h2, hb2.mp h3, hb3.mp h4, h5', h6', this]
        · rename_i h4
          have h4' : a.envelope ∉ retained.val := fun h => h4 (hb3.mpr h)
          simp only [WP.spec_ok, installOutcome]
          simp [hb.mp h1, hb1.mp h2, hb2.mp h3, h4']
      · rename_i h3
        have h3' : a.policy.val ≠ policy.val := fun h => h3 (hb2.mpr h)
        simp only [WP.spec_ok, installOutcome]
        simp [hb.mp h1, hb1.mp h2, h3']
    · rename_i h2
      have h2' : a.grant.scope_realm.val ≠ realm.val := fun h => h2 (hb1.mpr h)
      simp only [WP.spec_ok, installOutcome]
      simp [hb.mp h1, h2']
  · rename_i h1
    have h1' : a.realm.val ≠ realm.val := fun h => h1 (hb.mpr h)
    simp only [WP.spec_ok, installOutcome]
    simp [h1']

/-- Lean `currentGrant`'s filter: same scope, not revoked, inside its window. -/
def Live (g : authority.Installed) (scope : Slice U8) (now : U64) : Prop :=
  g.grant.scope.val = scope.val ∧ g.revoked = false ∧
    g.grant.valid_from.val ≤ now.val ∧ now.val < g.grant.expires.val

theorem live_spec (g : authority.Installed) (scope : Slice U8) (now : U64) :
    authority.live g scope now ⦃ r => r ↔ Live g scope now ⦄ := by
  unfold authority.live
  step with same_spec as ⟨ b, hb ⟩
  simp only [alloc.vec.Vec.deref, Slice.from_val] at hb
  split
  · rename_i hs
    split
    · rename_i hr
      simp [Live, hr]
    · rename_i hr
      split
      · rename_i hv
        simp only [WP.spec_ok, decide_eq_true_eq, Live]
        constructor
        · intro h; exact ⟨hb.mp hs, by simpa using hr, by scalar_tac, by scalar_tac⟩
        · rintro ⟨_, _, _, h⟩; scalar_tac
      · rename_i hv
        simp only [WP.spec_ok, Bool.false_eq_true, false_iff, Live]
        rintro ⟨_, _, h, _⟩; scalar_tac
  · rename_i hs
    simp only [WP.spec_ok, Bool.false_eq_true, false_iff, Live]
    rintro ⟨h, _⟩; exact hs (hb.mpr h)

/-- What `current` returns: the newest live grant for the scope, or none. -/
def CurrentOk (grants : Slice authority.Installed) (scope : Slice U8) (now : U64) :
    Option Usize → Prop
  | none => ∀ g ∈ grants.val, ¬ Live g scope now
  | some k => ∃ h : k.val < grants.val.length, Live (grants.val[k.val]'h) scope now ∧
      ∀ j (hj : j < grants.val.length), k.val < j → ¬ Live (grants.val[j]'hj) scope now

theorem current_loop_spec (grants : Slice authority.Installed) (scope : Slice U8) (now : U64)
    (i : Usize) (hi : i.val ≤ grants.val.length)
    (hnewer : ∀ j (hj : j < grants.val.length), i.val ≤ j → ¬ Live (grants.val[j]'hj) scope now) :
    authority.current_loop grants scope now i ⦃ o => CurrentOk grants scope now o ⦄ := by
  unfold authority.current_loop
  apply loop.spec_decr_nat (fun i : Usize => i.val)
    (fun i : Usize => i.val ≤ grants.val.length ∧
      ∀ j (hj : j < grants.val.length), i.val ≤ j → ¬ Live (grants.val[j]'hj) scope now)
  · intro i ⟨hi, hnewer⟩
    unfold authority.current_loop.body
    split
    · rename_i hpos
      step as ⟨ k, hk ⟩
      step as ⟨ g, hg ⟩
      step with live_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, CurrentOk]
        refine ⟨by scalar_tac, ?_, ?_⟩
        · rw [← hg]; exact hb.mp hbt
        · intro j hj hkj; exact hnewer j hj (by scalar_tac)
      · rename_i hbf
        simp only [WP.spec_ok]
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj hkj
        by_cases hji : i.val ≤ j
        · exact hnewer j hj hji
        · have : j = k.val := by scalar_tac
          subst this
          intro hl; apply hbf; apply hb.mpr
          rw [hg]; exact hl
    · rename_i hz
      simp only [WP.spec_ok, CurrentOk]
      intro g hmem
      obtain ⟨j, hj, hjg⟩ := List.getElem_of_mem hmem
      rw [← hjg]
      exact hnewer j hj (by scalar_tac)
  · exact ⟨hi, hnewer⟩

/-- `current` is Lean `currentGrant`: the newest live grant for exactly this
scope; a newer grant shadows older ones even when it covers less. -/
theorem current_spec (grants : Slice authority.Installed) (scope : Slice U8) (now : U64) :
    authority.current grants scope now ⦃ o => CurrentOk grants scope now o ⦄ := by
  unfold authority.current
  apply current_loop_spec
  · simp [Slice.len]
  · intro j hj hle; simp [Slice.len] at hle; omega

/-- Lean `revokeGrant` and `advanceTime` refuse or change only what they name. -/
theorem Authority.advance_spec (self : authority.Authority) (time : U64) :
    authority.Authority.advance self time ⦃ r =>
      (time.val < self.now.val → r = (.Err .ClockRewind, self)) ∧
      (self.now.val ≤ time.val → r = (.Ok (), { self with now := time })) ⦄ := by
  unfold authority.Authority.advance
  split <;> simp only [WP.spec_ok] <;>
    refine ⟨fun _ => ?_, fun _ => ?_⟩ <;> first | trivial | rfl | (exfalso; scalar_tac)

/-- Installation refused by a guard changes nothing. -/
theorem Authority.install_refused (self : authority.Authority) (a : authority.Authorization)
    (retained : Slice Digest) (r : authority.Refusal)
    (h : authority.install_check (alloc.vec.Vec.deref self.realm) (alloc.vec.Vec.deref self.policy)
      self.now (alloc.vec.Vec.deref self.grants) a retained = ok (.Err r)) :
    authority.Authority.install self a retained = ok (.Err r, self) := by
  unfold authority.Authority.install
  dsimp only
  rw [h]
  simp [core.result.Result.Insts.CoreOpsTry.branch,
    core.result.Result.Insts.CoreOpsTry_traitFromResidualResult.from_residual]

theorem any_same_loop_spec (items : Slice (alloc.vec.Vec U8)) (item : Slice U8) (j : Usize)
    (hj : j.val ≤ items.val.length)
    (hmiss : ∀ k, k < j.val → ∀ x, items.val[k]? = some x → x.val ≠ item.val) :
    authority.any_same_loop items item j ⦃ r => r ↔ ∃ x ∈ items.val, x.val = item.val ⦄ := by
  unfold authority.any_same_loop
  apply loop.spec_decr_nat (fun j : Usize => items.val.length - j.val)
    (fun j : Usize => j.val ≤ items.val.length ∧
      ∀ k, k < j.val → ∀ x, items.val[k]? = some x → x.val ≠ item.val)
  · intro j ⟨hj, hmiss⟩
    unfold authority.any_same_loop.body
    dsimp only
    split
    · step as ⟨ x, hx ⟩
      step with same_spec as ⟨ b, hb ⟩
      split
      · rename_i hbt
        simp only [WP.spec_ok, true_iff]
        refine ⟨x, ?_, by simpa [alloc.vec.Vec.deref, Slice.from_val] using hb.mp hbt⟩
        rw [hx]; exact List.getElem_mem _
      · rename_i hbf
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro j' hj'
        simp only [WP.spec_ok]
        and_intros
        all_goals try scalar_tac
        intro k hk x' hx'
        by_cases hkj : k < j.val
        · exact hmiss k hkj x' hx'
        · have : k = j.val := by scalar_tac
          subst this
          rw [List.getElem?_eq_getElem (by scalar_tac)] at hx'
          simp only [Option.some.injEq] at hx'
          rw [← hx', ← hx]
          intro heq
          exact hbf (hb.mpr (by simpa [alloc.vec.Vec.deref, Slice.from_val] using heq))
    · simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      rintro ⟨x, hmem, hxv⟩
      obtain ⟨k, hk, hkx⟩ := List.getElem_of_mem hmem
      exact hmiss k (by scalar_tac) x (by rw [List.getElem?_eq_getElem hk, hkx]) hxv
  · exact ⟨hj, hmiss⟩

theorem any_same_spec (items : Slice (alloc.vec.Vec U8)) (item : Slice U8) :
    authority.any_same items item ⦃ r => r ↔ ∃ x ∈ items.val, x.val = item.val ⦄ := by
  unfold authority.any_same
  apply any_same_loop_spec
  all_goals simp

theorem has_capability_loop_spec (caps : Slice U8) (c : U8) (i : Usize)
    (hi : i.val ≤ caps.val.length) (hmiss : ∀ k, k < i.val → caps.val[k]? ≠ some c) :
    authority.has_capability_loop caps c i ⦃ r => r ↔ c ∈ caps.val ⦄ := by
  unfold authority.has_capability_loop
  apply loop.spec_decr_nat (fun i : Usize => caps.val.length - i.val)
    (fun i : Usize => i.val ≤ caps.val.length ∧ ∀ k, k < i.val → caps.val[k]? ≠ some c)
  · intro i ⟨hi, hmiss⟩
    unfold authority.has_capability_loop.body
    dsimp only
    split
    · step as ⟨ x, hx ⟩
      split
      · rename_i heq
        simp only [WP.spec_ok, true_iff]
        rw [← heq, hx]; exact List.getElem_mem _
      · rename_i hne
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        and_intros
        all_goals try scalar_tac
        intro k hk
        by_cases hki : k < i.val
        · exact hmiss k hki
        · have : k = i.val := by scalar_tac
          subst this
          rw [List.getElem?_eq_getElem (by scalar_tac)]
          intro h; simp only [Option.some.injEq] at h
          exact hne (by rw [hx, h])
    · simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      intro hmem
      obtain ⟨k, hk, hkc⟩ := List.getElem_of_mem hmem
      exact hmiss k (by scalar_tac) (by rw [List.getElem?_eq_getElem hk, hkc])
  · exact ⟨hi, hmiss⟩

theorem has_capability_spec (caps : Slice U8) (c : U8) :
    authority.has_capability caps c ⦃ r => r ↔ c ∈ caps.val ⦄ := by
  unfold authority.has_capability
  apply has_capability_loop_spec
  all_goals simp

/-- Every requested resource is among the granted ones. -/
def Within (granted resources : Slice (alloc.vec.Vec U8)) : Prop :=
  ∀ r ∈ resources.val, ∃ x ∈ granted.val, x.val = r.val

theorem resources_within_loop_spec (granted resources : Slice (alloc.vec.Vec U8)) (i : Usize)
    (hi : i.val ≤ resources.val.length)
    (hdone : ∀ k, k < i.val → ∀ r, resources.val[k]? = some r → ∃ x ∈ granted.val, x.val = r.val) :
    authority.resources_within_loop granted resources i ⦃ b => b ↔ Within granted resources ⦄ := by
  unfold authority.resources_within_loop
  apply loop.spec_decr_nat (fun i : Usize => resources.val.length - i.val)
    (fun i : Usize => i.val ≤ resources.val.length ∧
      ∀ k, k < i.val → ∀ r, resources.val[k]? = some r → ∃ x ∈ granted.val, x.val = r.val)
  · intro i ⟨hi, hdone⟩
    unfold authority.resources_within_loop.body
    dsimp only
    split
    · step as ⟨ r, hr ⟩
      step with any_same_spec as ⟨ b, hb ⟩
      have hbv : b = true ↔ ∃ x ∈ granted.val, x.val = r.val := by
        simpa [alloc.vec.Vec.deref, Slice.from_val] using hb
      split
      · rename_i hbt
        apply WP.spec_bind (Usize.add_spec (by scalar_tac))
        intro i' hi'
        simp only [WP.spec_ok]
        and_intros
        all_goals try scalar_tac
        intro k hk r' hr'
        by_cases hki : k < i.val
        · exact hdone k hki r' hr'
        · have : k = i.val := by scalar_tac
          subst this
          rw [List.getElem?_eq_getElem (by scalar_tac)] at hr'
          simp only [Option.some.injEq] at hr'
          rw [← hr', ← hr]; exact hbv.mp hbt
      · rename_i hbf
        simp only [WP.spec_ok, Bool.false_eq_true, false_iff, Within]
        intro hall
        exact hbf (hbv.mpr (hall r (by rw [hr]; exact List.getElem_mem _)))
    · simp only [WP.spec_ok, true_iff, Within]
      intro r hmem
      obtain ⟨k, hk, hkr⟩ := List.getElem_of_mem hmem
      exact hdone k (by scalar_tac) r (by rw [List.getElem?_eq_getElem hk, hkr])
  · exact ⟨hi, hdone⟩

theorem resources_within_spec (granted resources : Slice (alloc.vec.Vec U8)) :
    authority.resources_within granted resources ⦃ b => b ↔ Within granted resources ⦄ := by
  unfold authority.resources_within
  apply resources_within_loop_spec
  all_goals simp

/-- `covers` is the grant conjunct of Lean `intentReady`: the current grant
for the scope carries the capability and every requested resource. -/
theorem covers_spec (grants : Slice authority.Installed) (scope : Slice U8) (now : U64)
    (c : U8) (resources : Slice (alloc.vec.Vec U8)) :
    authority.covers grants scope now c resources ⦃ b =>
      b ↔ ∃ k, CurrentOk grants scope now (some k) ∧
        ∃ g, grants.val[k.val]? = some g ∧ c ∈ g.grant.capabilities.val ∧
          Within (alloc.vec.Vec.deref g.grant.resources) resources ⦄ := by
  unfold authority.covers
  step with current_spec as ⟨ o, ho ⟩
  cases o with
  | none =>
    simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_exists, not_and]
    intro k hk
    obtain ⟨hlt, hlive, _⟩ := hk
    exact absurd hlive (ho _ (List.getElem_mem hlt))
  | some k =>
    have hk := ho
    obtain ⟨hlt, _, _⟩ := ho
    step as ⟨ g, hg ⟩
    have hget : grants.val[k.val]? = some g := by rw [List.getElem?_eq_getElem hlt, hg]
    step with has_capability_spec as ⟨ b, hb ⟩
    have hbv : b = true ↔ c ∈ g.grant.capabilities.val := by
      simpa [alloc.vec.Vec.deref, Slice.from_val] using hb
    split
    · rename_i hbt
      step with resources_within_spec as ⟨ r, hr ⟩
      constructor
      · intro hrt; exact ⟨k, hk, g, hget, hbv.mp hbt, hr.mp hrt⟩
      · rintro ⟨k', hk', g', hg', hc, hw⟩
        have : k' = k := by
          obtain ⟨h1, hl1, hn1⟩ := hk'
          obtain ⟨h2, hl2, hn2⟩ := hk
          apply UScalar.eq_of_val_eq
          by_contra hne
          rcases Nat.lt_or_gt_of_ne hne with hlt' | hgt
          · exact hn1 k.val h2 hlt' hl2
          · exact hn2 k'.val h1 hgt hl1
        subst this
        rw [hget] at hg'; simp only [Option.some.injEq] at hg'; subst hg'
        exact hr.mpr hw
    · rename_i hbf
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_exists, not_and]
      intro k' hk' g' hg' hc
      have : k' = k := by
        obtain ⟨h1, hl1, hn1⟩ := hk'
        obtain ⟨h2, hl2, hn2⟩ := hk
        apply UScalar.eq_of_val_eq
        by_contra hne
        rcases Nat.lt_or_gt_of_ne hne with hlt' | hgt
        · exact hn1 k.val h2 hlt' hl2
        · exact hn2 k'.val h1 hgt hl1
      subst this
      rw [hget] at hg'; simp only [Option.some.injEq] at hg'; subst hg'
      exact absurd (hbv.mpr hc) hbf

end Writfold.Core
