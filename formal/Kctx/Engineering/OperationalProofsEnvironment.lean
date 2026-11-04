import Kctx.Engineering.OperationalInvariant
import Kctx.Engineering.Actions
import Kctx.Engineering.ControlProofs
import Kctx.Engineering.OperationalProofs
import Kctx.Engineering.OperationalProofsReducer

set_option autoImplicit false
namespace Kctx.Engineering

private theorem run_frame {c : Contracts} {s t : OperationalState} {r : Run}
    (h : RunRepresentation c s r)
    (catalog : t.catalog = s.catalog) (realm : t.realm = s.realm)
    (retained : t.retained = s.retained) (grants : t.authority.grants = s.authority.grants)
    (budgets : t.budgets = s.budgets) (runs : t.runs = s.runs) : RunRepresentation c t r := by
  cases h
  constructor <;> simp_all [catalog, realm, retained, grants, budgets, runs]

private theorem effect_frame {s t : OperationalState} {e : Effect}
    (h : EffectRepresentation s e) (runs : t.runs = s.runs)
    (retained : t.retained = s.retained)
    (grants : t.authority.grants = s.authority.grants) : EffectRepresentation t e := by
  rcases h with ⟨a,b,d,g,p⟩
  constructor <;> simp_all [runs, retained, grants]

private theorem receipt_frame {c : Contracts} {s t : OperationalState} {r : Receipt}
    (h : ReceiptRepresentation c s r) (runs : t.runs = s.runs)
    (retained : t.retained = s.retained) : ReceiptRepresentation c t r := by
  rcases h with ⟨a,b,d,e⟩
  constructor <;> simp_all [runs, retained]

/-- Volatile or projection bookkeeping preserves historical representation and claim inventory. -/
private theorem frame {c : Contracts} {s t : OperationalState}
    (wf : OperationalWellFormed c s)
    (realm : t.realm = s.realm) (knowledge : t.knowledge = s.knowledge)
    (catalog : t.catalog = s.catalog) (release : t.catalogRelease = s.catalogRelease)
    (corpus : t.corpusPin = s.corpusPin) (activity : t.activityPin = s.activityPin)
    (projection : t.projectionEnvelope = s.projectionEnvelope)
    (visibility : t.visibilityEnvelope = s.visibilityEnvelope)
    (grants : t.authority.grants = s.authority.grants)
    (floor : t.authority.floor = s.authority.floor)
    (retained : t.retained = s.retained) (runs : t.runs = s.runs)
    (budgets : t.budgets = s.budgets) (effects : t.effects = s.effects)
    (ownership : t.ownership = s.ownership) (candidates : t.candidates = s.candidates)
    (captures : t.captures = s.captures) (receipts : t.receipts = s.receipts)
    (returned : t.returned = s.returned) (claims : t.claims = s.claims)
    (time : s.now ≤ t.now) :
    OperationalWellFormed c t := by
  refine {
    portfolio := by simpa only [knowledge] using wf.portfolio
    grants_unique := by simpa only [grants] using wf.grants_unique
    governing_floor := by simpa only [floor] using wf.governing_floor
    corpus_binding := by simpa only [knowledge, corpus] using wf.corpus_binding
    corpus_realm := by simpa only [corpus, realm] using wf.corpus_realm
    corpus_family := by simpa only [corpus] using wf.corpus_family
    activity_realm := by simpa only [activity, realm] using wf.activity_realm
    activity_family := by simpa only [activity] using wf.activity_family
    envelopes_retained := by
      simpa only [corpus, activity, projection, visibility, retained] using wf.envelopes_retained
    catalog_release := by simpa only [release, knowledge] using wf.catalog_release
    catalog_unique := by simpa only [catalog] using wf.catalog_unique
    catalog_authority := by simpa only [catalog, retained] using wf.catalog_authority
    activations_unique := by simpa only [runs] using wf.activations_unique
    accounts_unique := by simpa only [budgets] using wf.accounts_unique
    accounts_bounded := by simpa only [budgets] using wf.accounts_bounded
    runs := ?_
    effects_unique := by simpa only [effects] using wf.effects_unique
    effects := ?_
    ownership_unique := by simpa only [ownership] using wf.ownership_unique
    ownership := by simpa only [ownership, runs] using wf.ownership
    candidates_unique := by simpa only [candidates] using wf.candidates_unique
    candidates := by simpa only [candidates, retained] using wf.candidates
    captures_unique := by simpa only [captures] using wf.captures_unique
    captures := ?_
    receipts := ?_
    returned := by simpa only [returned, receipts, runs] using wf.returned
    claims_unique := by simpa only [claims] using wf.claims_unique
    claim_effects := by simpa only [claims, effects] using wf.claim_effects
    dispatch_inventory := by simpa only [claims, effects] using wf.dispatch_inventory }
  · intro r hr
    exact run_frame (wf.runs r (by simpa only [runs] using hr))
      catalog realm retained grants budgets runs
  · intro e he
    exact effect_frame (wf.effects e (by simpa only [effects] using he)) runs retained grants
  · intro capture hc
    obtain ⟨qualified, inRealm, past, material⟩ :=
      wf.captures capture (by simpa only [captures] using hc)
    exact ⟨qualified, inRealm.trans realm.symm, Nat.le_trans past time,
      by simpa only [retained] using material⟩
  · intro receipt hr
    exact receipt_frame (wf.receipts receipt (by simpa only [receipts] using hr)) runs retained

