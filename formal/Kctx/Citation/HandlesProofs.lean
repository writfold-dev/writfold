import Kctx.Citation.Handles

set_option autoImplicit false
namespace Kctx.Citation

theorem lookupAlias_append_retained (entries extra : List AliasEntry) (handle : Handle)
    (entry : AliasEntry) (found : lookupAlias entries handle = some entry) :
    lookupAlias (entries ++ extra) handle = some entry := by
  revert found
  induction entries with
  | nil => intro found; simp [lookupAlias] at found
  | cons first rest ih =>
      intro found
      by_cases same : first.handle = handle
      · simpa [lookupAlias, same] using found
      · simp only [lookupAlias, same, ↓reduceIte] at found
        simpa [lookupAlias, same] using ih found

theorem issueAlias_retains_scope (table result : AliasTable) (handle : Handle)
    (binding : Binding) (issuedAt expiresAt : Nat) (access : Access)
    (issued : issueAlias table handle binding issuedAt expiresAt access = some result) :
    result.scope = table.scope := by
  unfold issueAlias at issued
  split at issued
  · cases issued
    rfl
  · contradiction

theorem issueAlias_preserves_mapping (table result : AliasTable) (newHandle oldHandle : Handle)
    (binding : Binding) (issuedAt expiresAt : Nat) (access : Access) (oldEntry : AliasEntry)
    (issued : issueAlias table newHandle binding issuedAt expiresAt access = some result)
    (retained : lookupAlias table.entries oldHandle = some oldEntry) :
    lookupAlias result.entries oldHandle = some oldEntry := by
  unfold issueAlias at issued
  split at issued
  · cases issued
    exact lookupAlias_append_retained _ _ _ _ retained
  · contradiction

theorem issueAlias_duplicate_refused (table : AliasTable) (handle : Handle)
    (binding : Binding) (issuedAt expiresAt : Nat) (access : Access) (entry : AliasEntry)
    (retained : lookupAlias table.entries handle = some entry) :
    issueAlias table handle binding issuedAt expiresAt access = none := by
  simp [issueAlias, retained]

/-- Every durable result is the exact retained binding, not a reconstructed or
last-view-derived citation, and is authorized at the instant of expansion. -/
theorem expandAlias_exact_agreement (table : AliasTable) (scope : AliasScope)
    (handle : Handle) (now : Nat) (access : Access) (binding : Binding)
    (success : expandAlias table scope handle now access = .durable binding) :
    ∃ entry, lookupAlias table.entries handle = some entry ∧ entry.binding = binding ∧
      sameAliasAudience table.scope scope = true ∧
      table.scope.generation = scope.generation ∧
      bindingFitsAlias table.scope entry.binding = true ∧
      entry.issuedAt ≤ now ∧ now ≤ entry.expiresAt ∧ access binding = true := by
  unfold expandAlias at success
  split at success
  · contradiction
  · rename_i audience
    split at success
    · contradiction
    · rename_i generation
      split at success
      · contradiction
      · rename_i entry retained
        split at success
        · contradiction
        · rename_i fits
          split at success
          · contradiction
          · rename_i lifetime
            split at success
            · contradiction
            · rename_i allowed
              cases success
              exact ⟨entry, retained, rfl, by simpa using audience,
                by simpa using generation, by simpa using fits,
                (by simpa using lifetime : entry.issuedAt ≤ now ∧ now ≤ entry.expiresAt).1,
                (by simpa using lifetime : entry.issuedAt ≤ now ∧ now ≤ entry.expiresAt).2,
                by simpa using allowed⟩

