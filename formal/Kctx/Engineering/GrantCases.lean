import Lean
import Kctx.Engineering.Environment

set_option autoImplicit false
namespace Kctx.Engineering.GrantCases
open Lean

/-! Shared executable operator-grant scenarios. Installation and revocation are
the actual `environment` transitions and the selected grant is the actual
`currentGrant`; `covers` restates the grant conjunct of `intentReady`
verbatim. Every authorization is placed in `grantAuthorizations`, so these
cases exercise the interpreter's scope, policy, retention, freshness and time
guards, not signature verification. Differential evidence for the Rust
authority kernel, not a refinement proof. -/

/-- A run is named by its run id and its binding's realm and subject. -/
structure Who where
  run : Nat
  realm : Nat := 1
  subject : Nat := 1
  deriving Inhabited

def binding (w : Who) : Binding := { (default : Binding) with realm := ⟨w.realm⟩, subject := ⟨w.subject⟩ }
def runOf (w : Who) : Run := { (default : Run) with id := ⟨w.run⟩, binding := binding w }

structure Install where
  grant : Nat
  who : Who
  resources : List Nat := [1]
  capabilities : List Capability := [.read, .write]
  budget : Nat := 10
  validFrom : Nat := 10
  expires : Nat := 100
  revoked : Bool := false
  realm : Nat := 1
  policy : Nat := 1
  envelope : Nat := 40

inductive Step where
  | install (i : Install)
  | revoke (grant : Nat)
  | advance (time : Nat)
  | query (who : Who) (capability : Capability) (resources : List Nat)

structure Scenario where
  name : String
  steps : List Step

def grantOf (i : Install) : Grant where
  id := ⟨i.grant⟩
  run := ⟨i.who.run⟩
  binding := binding i.who
  resources := i.resources.map (⟨·⟩)
  capabilities := i.capabilities
  budget := i.budget
  validFrom := i.validFrom
  expires := i.expires
  revoked := i.revoked

def authorization (i : Install) : GrantAuthorization where
  realm := ⟨i.realm⟩
  policy := ⟨i.policy⟩
  grant := grantOf i
  envelope := ⟨i.envelope⟩

def installs (steps : List Step) : List GrantAuthorization :=
  steps.filterMap (fun | .install i => some (authorization i) | _ => none)

/-- Realm 1, policy 1, time 20, envelopes 40 and 41 retained, no grants. -/
def initialAuthority : Authority := { (default : Authority) with policy := ⟨1⟩ }
def initial : State :=
  { (default : State) with realm := ⟨1⟩, now := 20, retained := [⟨40⟩, ⟨41⟩], authority := initialAuthority }

/-- The grant conjunct of `intentReady`, unchanged. -/
def covers (s : State) (r : Run) (capability : Capability) (resources : List (Id .resource)) : Bool :=
  match currentGrant s r with
  | some g => g.capabilities.contains capability && subset resources g.resources
  | none => false

def who (w : Who) (grant : Nat := 1) (rs : List Nat := [1]) : Step := .install { grant, who := w, resources := rs }
def me : Who := ⟨1, 1, 1⟩
def ask (w : Who := me) (c : Capability := .write) (rs : List Nat := [1]) : Step := .query w c rs
def i1 : Install := { grant := 1, who := me }