/-- Grant updates retain historical root, binding, capability and resource scope. -/
private theorem grants_map {c : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed c s) (f : Grant → Grant)
    (ids : ∀ g, (f g).id = g.id) (roots : ∀ g, (f g).run = g.run)
    (bindings : ∀ g, (f g).binding = g.binding)
    (caps : ∀ g, (f g).capabilities = g.capabilities)
    (resources : ∀ g, (f g).resources = g.resources) :
    OperationalWellFormed c { s with authority := { s.authority with grants := s.authority.grants.map f } } := by
  refine { wf with grants_unique := ?_, runs := ?_, effects := ?_, receipts := ?_ }
  · simpa only [List.map_map, Function.comp_def, ids] using wf.grants_unique
  · intro r hr
    have old := wf.runs r hr
    refine { old with grant_bound := ?_ }
    obtain ⟨g, hg, root, binding, bound⟩ := old.grant_bound
    exact ⟨f g, List.mem_map.mpr ⟨g, hg, rfl⟩, (roots g).trans root,
      (bindings g).trans binding, by simpa only [caps] using bound⟩
  · intro e he
    have old := wf.effects e he
    refine { old with grant_bound := ?_ }
    obtain ⟨g, hg, root, binding, capability, footprint⟩ := old.grant_bound
    exact ⟨f g, List.mem_map.mpr ⟨g, hg, rfl⟩, (roots g).trans root,
      (bindings g).trans binding, by simpa only [caps] using capability,
      by simpa only [resources] using footprint⟩
  · intro r hr
    apply receipt_frame (wf.receipts r hr) <;> rfl

private theorem effects_map {c : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed c s) (f : Effect → Effect)
    (intent : ∀ e, (f e).intent = e.intent)
    (generation : ∀ e, (f e).generation = e.generation)
    (dispatch : ∀ e, (f e).dispatch = e.dispatch) :
    OperationalWellFormed c { s with effects := s.effects.map f } := by
  refine { wf with
    runs := ?_
    effects_unique := ?_
    effects := ?_
    receipts := ?_
    claim_effects := ?_
    dispatch_inventory := ?_ }
  · intro r hr
    apply run_frame (wf.runs r hr) <;> rfl
  · simpa only [List.map_map, Function.comp_def, intent] using wf.effects_unique
  · intro e he
    obtain ⟨old, hold, rfl⟩ := List.mem_map.mp he
    have rep := wf.effects old hold
    exact {
      activation := by simpa only [intent] using rep.activation
      closure := by simpa only [intent] using rep.closure
      dispatch := by simpa only [intent, generation, dispatch] using rep.dispatch
      grant_bound := by simpa only [intent] using rep.grant_bound
      provider_binding := by simpa only [intent] using rep.provider_binding }
  · intro r hr
    apply receipt_frame (wf.receipts r hr) <;> rfl
  · intro claim member
    obtain ⟨effect, present, sameIntent, sameGeneration⟩ := wf.claim_effects claim member
    exact ⟨f effect, List.mem_map.mpr ⟨effect, present, rfl⟩,
      (intent effect).trans sameIntent, (generation effect).trans sameGeneration⟩
  · intro effect present claim stamp
    obtain ⟨old, member, rfl⟩ := List.mem_map.mp present
    exact wf.dispatch_inventory old member claim ((dispatch old).symm.trans stamp)

