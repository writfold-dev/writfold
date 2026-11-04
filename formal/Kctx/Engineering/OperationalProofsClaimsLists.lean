import Kctx.Engineering.OperationalProofsRecordsCatalog

set_option autoImplicit false
set_option maxRecDepth 8192
set_option maxHeartbeats 3200000
namespace Kctx.Engineering

/-- Authoritative retained stamps, including stamps superseded by a retry. -/
structure OperationalClaims (s : OperationalState) : Prop where
  claims_unique : (s.claims.map Dispatch.id).Nodup
  claim_effects : ∀ claim ∈ s.claims, ∃ effect ∈ s.effects,
    effect.intent = claim.intent ∧ effect.generation = claim.generation
  dispatch_inventory : ∀ effect ∈ s.effects, ∀ claim,
    effect.dispatch = some claim → claim ∈ s.claims

theorem OperationalWellFormed.claimFields {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) : OperationalClaims s :=
  ⟨wf.claims_unique, wf.claim_effects, wf.dispatch_inventory⟩

/-- The list formulation makes the frame's unrelated fields immaterial. -/
def ClaimInventory (effects : List Effect) (claims : List Dispatch) : Prop :=
  (claims.map Dispatch.id).Nodup ∧
  (∀ claim ∈ claims, ∃ effect ∈ effects,
    effect.intent = claim.intent ∧ effect.generation = claim.generation) ∧
  (∀ effect ∈ effects, ∀ claim, effect.dispatch = some claim → claim ∈ claims)

theorem OperationalClaims.inventory {s : OperationalState} (h : OperationalClaims s) :
    ClaimInventory s.effects s.claims :=
  ⟨h.claims_unique, h.claim_effects, h.dispatch_inventory⟩

theorem OperationalClaims.of_inventory {s : OperationalState}
    (h : ClaimInventory s.effects s.claims) : OperationalClaims s :=
  ⟨h.1, h.2.1, h.2.2⟩

theorem claims_inventory_prepare {effects : List Effect} {claims : List Dispatch}
    (before : ClaimInventory effects claims) (intent : Intent) (generation : Nat) :
    ClaimInventory (⟨intent, generation, .prepared, none⟩ :: effects) claims := by
  refine ⟨before.1, ?_, ?_⟩
  · intro claim member
    obtain ⟨effect, present, anchors⟩ := before.2.1 claim member
    exact ⟨effect, List.mem_cons_of_mem _ present, anchors⟩
  · intro effect member claim selected
    rcases List.mem_cons.mp member with rfl | member
    · cases selected
    · exact before.2.2 effect member claim selected

/-- Map transport retains every old intent/generation witness and transports the
exact dispatch value, not merely its identifier. -/
theorem claims_inventory_map {effects : List Effect} {claims : List Dispatch}
    (before : ClaimInventory effects claims) (f : Effect → Effect) (g : Dispatch → Dispatch)
    (effectAnchors : ∀ effect ∈ effects,
      (f effect).intent = effect.intent ∧ (f effect).generation = effect.generation)
    (claimAnchors : ∀ claim ∈ claims,
      (g claim).intent = claim.intent ∧ (g claim).generation = claim.generation)
    (ids : ∀ claim ∈ claims, (g claim).id = claim.id)
    (dispatches : ∀ effect ∈ effects, (f effect).dispatch = effect.dispatch.map g) :
    ClaimInventory (effects.map f) (claims.map g) := by
  refine ⟨?_, ?_, ?_⟩
  · have keys : (claims.map g).map Dispatch.id = claims.map Dispatch.id := by
      simp only [List.map_map]
      apply List.map_congr_left
      intro claim member
      exact ids claim member
    rw [keys]
    exact before.1
  · intro claim member
    obtain ⟨old, present, rfl⟩ := List.mem_map.mp member
    obtain ⟨effect, effectMember, sameIntent, sameGeneration⟩ := before.2.1 old present
    exact ⟨f effect, List.mem_map.mpr ⟨effect, effectMember, rfl⟩,
      (effectAnchors effect effectMember).1.trans
        (sameIntent.trans (claimAnchors old present).1.symm),
      (effectAnchors effect effectMember).2.trans
        (sameGeneration.trans (claimAnchors old present).2.symm)⟩
  · intro effect member claim selected
    obtain ⟨old, present, rfl⟩ := List.mem_map.mp member
    rw [dispatches old present] at selected
    cases found : old.dispatch with
    | none => simp [found] at selected
    | some stamp =>
      have same : g stamp = claim := by simpa [found] using selected
      exact List.mem_map.mpr ⟨stamp, before.2.2 old present stamp found, same⟩