def scenarios : List Scenario :=
  [ ⟨"install-then-cover", [.install i1, ask, ask (c := .read), ask (rs := [])]⟩,
    ⟨"capability-not-granted", [.install i1, ask (c := .publish), ask (c := .check)]⟩,
    ⟨"resource-not-granted", [.install i1, ask (rs := [2]), ask (rs := [1, 2])]⟩,
    ⟨"other-run", [.install i1, ask ⟨2, 1, 1⟩]⟩,
    ⟨"other-binding", [.install i1, ask ⟨1, 1, 2⟩]⟩,
    ⟨"foreign-envelope-realm", [.install { i1 with realm := 2 }, ask]⟩,
    ⟨"foreign-binding-realm", [.install { i1 with who := ⟨1, 2, 1⟩ }, ask ⟨1, 2, 1⟩]⟩,
    ⟨"stale-policy", [.install { i1 with policy := 2 }, ask]⟩,
    ⟨"envelope-not-retained", [.install { i1 with envelope := 42 }, ask]⟩,
    ⟨"not-yet-valid", [.install { i1 with validFrom := 21 }, ask]⟩,
    ⟨"valid-from-now", [.install { i1 with validFrom := 20 }, ask]⟩,
    ⟨"expires-now", [.install { i1 with expires := 20 }, ask]⟩,
    ⟨"installed-revoked", [.install { i1 with revoked := true }, ask]⟩,
    ⟨"duplicate-id", [.install i1, .install { i1 with resources := [1, 2], envelope := 41 },
      ask (rs := [2])]⟩,
    ⟨"revoke-stops-cover", [.install i1, .revoke 1, ask]⟩,
    ⟨"revoked-id-not-reinstalled", [.install i1, .revoke 1, .install { i1 with envelope := 41 }, ask]⟩,
    ⟨"renewal-after-revoke", [.install i1, .revoke 1,
      .install { i1 with grant := 2, resources := [1, 2], envelope := 41 }, ask (rs := [1, 2])]⟩,
    ⟨"revoke-unknown-is-noop", [.install i1, .revoke 9, ask]⟩,
    ⟨"expiry-by-time", [.install i1, .advance 99, ask, .advance 100, ask]⟩,
    ⟨"time-never-rewinds", [.install i1, .advance 19, ask]⟩,
    ⟨"newest-grant-selected", [.install i1,
      .install { i1 with grant := 2, capabilities := [.read], envelope := 41 }, ask, ask (c := .read)]⟩,
    ⟨"newest-revoked-falls-back", [.install i1,
      .install { i1 with grant := 2, capabilities := [.read], envelope := 41 }, .revoke 2, ask]⟩,
    ⟨"newest-expired-falls-back", [.install i1,
      .install { i1 with grant := 2, capabilities := [.read], expires := 30, envelope := 41 },
      .advance 30, ask]⟩,
    ⟨"newest-not-covering-no-fallback", [.install i1,
      .install { i1 with grant := 2, resources := [2], envelope := 41 }, ask, ask (rs := [2])]⟩,
    ⟨"install-after-expiry-refused", [.advance 100, .install i1, ask]⟩,
    ⟨"two-runs-independent", [.install i1, .install { grant := 2, who := ⟨2, 1, 1⟩, envelope := 41 },
      .revoke 1, ask, ask ⟨2, 1, 1⟩]⟩ ]

def capName : Capability → String
  | .read => "read" | .check => "check" | .write => "write" | .publish => "publish"
def grantJson (g : Grant) : Json := Json.mkObj
  [("id", toJson g.id.value), ("run", toJson g.run.value),
   ("realm", toJson g.binding.realm.value), ("subject", toJson g.binding.subject.value),
   ("resources", toJson (g.resources.map (·.value))),
   ("capabilities", toJson (g.capabilities.map capName)),
   ("budget", toJson g.budget), ("valid_from", toJson g.validFrom),
   ("expires", toJson g.expires), ("revoked", toJson g.revoked)]

def whoJson (w : Who) : Json :=
  Json.mkObj [("run", toJson w.run), ("realm", toJson w.realm), ("subject", toJson w.subject)]

def capJson (c : Capability) : Json := toJson (capName c)

def stepJson (contracts : Contracts) (s : State) : Step → Json × State
  | .install i =>
      let a := authorization i
      let next := environment contracts s (.installGrant a)
      (Json.mkObj [("kind", "install"), ("realm", toJson i.realm), ("policy", toJson i.policy),
        ("envelope", toJson i.envelope), ("grant", grantJson a.grant),
        ("lean_ok", toJson next.isSome)], next.getD s)
  | .revoke g =>
      let next := environment contracts s (.revokeGrant ⟨g⟩)
      (Json.mkObj [("kind", "revoke"), ("grant", toJson g), ("lean_ok", toJson next.isSome)],
        next.getD s)
  | .advance t =>
      let next := environment contracts s (.advanceTime t)
      (Json.mkObj [("kind", "advance"), ("time", toJson t), ("lean_ok", toJson next.isSome)],
        next.getD s)
  | .query w c rs =>
      let current := currentGrant s (runOf w)
      (Json.mkObj [("kind", "query"), ("who", whoJson w), ("capability", capJson c),
        ("resources", toJson rs),
        ("current_grant", match current with | none => .null | some g => toJson g.id.value),
        ("covers", toJson (covers s (runOf w) c (rs.map (⟨·⟩))))], s)

def run (steps : List Step) : List Json := Id.run do
  let contracts : Contracts := { (default : Contracts) with grantAuthorizations := installs steps }
  let mut s := initial
  let mut out := #[]
  for step in steps do
    let (json, next) := stepJson contracts s step
    out := out.push (json.setObjVal! "now_after" (toJson next.now) |>.setObjVal!
      "grants_after" (toJson (next.authority.grants.map grantJson)))
    s := next
  return out.toList

end Kctx.Engineering.GrantCases

open Kctx.Engineering.GrantCases in
def main : IO UInt32 := do
  let doc := Lean.Json.mkObj
    [("schema", "writfold-grant-lean-cases/1"),
     ("initial", Lean.Json.mkObj [("realm", 1), ("policy", 1), ("now", 20),
       ("retained", Lean.toJson [40, 41])]),
     ("scenarios", Lean.Json.arr (scenarios.map (fun s =>
        Lean.Json.mkObj [("name", Lean.toJson s.name), ("steps", Lean.Json.arr (run s.steps).toArray)])).toArray)]
  IO.println doc.pretty
  return 0