private theorem effect_unknown {c : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed c s) (current : Effect) (member : current ∈ s.effects) :
    OperationalWellFormed c { s with effects := s.effects.map (fun old =>
      if old.intent.effect == current.intent.effect then
        { current with outcome := .unknown } else old) } := by
  have same : s.effects.map (fun old =>
      if old.intent.effect == current.intent.effect then
        { current with outcome := .unknown } else old) =
      s.effects.map (fun old =>
        if old.intent.effect == current.intent.effect then
          { old with outcome := .unknown } else old) := by
    apply List.map_congr_left
    intro old hold
    split
    next selected =>
      have equal := member_eq_of_unique_key wf.effects_unique hold member
        (beq_iff_eq.mp selected)
      subst old
      rfl
    next => rfl
  rw [same]
  apply effects_map wf <;> intro effect <;> dsimp only <;> split <;> rfl

private theorem ownership_map {c : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed c s) (f : Reservation → Reservation)
    (resource : ∀ o, (f o).resource = o.resource)
    (holder : ∀ o, (f o).holder = o.holder) (owner : ∀ o, (f o).owner = o.owner) :
    OperationalWellFormed c { s with ownership := s.ownership.map f } := by
  refine { wf with
    runs := ?_
    effects := ?_
    ownership_unique := ?_
    ownership := ?_
    receipts := ?_ }
  · intro r hr
    apply run_frame (wf.runs r hr) <;> rfl
  · intro e he
    apply effect_frame (wf.effects e he) <;> rfl
  · simpa only [List.map_map, Function.comp_def, resource] using wf.ownership_unique
  · intro o ho
    obtain ⟨old, hold, rfl⟩ := List.mem_map.mp ho
    simpa only [holder, owner] using wf.ownership old hold
  · intro r hr
    apply receipt_frame (wf.receipts r hr) <;> rfl

private theorem floor_change {c : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed c s) (floor : List Gate)
    (bound : ∀ gate ∈ c.kernelFloor, gate ∈ floor) :
    OperationalWellFormed c { s with authority := { s.authority with floor := floor } } := by
  refine { wf with governing_floor := bound, runs := ?_, effects := ?_, receipts := ?_ }
  · intro r hr
    apply run_frame (wf.runs r hr) <;> rfl
  · intro e he
    apply effect_frame (wf.effects e he) <;> rfl
  · intro r hr
    apply receipt_frame (wf.receipts r hr) <;> rfl

private theorem activity_change {c : Contracts} {s : OperationalState}
    (wf : OperationalWellFormed c s) (pin : Ledger.NativePin)
    (realm : pin.realm = s.realm) (family : pin.family = .activity)
    (retained : pin.envelope ∈ s.retained) :
    OperationalWellFormed c { s with activityPin := pin } := by
  refine { wf with
    activity_realm := realm
    activity_family := family
    envelopes_retained := ?_
    runs := ?_
    effects := ?_
    receipts := ?_ }
  · intro object member
    simp only [List.mem_cons, List.mem_singleton] at member
    rcases member with h | h | h | h | h
    · subst object; exact wf.envelopes_retained _ (by simp)
    · subst object; exact retained
    · subst object; exact wf.envelopes_retained _ (by simp)
    · subst object; exact wf.envelopes_retained _ (by simp)
    · cases h
  · intro r hr
    apply run_frame (wf.runs r hr) <;> rfl
  · intro e he
    apply effect_frame (wf.effects e he) <;> rfl
  · intro r hr
    apply receipt_frame (wf.receipts r hr) <;> rfl

