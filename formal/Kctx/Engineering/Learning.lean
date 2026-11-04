import Kctx.Engineering.Core

set_option autoImplicit false
namespace Kctx.Engineering

def resolve (contracts : Contracts) (s : State) (p : Procedure) : Option State :=
  if contracts.packages.contains p && contracts.packageReleases.contains (s.catalogRelease, p) &&
      retained s p.closure &&
      s.catalogRelease == s.knowledge.kb.snapshot && !s.catalog.any (·.revision == p.revision) &&
      validateProcedure s.catalog contracts.predicates p &&
      subset s.authority.floor p.floor then
    some { s with catalog := p :: s.catalog }
  else none

def activate (contracts : Contracts) (s : State) (run : Run) (budget : Nat) : Option State :=
  if s.catalog.contains run.procedure && live s run && run.parent.isNone && run.delegation.isNone &&
      run.binding.realm == s.realm && run.control == ⟨[], [], []⟩ && run.loaded.isEmpty &&
      retained s run.procedure.closure && run.binding.inputType == run.procedure.inputType &&
      run.input.type == run.procedure.inputType && retained s run.input.objects &&
      predicatesHold contracts s run run.procedure.preconditions run.input &&
      subset run.capabilities run.procedure.ceiling &&
      (match currentGrant s run with
       | some g => subset run.capabilities g.capabilities && budget ≤ g.budget
       | none => false) &&
      subset run.procedure.floor run.floor && budget > 0 &&
      !s.budgets.any (·.root == run.id) && !s.runs.any (·.id == run.id) &&
      run.stream != s.controlStream &&
      !s.runs.any (·.stream == run.stream) && !s.candidates.any (·.stream == run.stream) &&
      !s.runs.any (·.invocation == run.invocation) then
    some { s with runs := run :: s.runs, budgets := ⟨run.id, budget, 0⟩ :: s.budgets }
  else none

def loadResource (s : State) (run : ActivationId) (object : ObjectId) : Option State := do
  let r ← findRun s run
  if live s r && r.procedure.closure.contains object && s.retained.contains object &&
      !r.loaded.contains object then
    let charged ← charge s r.id 1
    some (putRun charged { r with loaded := object :: r.loaded })
  else none

/-- Migration is explicit and keeps old receipts and effect identities intact.
It cannot rebind an invocation while an external effect is unsettled. -/
def rebind (contracts : Contracts) (s : State) (run : ActivationId) (p : Procedure) (binding : Binding)
    (invocation : ActivationId) : Option State := do
  let r ← findRun s run
  let g ← currentGrant s { r with binding := binding }
  if s.alive && (r.status == .active || r.status == .cancelled) &&
      s.catalog.contains p && retained s p.closure &&
      contracts.migrations.contains ⟨r.id, r.binding, binding, s.incarnation, s.world.provider⟩ &&
      !s.ownership.any (·.owner == r.id) &&
      !s.authority.withdrawn.contains p.revision && binding.inputType == p.inputType &&
      binding.realm == s.realm && r.input.type == p.inputType &&
      !s.runs.any (·.invocation == invocation) && r.parent.isNone &&
      !s.runs.any (fun child => child.id == r.id && child.parent.isSome && child.status == .active) &&
      !s.effects.any (fun e => e.intent.run == r.id &&
        (e.outcome == .claimed || e.outcome == .unknown || e.outcome == .partial)) then
    let next : Run := { r with
      procedure := p
      binding := binding
      invocation := invocation
      incarnation := s.incarnation
      status := .active
      capabilities := childCapabilities g.capabilities r.capabilities p.ceiling
      floor := inheritedFloor r.floor p.floor s.authority.floor
      correctionGeneration := s.authority.correctionGeneration
      policy := s.authority.policy
      design := none
      loaded := []
      control := ⟨[], [], []⟩ }
    if predicatesHold contracts s next p.preconditions next.input then
      let charged ← charge s r.id 1
      let old : State := putRun charged { r with status := .retired }
      some { old with
        runs := next :: old.runs
        effects := old.effects.map (fun (e : Effect) =>
          if e.intent.run == r.id && e.outcome == .prepared then
            { e with outcome := .cancelledBeforeDispatch } else e) }
    else none
  else none

