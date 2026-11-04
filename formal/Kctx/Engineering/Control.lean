import Kctx.Engineering.Types

set_option autoImplicit false
namespace Kctx.Engineering

def subset {α : Type} [BEq α] (xs ys : List α) : Bool := xs.all ys.contains

def childCapabilities (grant parent ceiling : List Capability) : List Capability :=
  parent.filter (fun cap => grant.contains cap && ceiling.contains cap)

def inheritedFloor (parent callee policy : List Gate) : List Gate := parent ++ callee ++ policy

def findRun (s : State) (id : ActivationId) : Option Run := s.runs.find? (·.invocation == id)
def findEffect (s : State) (id : Id .effect) : Option Effect := s.effects.find? (·.intent.effect == id)
def putRun (s : State) (r : Run) : State :=
  { s with runs := s.runs.map (fun old => if old.invocation == r.invocation then r else old) }
def putEffect (s : State) (e : Effect) : State :=
  { s with effects := s.effects.map (fun old => if old.intent.effect == e.intent.effect then e else old) }
def retained (s : State) (objects : List ObjectId) : Bool := subset objects s.retained

def currentGrant (s : State) (r : Run) : Option Grant :=
  s.authority.grants.find? (fun g => g.run == r.id && g.binding == r.binding &&
    !g.revoked && g.validFrom ≤ s.now && s.now < g.expires)

def locallyLive (s : State) (r : Run) : Bool :=
  s.alive && r.status == .active && r.binding.realm == s.realm && r.incarnation == s.incarnation &&
  !s.authority.withdrawn.contains r.procedure.revision &&
  r.correctionGeneration == s.authority.correctionGeneration && r.policy == s.authority.policy &&
  subset s.authority.floor r.floor && (currentGrant s r).isSome

def ancestryLive (s : State) : Nat → Run → Bool
  | 0, _ => false
  | fuel + 1, r => locallyLive s r && match r.parent with
      | none => true
      | some parent => (findRun s parent).any (fun p =>
          p.id == r.id && p.binding == r.binding &&
          subset r.capabilities p.capabilities && subset p.floor r.floor && ancestryLive s fuel p)

def live (s : State) (r : Run) : Bool := ancestryLive s (s.runs.length + 1) r

def descendsFrom (s : State) (ancestor : ActivationId) : Nat → Run → Bool
  | 0, _ => false
  | fuel + 1, r => match r.parent with
    | none => false
    | some parent => parent == ancestor ||
        (match findRun s parent with | none => false | some p => descendsFrom s ancestor fuel p)

/-- There is one account per root, never a copied child allowance. -/
def charge (s : State) (root : RunId) (cost : Nat) : Option State := do
  let account ← s.budgets.find? (·.root == root)
  if cost > 0 && account.spent + cost ≤ account.limit then
    some { s with budgets := s.budgets.map (fun old =>
      if old.root == root then { old with spent := old.spent + cost } else old) }
  else none

def nodeInput (r : Run) (n : Node) : TypedValue :=
  if n.dependencies.isEmpty then r.input else
    ⟨n.input, n.dependencies.flatMap (fun d =>
      match r.control.results.find? (·.node == d.node) with
      | none => [] | some result => result.value.objects)⟩

def effectiveNodeFloor (s : State) (r : Run) (n : Node) : List Gate :=
  inheritedFloor r.floor n.floor s.authority.floor

def receiptRoleValid (s : State) (r : Run) (n : Node) (receipt : Receipt) : Bool :=
  match receipt.role with
  | .gate => receipt.output.type == .evidence
  | .procedureResult =>
      n.form == .«end» && receipt.output.type == r.procedure.outputType
  | .effectResult =>
      receipt.output.type == n.output && s.effects.any (fun e =>
        e.intent.effect == receipt.effect && e.intent.invocation == receipt.invocation &&
        e.intent.node == n.id && e.intent.binding == r.binding &&
        e.intent.procedure == r.procedure.revision && n.form == .effect e.intent.capability &&
        match receipt.outcome with
        | .success => e.outcome == .confirmed
        | .failed => e.outcome == .failed
        | .cancelled => e.outcome == .cancelledBeforeDispatch
        | .unknownEffect => e.outcome == .unknown || e.outcome == .partial
        | .blocked => false)