/-- Checked initialization establishes every historical field from an empty history. -/
theorem init_operational_wellFormed {contracts : Contracts} {input : InitialInputs} {s : State}
    (h : init contracts input = some s) : OperationalWellFormed contracts s.toOperationalState := by
  unfold init at h
  split at h
  next valid =>
    cases h
    simp only [initialInputsValid, Bool.and_eq_true, beq_iff_eq,
      subset, List.all_eq_true, List.contains_iff_mem] at valid
    obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨portfolio, grants⟩, floor⟩, corpus⟩, corpusRealm⟩,
      corpusFamily⟩, activityRealm⟩, activityFamily⟩, envelopes⟩, _⟩, _⟩, _⟩, _⟩ := valid
    refine {
      portfolio := portfolio, grants_unique := grants, governing_floor := floor
      corpus_binding := corpus, corpus_realm := corpusRealm, corpus_family := corpusFamily
      activity_realm := activityRealm, activity_family := activityFamily
      envelopes_retained := envelopes, catalog_release := rfl
      catalog_unique := ?_, catalog_authority := ?_
      activations_unique := ?_, accounts_unique := ?_, accounts_bounded := ?_
      runs := ?_, effects_unique := ?_, effects := ?_
      ownership_unique := ?_, ownership := ?_
      candidates_unique := ?_, candidates := ?_
      captures_unique := ?_, captures := ?_, receipts := ?_, returned := ?_
      claims_unique := ?_, claim_effects := ?_, dispatch_inventory := ?_ }
    all_goals first
      | (change ([] : List _).Nodup; exact List.nodup_nil)
      | (intro x hx; change x ∈ ([] : List _) at hx; cases hx)
  next => contradiction

private theorem environmentBody_preserves {contracts : Contracts} {s t : State}
    {action : Environment} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : environmentBody contracts s action = some t) :
    OperationalWellFormed contracts t.toOperationalState := by
  cases action with
  | advanceTime time =>
      simp only [environmentBody] at h
      split at h
      next monotone => cases h; apply frame wf <;> first | rfl | exact monotone
      next => contradiction
  | revokeGrant id =>
      simp only [environmentBody, Option.some.injEq] at h
      subst t
      apply grants_map wf <;> intro g <;> split <;> rfl
  | changePolicy id floor =>
      simp only [environmentBody] at h
      split at h
      next bound =>
        cases h
        have updated := floor_change wf floor (by
          simpa only [subset, List.all_eq_true, List.contains_iff_mem] using bound)
        apply frame updated <;> first | rfl | exact Nat.le_refl _
      next => contradiction
  | withdrawProcedure revision =>
      cases h
      apply frame wf <;> first | rfl | exact Nat.le_refl _
  | correctKnowledge =>
      cases h
      apply frame wf <;> first | rfl | exact Nat.le_refl _
  | advanceUpstream revision =>
      cases h
      apply frame wf <;> first | rfl | exact Nat.le_refl _
  | changeProviderState revision producer consumer =>
      cases h
      apply frame wf <;> first | rfl | exact Nat.le_refl _
  | completeExternalEffect event producer consumer =>
      simp only [environmentBody, completeExternalEffect] at h
      operational_success h
      all_goals apply frame wf <;> first | rfl | exact Nat.le_refl _
  | loseReply id =>
      cases he : findEffect s id with
      | none => simp [environmentBody, he] at h
      | some current =>
          cases hd : current.dispatch with
          | none => simp [environmentBody, he, hd] at h
          | some dispatch =>
              simp only [environmentBody, he, hd, bind, Option.bind] at h
              split at h
              · split at h
                next unknown =>
                  cases h
                  have member : current ∈ s.effects :=
                    List.mem_of_find?_eq_some he
                  have mapped := effect_unknown wf current member
                  apply frame mapped <;> first | rfl | simpa only [putEffect, hd] | exact Nat.le_refl _
                next known => cases h; apply frame wf <;> first | rfl | exact Nat.le_refl _
              · contradiction
  | terminateProcess resource generation processGeneration =>
      simp only [environmentBody] at h
      split at h
      · cases h
        apply ownership_map wf <;> intro owner <;> split <;> rfl
      · contradiction