def resumeAuthorizationValid (s : State) (r : Run) (a : ResumeAuthorization) : Bool :=
  a.activation == r.invocation && a.root == r.id && a.procedure == r.procedure.revision &&
  a.binding == r.binding && a.input == r.input && a.control == r.control &&
  a.previousIncarnation == r.incarnation && a.incarnation == s.incarnation &&
  a.policy == s.authority.policy && a.correctionGeneration == s.authority.correctionGeneration &&
  a.provider == s.world.provider && a.validFrom ≤ s.now && s.now < a.expires

/-- Recovery does not migrate inputs, control, invocation or logical effects.
The qualified observation revalidates that exact retained state; parent
activations must resume first, and all old owned trees must be actually quiet. -/
def resume (contracts : Contracts) (s : State) (activation : ActivationId) : Option State := do
  let r ← findRun s activation
  let g ← currentGrant s r
  let parent := r.parent.bind (findRun s)
  let parentCapabilities := (parent.map Run.capabilities).getD r.capabilities
  let parentFloor := (parent.map Run.floor).getD []
  let next : Run := { r with
    incarnation := s.incarnation
    capabilities := r.capabilities.filter (fun cap =>
      g.capabilities.contains cap && r.procedure.ceiling.contains cap && parentCapabilities.contains cap)
    floor := inheritedFloor r.floor r.procedure.floor (s.authority.floor ++ parentFloor)
    correctionGeneration := s.authority.correctionGeneration
    policy := s.authority.policy
    loaded := [] }
  if s.alive && r.status == .active && r.incarnation != s.incarnation &&
      s.catalog.contains r.procedure && contracts.packages.contains r.procedure &&
      retained s r.procedure.closure &&
      contracts.resumptions.any (resumeAuthorizationValid s r) &&
      (r.parent.isNone || parent.any (fun p => live s p && p.id == r.id && p.binding == r.binding)) &&
      s.ownership.all (fun o => o.owner != r.id ||
        (o.children == 0 && contracts.quiescent.contains
          (o.resource, o.generation, o.processGeneration))) &&
      live s next && predicatesHold contracts s next r.procedure.preconditions r.input then
    let charged ← charge s r.id 1
    some (putRun charged next)
  else none

def reopen (s : State) (run : ActivationId) (command : DesignReopenCommand) : Option State := do
  let r ← findRun s run
  let d ← r.design
  if live s r then do
    let next ← reopenDesign d command
    some (putRun s { r with design := some next })
  else none

def cancel (s : State) (run : ActivationId) : Option State := do
  let r ← findRun s run
  if r.status == .active then
    let cancelled : State := { s with runs := s.runs.map (fun (child : Run) =>
      if child.status == .active &&
          (child.invocation == r.invocation || descendsFrom s r.invocation s.runs.length child) then
        { child with status := .cancelled }
      else child) }
    some { cancelled with effects := s.effects.map (fun (e : Effect) =>
      if e.outcome == .prepared &&
          (findRun cancelled e.intent.invocation).any (fun (child : Run) => child.status == .cancelled) then
        { e with outcome := .cancelledBeforeDispatch } else e) }
  else none

def propose (s : State) (candidate : Candidate) : Option State :=
  if s.alive && !candidate.evaluated && !candidate.reviewed && retained s candidate.evidence &&
      candidate.stream != s.controlStream &&
      !s.runs.any (·.stream == candidate.stream) &&
      !s.candidates.any (·.stream == candidate.stream) &&
      !s.candidates.any (·.id == candidate.id) then
    some { s with candidates := candidate :: s.candidates }
  else none

def evaluate (contracts : Contracts) (s : State) (id : Id .candidate) : Option State := do
  let candidate ← s.candidates.find? (·.id == id)
  if !candidate.evaluated && retained s (candidate.rubric :: candidate.trials) &&
      !candidate.trials.isEmpty &&
      contracts.evaluations.contains candidate.toCandidateMaterial then
    some { s with candidates := s.candidates.map (fun c =>
      if c.id == id then { c with evaluated := true } else c) }
  else none

def review (contracts : Contracts) (s : State) (id : Id .candidate) : Option State := do
  let candidate ← s.candidates.find? (·.id == id)
  if candidate.evaluated && contracts.reviews.contains candidate.toCandidateMaterial then
    some { s with candidates := s.candidates.map (fun c =>
      if c.id == id then { c with reviewed := true } else c) }
  else none

