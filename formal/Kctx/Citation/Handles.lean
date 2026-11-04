import Kctx.Citation.Types

set_option autoImplicit false
namespace Kctx.Citation

/-- Generation is checked separately so a rotated local table reports expiry,
while a foreign audience never learns whether the requested handle exists. -/
def sameAliasAudience (left right : AliasScope) : Bool := decide
  (left.realm = right.realm ∧ left.principal = right.principal ∧
   left.session = right.session ∧ left.release = right.release ∧
   left.context = right.context)

def bindingFitsAlias (scope : AliasScope) (binding : Binding) : Bool := decide
  (binding.representation.realm = scope.realm ∧ binding.release = scope.release ∧
   binding.context = scope.context)

/-- First retained occurrence wins even for hostile raw tables. -/
def lookupAlias (entries : List AliasEntry) (handle : Handle) : Option AliasEntry :=
  match entries with
  | [] => none
  | entry :: rest =>
      if entry.handle = handle then some entry else lookupAlias rest handle

def issueAlias (table : AliasTable) (handle : Handle) (binding : Binding)
    (issuedAt expiresAt : Nat) (access : Access) : Option AliasTable :=
  if bindingFitsAlias table.scope binding = true ∧ access binding = true ∧
      issuedAt ≤ expiresAt ∧ lookupAlias table.entries handle = none then
    some { table with entries := table.entries ++ [⟨handle, binding, issuedAt, expiresAt⟩] }
  else none

/-- Expiry is inclusive: issuedAt ≤ now ≤ expiresAt. No source search, implicit
last-view state, or repair of hostile bindings participates in expansion. -/
def expandAlias (table : AliasTable) (scope : AliasScope) (handle : Handle)
    (now : Nat) (access : Access) : AliasResult :=
  if sameAliasAudience table.scope scope = false then .foreign
  else if table.scope.generation ≠ scope.generation then .expired
  else match lookupAlias table.entries handle with
    | none => .unavailable
    | some entry =>
        if bindingFitsAlias table.scope entry.binding = false then .foreign
        else if ¬ (entry.issuedAt ≤ now ∧ now ≤ entry.expiresAt) then .expired
        else if access entry.binding = false then .denied
        else .durable entry.binding

end Kctx.Citation