def receiptValid (s : State) (r : Run) (receipt : Receipt) : Bool :=
  live s r && receipt.binding == r.binding && receipt.procedure == r.procedure.revision &&
  receipt.complete && retained s receipt.objects &&
  (findNode r.procedure receipt.node).any (fun n =>
    receipt.input == nodeInput r n && subset (effectiveNodeFloor s r n) receipt.gates &&
      receiptRoleValid s r n receipt) &&
  receipt.observedAt ≤ s.now && s.now < receipt.expires && receipt.rootInput == r.input &&
  receipt.policy == s.authority.policy && receipt.outputType == receipt.output.type &&
  receipt.correctionGeneration == s.authority.correctionGeneration

def evidenceReady (s : State) (r : Run) (n : Node) : Bool :=
  s.receipts.any (fun receipt => receiptValid s r receipt && receipt.invocation == r.invocation &&
    receipt.role == .gate && receipt.node == n.id &&
    subset (effectiveNodeFloor s r n) receipt.gates &&
    receipt.outcome == .success)

def evidenceApplicable (s : State) (r : Run) (e : TypedEvidence) : Bool :=
  e.invocation == r.invocation && e.binding == r.binding && retained s e.objects &&
  retained s (e.input.objects ++ e.output.objects) && e.policy == s.authority.policy &&
  e.correctionGeneration == s.authority.correctionGeneration && s.now < e.expires

def predicatesHold (contracts : Contracts) (s : State) (r : Run)
    (predicates : List PredicateId) (value : TypedValue) : Bool :=
  predicates.all (fun pred =>
    contracts.predicates.contains ⟨pred, value.type, .evidence⟩ &&
    contracts.typedEvidence.any (fun e => evidenceApplicable s r e &&
      e.predicate == pred && e.input == value && e.output.type == .evidence))

def successful (r : Run) (id : NodeId) : Bool :=
  r.control.results.any (fun result => result.node == id && result.outcome == .success)


def capabilityAvailable (s : State) (r : Run) (n : Node) (cap : Capability) : Bool :=
  n.ceiling.contains cap && r.capabilities.contains cap &&
  (currentGrant s r).any (fun grant => grant.capabilities.contains cap)

def enabled (s : State) (r : Run) (n : Node) : Bool :=
  live s r && r.procedure.nodes.contains n &&
  !r.control.results.any (·.node == n.id) && !r.control.pending.any (·.node == n.id) &&
  (n.id == r.procedure.entry || !n.dependencies.isEmpty || !n.prerequisites.isEmpty ||
    r.control.routes.any (·.2 == n.id)) &&
  n.dependencies.all (fun d => r.control.results.any (fun result =>
    result.node == d.node && result.value.type == d.type && result.outcome == .success)) &&
  n.prerequisites.all (successful r) &&
  r.procedure.nodes.all (fun source => match source.form with
    | .select routes => !routes.contains n.id || r.control.routes.contains (source.id, n.id)
    | _ => true) &&
  n.context == r.binding.subject && subset n.scope r.binding.dependencies &&
  (match n.form with
   | .read _ => capabilityAvailable s r n .read
   | .check _ | .judge _ => capabilityAvailable s r n .check
   | .effect cap => capabilityAvailable s r n cap
   | _ => true) &&
  (nodeInput r n).type == n.input

def nodeReady (contracts : Contracts) (s : State) (r : Run) (n : Node) : Bool :=
  enabled s r n && subset n.requiredContext r.loaded &&
  predicatesHold contracts s r n.preconditions (nodeInput r n)

def recordResult (r : Run) (n : Node) (value : TypedValue) (outcome : Completion) : Run :=
  { r with control := { r.control with results := ⟨n.id, value, outcome⟩ :: r.control.results } }