def learningApprovalValid (s : State) (candidate : Candidate) (approval : LearningApproval) : Bool :=
  approval.material == candidate.toCandidateMaterial && approval.policy == s.authority.policy &&
  approval.validFrom ≤ s.now && s.now < approval.expires

def learningAuthorized (contracts : Contracts) (s : State) (candidate : Candidate) : Bool :=
  contracts.learningApprovals.any (learningApprovalValid s candidate)

def prepareAdmission (contracts : Contracts) (s : State) (id : Id .candidate) : Option State := do
  let c ← s.candidates.find? (·.id == id)
  if s.alive && c.evaluated && c.reviewed && retained s (c.evidence ++ c.rubric :: c.trials) &&
      contracts.evaluations.contains c.toCandidateMaterial &&
      contracts.reviews.contains c.toCandidateMaterial && learningAuthorized contracts s c &&
      contracts.corpusBindings.contains (c.plan.candidate.snapshot, c.corpusPin) &&
      c.corpusPin.realm == s.realm && c.corpusPin.family == .corpus &&
      s.retained.contains c.corpusPin.envelope &&
      contracts.approvals.any (·.plan == c.plan) && portfolioPlanValid s.knowledge c.plan &&
      c.plan.expectedSnapshot == s.knowledge.kb.snapshot &&
      (match c.procedure with
       | none => true
       | some p => contracts.packages.contains p && retained s p.closure &&
           contracts.packageReleases.contains (c.plan.candidate.snapshot, p) &&
           validateProcedure s.catalog contracts.predicates p &&
           !s.authority.withdrawn.contains p.revision &&
           subset s.authority.floor p.floor) then
    some { s with admission := some ⟨c, s.knowledge.kb.snapshot,
      s.authority.policy, s.authority.correctionGeneration⟩ }
  else none

/-- Existing signed portfolio admission is the only knowledge mutation. The
selected new state is obtained from the old applyPortfolio, not invented here. -/
def publishAdmission (contracts : Contracts) (s : State) : Option State := do
  let a ← s.admission
  if s.alive && a.expected == s.knowledge.kb.snapshot &&
      a.policy == s.authority.policy && a.correctionGeneration == s.authority.correctionGeneration &&
      s.candidates.contains a.candidate && a.candidate.evaluated && a.candidate.reviewed &&
      contracts.evaluations.contains a.candidate.toCandidateMaterial &&
      contracts.reviews.contains a.candidate.toCandidateMaterial &&
      learningAuthorized contracts s a.candidate &&
      portfolioPlanValid s.knowledge a.candidate.plan &&
      retained s (a.candidate.evidence ++ a.candidate.rubric :: a.candidate.trials) &&
      contracts.corpusBindings.contains (a.candidate.plan.candidate.snapshot, a.candidate.corpusPin) &&
      a.candidate.corpusPin.realm == s.realm && a.candidate.corpusPin.family == .corpus &&
      s.retained.contains a.candidate.corpusPin.envelope &&
      (match a.candidate.procedure with
       | none => true
       | some p => contracts.packages.contains p && retained s p.closure &&
           contracts.packageReleases.contains (a.candidate.plan.candidate.snapshot, p) &&
           validateProcedure s.catalog contracts.predicates p &&
           !s.authority.withdrawn.contains p.revision &&
           subset s.authority.floor p.floor) then
    let result := applyPortfolio s.knowledge s.knowledgeJournal a.candidate.plan contracts.approvals
    if result.outcome == .applied then
      some { s with
        knowledge := result.state
        knowledgeJournal := result.journal
        catalogRelease := result.state.kb.snapshot
        corpusPin := a.candidate.corpusPin
        admission := none }
    else none
  else none

def acknowledgeAdmission (s : State) (id : Id .candidate) : Option State := do
  let c ← s.candidates.find? (·.id == id)
  if s.knowledgeJournal.any (·.plan == c.plan) then
    some { s with acknowledged := id :: s.acknowledged }
  else none

def retire (s : State) (run : ActivationId) : Option State := do
  let r ← findRun s run
  if (r.status == .cancelled || r.status == .completed) &&
      !s.ownership.any (·.holder == run) &&
      !s.runs.any (fun child => child.parent == some r.invocation && child.status == .active) then
    some (putRun s { r with status := .retired })
  else none

end Kctx.Engineering