/-- Replacing an effect cannot remove historical stamp witnesses: unique effect
identities make every replaced entry the selected effect itself. -/
theorem claims_inventory_put {effects : List Effect} {claims : List Dispatch}
    (before : ClaimInventory effects claims)
    (unique : (effects.map (fun effect => effect.intent.effect)).Nodup)
    {old next : Effect} (member : old ∈ effects)
    (intent : next.intent = old.intent) (generation : next.generation = old.generation)
    (dispatch : ∀ stamp, next.dispatch = some stamp → stamp ∈ claims) :
    ClaimInventory
      (effects.map (fun effect => if effect.intent.effect == next.intent.effect then next else effect))
      claims := by
  have anchors : ∀ effect ∈ effects,
      (if effect.intent.effect == next.intent.effect then next else effect).intent = effect.intent ∧
      (if effect.intent.effect == next.intent.effect then next else effect).generation = effect.generation := by
    intro effect present
    split
    · next same =>
        have eq : effect = old := member_eq_of_unique_key unique present member
          (by simpa [intent] using same)
        subst effect
        exact ⟨intent, generation⟩
    · exact ⟨rfl, rfl⟩
  refine ⟨before.1, ?_, ?_⟩
  · intro claim present
    obtain ⟨effect, effectMember, sameIntent, sameGeneration⟩ := before.2.1 claim present
    refine ⟨_, List.mem_map.mpr ⟨effect, effectMember, rfl⟩, ?_, ?_⟩
    · exact (anchors effect effectMember).1.trans sameIntent
    · exact (anchors effect effectMember).2.trans sameGeneration
  · intro effect present claim selected
    obtain ⟨original, originalMember, rfl⟩ := List.mem_map.mp present
    split at selected
    · exact dispatch claim selected
    · exact before.2.2 original originalMember claim selected

/-- Both initial claims and retries prepend a genuinely fresh stamp. Old stamps
retain the same effect witness even when the current dispatch is overwritten. -/
theorem claims_inventory_prepend {effects : List Effect} {claims : List Dispatch}
    (before : ClaimInventory effects claims)
    (unique : (effects.map (fun effect => effect.intent.effect)).Nodup)
    {old : Effect} (member : old ∈ effects) (stamp : Dispatch)
    (intent : stamp.intent = old.intent) (generation : stamp.generation = old.generation)
    (fresh : ∀ previous ∈ claims, previous.id ≠ stamp.id) :
    ClaimInventory
      (effects.map (fun effect => if effect.intent.effect == old.intent.effect then
        { old with outcome := .claimed, dispatch := some stamp } else effect))
      (stamp :: claims) := by
  have extended : ClaimInventory effects (stamp :: claims) := by
    refine ⟨?_, ?_, ?_⟩
    · simp only [List.map_cons, List.nodup_cons]
      refine ⟨?_, before.1⟩
      intro present
      obtain ⟨previous, member, same⟩ := List.mem_map.mp present
      exact fresh previous member same
    · intro claim present
      rcases List.mem_cons.mp present with rfl | present
      · exact ⟨old, member, intent.symm, generation.symm⟩
      · exact before.2.1 claim present
    · intro effect present claim selected
      exact List.mem_cons_of_mem _ (before.2.2 effect present claim selected)
  apply claims_inventory_put (next := { old with outcome := .claimed, dispatch := some stamp })
    extended unique member rfl rfl
  intro claim selected
  have same : stamp = claim := Option.some.inj selected
  subst claim
  exact List.mem_cons.mpr (Or.inl rfl)

