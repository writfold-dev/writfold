import Kctx.Citation.Correspondence
import Kctx.Citation.Handles
import Kctx.Citation.Presentation
import Kctx.Engineering.Conservation

set_option autoImplicit false
namespace Kctx.Citation

/-- The citation layer cannot write legacy state or its operation journal. -/
structure State where
  engineering : Engineering.State
  citations : Data
  deriving DecidableEq, Repr, Inhabited

inductive LocalCommand where
  | retain (representation : Representation)
  | openAliases (scope : AliasScope)
  | issueAlias (scope : AliasScope) (handle : Handle) (binding : Binding)
      (issuedAt expiresAt : Nat)
  | retireAliases (scope : AliasScope)
  | acquisitionHint (reference : RepresentationRef) (checkedAt : Nat)
  deriving DecidableEq, Repr, Inhabited

def tableAt (tables : List AliasTable) (scope : AliasScope) : Option AliasTable :=
  tables.find? (fun table => decide (table.scope = scope))

/-- These hints schedule acquisition only. They are never Observation.observedAt. -/
def localUpdate (access : Access) (data : Data) : LocalCommand → Option Data
  | .retain representation =>
      (retain data.representations representation).map
        (fun retained => { data with representations := retained })
  | .openAliases scope =>
      if (tableAt data.aliases scope).isSome then none
      else some { data with aliases := data.aliases ++ [⟨scope, []⟩] }
  | .issueAlias scope handle binding issuedAt expiresAt => do
      let table ← tableAt data.aliases scope
      let next ← issueAlias table handle binding issuedAt expiresAt access
      let aliases := data.aliases.map (fun old => if old.scope = scope then next else old)
      some { data with aliases := aliases }
  | .retireAliases scope =>
      some { data with aliases := data.aliases.filter (fun table => decide (table.scope ≠ scope)) }
  | .acquisitionHint reference checkedAt =>
      some { data with acquisitionHints :=
        data.acquisitionHints.filter (fun hint => decide (hint.1 ≠ reference)) ++ [(reference, checkedAt)] }

def localStep (access : Access) (state : State) (command : LocalCommand) : Option State :=
  (localUpdate access state.citations command).map (fun data => { state with citations := data })

inductive Query where
  | resolve (binding : Binding)
  | compare (original target : Binding) (witness : Option CorrespondenceWitness) (budget : Nat)
  | expand (tableScope currentScope : AliasScope) (handle : Handle) (now : Nat)
  | present (query : NoticeQuery) (cursor : Option NoticeCursor)
  | assess (context : Context) (claim : Ref)
  deriving DecidableEq, Repr, Inhabited

inductive Response where
  | resolution (result : ResolveResult)
  | comparison (result : CompareResult)
  | alias (result : AliasResult)
  | presentation (result : NoticePage)
  | presentationError (error : PresentationError)
  | assessment (result : Assessment)
  deriving DecidableEq, Repr, Inhabited

/-- A query returns data, never a new execution state or a knowledge witness.
currentScope and access come from the authenticated host, not the alias token. -/
def inspect (state : State) (access : Access) : Query → Response
  | .resolve binding => .resolution (resolve state.citations.representations access binding)
  | .compare original target witness budget =>
      .comparison (compare state.citations.representations access original target witness budget)
  | .expand scope current handle now =>
      .alias (match tableAt state.citations.aliases scope with
        | none => .expired
        | some table => expandAlias table current handle now access)
  | .present query cursor => match presentNotices query cursor with
      | .ok page => .presentation page
      | .error error => .presentationError error
  | .assess context claim => .assessment (assess state.engineering.knowledge.kb context claim)

/-- Closed transitions include the actual pure evaluator and the unchanged
engineering transition relation. No arbitrary replacement legacy state exists. -/
inductive Transition (contracts : Engineering.Contracts) (access : Access) : State → State → Prop where
  | local {before after : State} {command : LocalCommand} :
      localStep access before command = some after → Transition contracts access before after
  | inspection (state : State) (query : Query) (response : Response) :
      inspect state access query = response → Transition contracts access state state
  | engineering {state : State} {next : Engineering.State} :
      Engineering.Transition contracts state.engineering next →
      Transition contracts access state { state with engineering := next }
  | refused (state : State) : Transition contracts access state state

inductive Trace (contracts : Engineering.Contracts) (access : Access) (initial : State) : State → Prop where
  | initial : Trace contracts access initial initial
  | next {before after : State} : Trace contracts access initial before →
      Transition contracts access before after → Trace contracts access initial after

inductive CitationTrace (access : Access) (initial : State) : State → Prop where
  | initial : CitationTrace access initial initial
  | local {before after : State} {command : LocalCommand} :
      CitationTrace access initial before → localStep access before command = some after →
      CitationTrace access initial after
  | inspection {state : State} (query : Query) (response : Response) :
      CitationTrace access initial state → inspect state access query = response →
      CitationTrace access initial state

end Kctx.Citation