/-- The declared scope conclusion is field identity, not just the value of the
same executable audience predicate used in the expansion guard. -/
theorem expandAlias_scope_identity (table : AliasTable) (scope : AliasScope)
    (handle : Handle) (now : Nat) (access : Access) (binding : Binding)
    (success : expandAlias table scope handle now access = .durable binding) :
    table.scope = scope ∧ binding.representation.realm = scope.realm ∧
      binding.release = scope.release ∧ binding.context = scope.context := by
  obtain ⟨entry, _, bound, audience, generation, fits, _, _, _⟩ :=
    expandAlias_exact_agreement table scope handle now access binding success
  have same : table.scope = scope := by
    cases ht : table.scope <;> cases hs : scope <;> simp_all [sameAliasAudience]
  subst binding
  have linked : entry.binding.representation.realm = table.scope.realm ∧
      entry.binding.release = table.scope.release ∧ entry.binding.context = table.scope.context := by
    simpa only [bindingFitsAlias, decide_eq_true_eq] using fits
  rw [same] at linked
  exact ⟨same, linked⟩

theorem expandAlias_foreign_refused (table : AliasTable) (scope : AliasScope)
    (handle : Handle) (now : Nat) (access : Access)
    (foreign : sameAliasAudience table.scope scope = false) :
    expandAlias table scope handle now access = .foreign := by
  simp [expandAlias, foreign]

theorem expandAlias_generation_refused (table : AliasTable) (scope : AliasScope)
    (handle : Handle) (now : Nat) (access : Access)
    (audience : sameAliasAudience table.scope scope = true)
    (generation : table.scope.generation ≠ scope.generation) :
    expandAlias table scope handle now access = .expired := by
  simp [expandAlias, audience, generation]

theorem expandAlias_revoked_refused (table : AliasTable) (scope : AliasScope)
    (handle : Handle) (now : Nat) (access : Access) (binding : Binding)
    (revoked : access binding = false) :
    expandAlias table scope handle now access ≠ .durable binding := by
  intro success
  obtain ⟨_, _, _, _, _, _, _, _, allowed⟩ :=
    expandAlias_exact_agreement table scope handle now access binding success
  simp [revoked] at allowed

theorem expandAlias_expired_refused (table : AliasTable) (scope : AliasScope)
    (handle : Handle) (now : Nat) (access : Access) (entry : AliasEntry)
    (retained : lookupAlias table.entries handle = some entry)
    (expired : ¬ (entry.issuedAt ≤ now ∧ now ≤ entry.expiresAt)) (binding : Binding) :
    expandAlias table scope handle now access ≠ .durable binding := by
  intro success
  obtain ⟨actual, found, _, _, _, _, issued, ends, _⟩ :=
    expandAlias_exact_agreement table scope handle now access binding success
  have equal : actual = entry := Option.some.inj (found.symm.trans retained)
  subst actual
  exact expired ⟨issued, ends⟩

theorem expandAlias_revoked_denied (table : AliasTable) (scope : AliasScope)
    (handle : Handle) (now : Nat) (access : Access) (entry : AliasEntry)
    (audience : sameAliasAudience table.scope scope = true)
    (generation : table.scope.generation = scope.generation)
    (retained : lookupAlias table.entries handle = some entry)
    (fits : bindingFitsAlias table.scope entry.binding = true)
    (lifetime : entry.issuedAt ≤ now ∧ now ≤ entry.expiresAt)
    (revoked : access entry.binding = false) :
    expandAlias table scope handle now access = .denied := by
  simp [expandAlias, audience, generation, retained, fits, lifetime, revoked]

theorem expandAlias_lifetime_expired (table : AliasTable) (scope : AliasScope)
    (handle : Handle) (now : Nat) (access : Access) (entry : AliasEntry)
    (audience : sameAliasAudience table.scope scope = true)
    (generation : table.scope.generation = scope.generation)
    (retained : lookupAlias table.entries handle = some entry)
    (fits : bindingFitsAlias table.scope entry.binding = true)
    (expired : ¬ (entry.issuedAt ≤ now ∧ now ≤ entry.expiresAt)) :
    expandAlias table scope handle now access = .expired := by
  simp [expandAlias, audience, generation, retained, fits, expired]

end Kctx.Citation