def joinReady (s : State) (r : Run) (calls : List NodeId) : Bool :=
  calls.all (fun node => s.returned.any (fun result => result.parent == r.invocation &&
    result.node == node && result.binding == r.binding && result.receipt.outcome == .success &&
    (findRun s result.child).any (fun child => child.id == r.id &&
      child.parent == some r.invocation && child.status == .completed && child.binding == r.binding &&
      child.procedure.revision == result.procedure && child.input == result.input)))

/-- Pure nodes consume checked registered evidence. Effect results require a
reconciled exact effect; Call results can only be produced by returnChild. -/
def advanceNode (contracts : Contracts) (s : State) (activation : ActivationId)
    (id : NodeId) (value : TypedValue) : Option State := do
  let r ← findRun s activation
  let n ← findNode r.procedure id
  if nodeReady contracts s r n && value.type == n.output && retained s value.objects &&
      predicatesHold contracts s r n.postconditions value then
    let allowed : Bool := match n.form with
      | .read object => r.loaded.contains object && value.objects == [object]
      | .judge pred | .check pred => contracts.typedEvidence.any (fun e =>
          evidenceApplicable s r e && e.predicate == pred && e.input == nodeInput r n && e.output == value)
      | .join calls => joinReady s r calls
      | .«end» => r.control.pending.isEmpty &&
          !s.effects.any (fun e : Effect => e.intent.invocation == r.invocation &&
            (e.outcome == .prepared || e.outcome == .claimed ||
              e.outcome == .unknown || e.outcome == .partial)) &&
          predicatesHold contracts s r r.procedure.postconditions value
      | .effect cap => s.effects.any (fun e : Effect => e.intent.invocation == r.invocation &&
          e.intent.node == n.id && e.intent.binding == r.binding && e.intent.capability == cap &&
          e.outcome == .confirmed && s.receipts.any (fun receipt : Receipt =>
            receipt.role == .effectResult && receipt.effect == e.intent.effect &&
            receipt.invocation == r.invocation && receipt.node == n.id && receiptValid s r receipt &&
            receipt.outcome == .success && receipt.output == value))
      | .select _ | .call _ => false
    if allowed then
      let charged ← match n.form with | .effect _ => some s | _ => charge s r.id n.cost
      let next : Run := recordResult r n value .success
      some (putRun charged (if n.form == (NodeForm.end) && r.parent.isNone then
        { next with status := .completed } else next))
    else none
  else none

def failureRoute (n : Node) (outcome : Completion) : Option NodeId :=
  if outcome == .cancelled then n.cancellation else n.failure

def recordFailure (r : Run) (n : Node) (receipt : Receipt) : Run :=
  let next := recordResult r n receipt.output receipt.outcome
  { next with control := { next.control with routes :=
    (failureRoute n receipt.outcome).toList.map (fun target => (n.id, target)) ++ next.control.routes } }

/-- Non-success is authenticated evidence too. It cannot produce a successful
dependency or satisfy a Join, and can take only the declared non-success route. -/
def failNode (contracts : Contracts) (s : State) (activation : ActivationId)
    (id : NodeId) (receipt : Receipt) : Option State := do
  let r ← findRun s activation
  let n ← findNode r.procedure id
  if enabled s r n && receipt.invocation == r.invocation && receipt.node == n.id &&
      receipt.outcome != .success && retained s receipt.output.objects &&
      s.receipts.contains receipt && contracts.receipts.contains receipt && receiptValid s r receipt &&
      (match n.form with
       | .call _ => false
       | .effect _ => receipt.role == .effectResult
       | .«end» => receipt.role == .procedureResult
       | _ => receipt.role == .gate) then
    let charged ← charge s r.id n.cost
    some (putRun charged (recordFailure r n receipt))
  else none