/-- Sending mutates the selected effect and the matching retained stamp together.
Uniqueness excludes changing a stamp belonging to any different current effect. -/
theorem claims_inventory_issue {contracts : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed contracts s) {old : Effect} {stamp : Dispatch}
    (member : old ∈ s.effects) (selected : old.dispatch = some stamp) :
    ClaimInventory
      (s.effects.map (fun effect => if effect.intent.effect == old.intent.effect then
        { old with dispatch := some { stamp with issued := true } } else effect))
      (s.claims.map (fun previous => if previous.id == stamp.id then
        { previous with issued := true } else previous)) := by
  let f := fun effect : Effect => if effect.intent.effect == old.intent.effect then
    { old with dispatch := some { stamp with issued := true } } else effect
  let g := fun previous : Dispatch => if previous.id == stamp.id then
    { previous with issued := true } else previous
  apply claims_inventory_map wf.claimFields.inventory f g
  · intro effect present
    dsimp [f]
    split
    · next same =>
        have equal : effect = old := member_eq_of_unique_key wf.effects_unique present member
          (by simpa using same)
        subst effect
        exact ⟨rfl, rfl⟩
    · exact ⟨rfl, rfl⟩
  · intro claim _
    dsimp [g]
    split <;> exact ⟨rfl, rfl⟩
  · intro claim _
    dsimp [g]
    split <;> rfl
  · intro effect present
    dsimp [f]
    split
    · next same =>
        have equal : effect = old := member_eq_of_unique_key wf.effects_unique present member
          (by simpa using same)
        subst effect
        simp [selected, g]
    · next different =>
        cases found : effect.dispatch with
        | none => rfl
        | some other =>
          have otherMember := wf.dispatch_inventory effect present other found
          have stampMember := wf.dispatch_inventory old member stamp selected
          have unequal : other.id ≠ stamp.id := by
            intro same
            have equal : other = stamp := member_eq_of_unique_key wf.claims_unique
              otherMember stampMember same
            have otherIntent := (wf.effects effect present).dispatch other found
            have stampIntent := (wf.effects old member).dispatch stamp selected
            have sameEffect : effect.intent.effect = old.intent.effect :=
              congrArg Intent.effect (otherIntent.1.symm.trans
                ((congrArg Dispatch.intent equal).trans stampIntent.1))
            exact different (by simpa using sameEffect)
          simp [found, g, unequal]

/-- Outcome-only transitions preserve the exact dispatch stamp. -/
theorem claims_inventory_outcome {effects : List Effect} {claims : List Dispatch}
    (before : ClaimInventory effects claims)
    (unique : (effects.map (fun effect => effect.intent.effect)).Nodup)
    {old : Effect} (member : old ∈ effects) (outcome : EffectOutcome) :
    ClaimInventory
      (effects.map (fun effect => if effect.intent.effect == old.intent.effect then
        { old with outcome := outcome } else effect)) claims := by
  apply claims_inventory_put (next := { old with outcome := outcome })
    before unique member rfl rfl
  exact before.2.2 old member

/-- Cancellation maps effects without altering their subject or dispatch. -/
theorem claims_inventory_preserve_dispatch {effects : List Effect} {claims : List Dispatch}
    (before : ClaimInventory effects claims) (f : Effect → Effect)
    (anchors : ∀ effect ∈ effects,
      (f effect).intent = effect.intent ∧ (f effect).generation = effect.generation)
    (dispatches : ∀ effect ∈ effects, (f effect).dispatch = effect.dispatch) :
    ClaimInventory (effects.map f) claims := by
  have transported := claims_inventory_map before f id anchors
    (fun _ _ => ⟨rfl, rfl⟩) (fun _ _ => rfl)
    (fun effect member => by simpa using dispatches effect member)
  simpa using transported

end Kctx.Engineering