private theorem environmentBody_fixed {contracts : Contracts} {s t : State}
    {action : Environment} (h : environmentBody contracts s action = some t) :
    t.realm = s.realm ∧ t.knowledge = s.knowledge := by
  cases action <;> simp only [environmentBody, completeExternalEffect] at h
  all_goals operational_success h
  all_goals exact ⟨rfl, rfl⟩

/-- Every external observation preserves historical representation, not live authority. -/
theorem environment_preserves_operationalWellFormed {contracts : Contracts} {s t : State}
    {action : Environment} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : environment contracts s action = some t) :
    OperationalWellFormed contracts t.toOperationalState := by
  cases hb : environmentBody contracts s action with
  | none => simp [environment, hb] at h
  | some next =>
      simp only [environment, hb, bind, Option.bind, Option.some.injEq] at h
      subst t
      have body := environmentBody_preserves wf hb
      have stable := environmentBody_fixed hb
      apply frame body <;> first
        | rfl | exact stable.1.symm | exact stable.2.symm | exact Nat.le_refl _

private theorem project_preserves {contracts : Contracts} {s t : State}
    {pin : Ledger.NativePin} {heads : List Ledger.Head} {epoch : Nat}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : project contracts s pin heads epoch = some t) :
    OperationalWellFormed contracts t.toOperationalState := by
  unfold project at h
  split at h
  next guard =>
    cases h
    simp only [Bool.and_eq_true, beq_iff_eq, List.contains_iff_mem] at guard
    have updated := activity_change wf pin guard.1.1.1.1.2 guard.1.1.1.2 guard.1.1.2
    apply frame updated <;> first | rfl | exact Nat.le_refl _
  next => contradiction

private theorem acquireCut_preserves {contracts : Contracts} {s t : State} {id : Id .cut}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : acquireCut contracts s id = some t) :
    OperationalWellFormed contracts t.toOperationalState := by
  unfold acquireCut at h
  split at h
  · simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨ledger, _, h⟩ := h
    cases h
    exact wf
  · contradiction

private theorem releaseReadCut_preserves {contracts : Contracts} {s t : State} {id : Id .cut}
    (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : releaseReadCut s id = some t) :
    OperationalWellFormed contracts t.toOperationalState := by
  simp only [releaseReadCut, bind, Option.bind_eq_some_iff] at h
  obtain ⟨ledger, _, h⟩ := h
  cases h
  exact wf

private theorem crash_preserves {contracts : Contracts} {s : State}
    (wf : OperationalWellFormed contracts s.toOperationalState) :
    OperationalWellFormed contracts (crash s).toOperationalState := by
  have mapped := effects_map wf
    (fun e => if e.outcome == .prepared || e.outcome == .claimed then
      { e with outcome := .unknown } else e)
    (by intro e; dsimp only; split <;> rfl)
    (by intro e; dsimp only; split <;> rfl)
    (by intro e; dsimp only; split <;> rfl)
  apply frame mapped <;> first | rfl | exact Nat.le_refl _

/-- Local cuts and recovery preserve the historical graph. Crash marks prepared
and claimed effects unknown without changing their intent or claim inventory. -/
theorem localAction_preserves_operationalWellFormed {contracts : Contracts} {s t : State}
    {action : Action} (wf : OperationalWellFormed contracts s.toOperationalState)
    (h : localAction contracts s action = some t) :
    OperationalWellFormed contracts t.toOperationalState := by
  cases action <;> simp only [localAction] at h
  all_goals first
    | contradiction
    | exact project_preserves wf h
    | exact acquireCut_preserves wf h
    | exact releaseReadCut_preserves wf h
    | (cases h; exact crash_preserves wf)
    | (unfold recover at h; split at h
       · cases h; apply frame wf <;> first | rfl | exact Nat.le_refl _
       · contradiction)

end Kctx.Engineering