def selectRoute (contracts : Contracts) (s : State) (activation : ActivationId)
    (id route : NodeId) (value : TypedValue) : Option State := do
  let r ← findRun s activation
  let n ← findNode r.procedure id
  match n.form with
  | .select routes =>
      if nodeReady contracts s r n && routes.contains route && value.type == n.output &&
          retained s value.objects && predicatesHold contracts s r n.postconditions value then
        let charged ← charge s r.id n.cost
        let next := recordResult r n value .success
        some (putRun charged { next with control := { next.control with
          routes := (id, route) :: next.control.routes } })
      else none
  | _ => none

def call (contracts : Contracts) (s : State) (parent : ActivationId) (node : NodeId)
    (invocation : ActivationId) (callee : Procedure) (binding : Binding) : Option State := do
  let p ← findRun s parent
  let n ← findNode p.procedure node
  let g ← currentGrant s p
  match n.form with
  | .call target =>
      if nodeReady contracts s p n && s.catalog.contains callee &&
          target.procedure == callee.id && target.revision == callee.revision &&
          target.input == callee.inputType && target.output == callee.outputType &&
          callee.rank < p.procedure.rank && !s.authority.withdrawn.contains callee.revision &&
          retained s callee.closure && binding == p.binding &&
          (nodeInput p n).type == callee.inputType &&
          !s.runs.any (·.invocation == invocation) then
        let next : Run := { p with
          invocation := invocation, parent := some p.invocation, procedure := callee,
          delegation := some ⟨p.invocation, p.capabilities, effectiveNodeFloor s p n⟩,
          input := nodeInput p n,
          capabilities := childCapabilities g.capabilities
            (childCapabilities p.capabilities target.delegation n.ceiling) callee.ceiling,
          floor := inheritedFloor (effectiveNodeFloor s p n) callee.floor s.authority.floor,
          loaded := [], control := ⟨[], [], []⟩ }
        if predicatesHold contracts s next callee.preconditions next.input then
          let charged ← charge s p.id n.cost
          let waiting := putRun charged { p with control := { p.control with
            pending := ⟨node, invocation, target, binding, next.input⟩ :: p.control.pending } }
          some { waiting with runs := next :: waiting.runs }
        else none
      else none
  | _ => none

def returnChild (contracts : Contracts) (s : State) (parent child : ActivationId)
    (receipt : Receipt) : Option State := do
  let p ← findRun s parent
  let c ← findRun s child
  let pending ← p.control.pending.find? (·.child == child)
  let n ← findNode p.procedure pending.node
  if live s p && live s c && c.id == p.id && c.parent == some p.invocation &&
      c.binding == p.binding && pending.binding == p.binding && pending.input == c.input &&
      pending.input == nodeInput p n &&
      pending.target.procedure == c.procedure.id && pending.target.revision == c.procedure.revision &&
      n.form == .call pending.target && c.control.pending.isEmpty &&
      c.procedure.nodes.any (fun last => last.form == .«end» && last.id == receipt.node &&
        c.control.results.any (fun result => result.node == last.id &&
          result.outcome == receipt.outcome && result.value == receipt.output)) &&
      receipt.role == .procedureResult &&
      receipt.invocation == c.invocation && receipt.rootInput == c.input &&
      receipt.output.type == pending.target.output &&
      s.receipts.contains receipt && receiptValid s c receipt &&
      (receipt.outcome != .success ||
        (predicatesHold contracts s c c.procedure.postconditions receipt.output &&
         predicatesHold contracts s p n.postconditions receipt.output)) &&
      !s.effects.any (fun e => e.intent.invocation == c.invocation &&
        (e.outcome == .prepared || e.outcome == .claimed || e.outcome == .unknown || e.outcome == .partial)) then
    let parentResult := if receipt.outcome == .success then recordResult p n receipt.output .success
      else recordFailure p n receipt
    let next := putRun (putRun s { c with status := .completed })
      { parentResult with control := { parentResult.control with
        pending := p.control.pending.filter (·.child != child) } }
    some { next with returned :=
      ⟨p.invocation, c.invocation, n.id, c.binding, c.procedure.revision, c.input, receipt.output, receipt⟩ :: s.returned }
  else none

end Kctx.Engineering
