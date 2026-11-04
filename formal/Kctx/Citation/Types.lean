import Kctx.Engineering.Identity

set_option autoImplicit false
namespace Kctx.Citation

/-! Mathematical citation inputs. Historical Kctx identifiers and signed semantics
are unchanged. Exact byte preimages are modeled rather than assuming hashes are
injective; production digest/capture/authorization refinement is separate. -/

inductive NameKind where
  | session | handle
  deriving DecidableEq, Repr, Inhabited

structure Name (kind : NameKind) where
  value : Nat
  deriving DecidableEq, Repr, Inhabited

abbrev SessionId := Name .session
abbrev Handle := Name .handle
abbrev RealmId := Engineering.Id .realm

inductive RepresentationProfile where
  | gitBlob | unsupportedNativeUtf16
  deriving DecidableEq, Repr, Inhabited

inductive CoordinateUnit where
  | byte | utf16
  deriving DecidableEq, Repr, Inhabited

/-- Repository and path provenance are distinct from representation identity. -/
structure RepresentationRef where
  realm : RealmId
  repository : SourceId
  revision : SourceRevision
  path : String
  content : ContentId
  profile : RepresentationProfile
  deriving DecidableEq, Repr, Inhabited

/-- Half-open mathematical offsets. Validation is performed by resolve. -/
structure Selector where
  first : Nat
  last : Nat
  unit : CoordinateUnit
  deriving DecidableEq, Repr, Inhabited

structure Binding where
  id : CitationId
  release : SnapshotId
  context : ContextId
  representation : RepresentationRef
  selector : Selector
  expected : List UInt8
  deriving DecidableEq, Repr, Inhabited

structure Representation where
  reference : RepresentationRef
  bytes : List UInt8
  deriving DecidableEq, Repr, Inhabited

abbrev Store := List Representation

/-- Supplied current policy decision for this exact binding. A model assumption,
not a capability supplied by untrusted content or a proof of real credentials. -/
abbrev Access := Binding → Bool

inductive InvalidReason where
  | unsupportedProfile | mixedUnits | invalidBounds | selectionMismatch
  deriving DecidableEq, Repr, Inhabited

inductive ResolveResult where
  | exact (bytes : List UInt8) (provenance : RepresentationRef)
  | invalid (reason : InvalidReason)
  | unavailable
  | denied
  deriving DecidableEq, Repr, Inhabited

/-- Unavailable enumeration is not an exhaustive empty search. Presentation
retains this distinction independently of its own budget and omitted notices. -/
inductive Coverage where
  | unavailable
  | measured (examined total : Nat)
  deriving DecidableEq, Repr, Inhabited

def Coverage.examined : Coverage → Nat
  | .unavailable => 0
  | .measured examined _ => examined

def Coverage.total : Coverage → Option Nat
  | .unavailable => none
  | .measured _ total => some total

def Coverage.complete : Coverage → Bool
  | .unavailable => false
  | .measured examined total => examined == total

/-- The initial profile only transports the same immutable occurrence. A text
match is not a constructor. New transformation profiles need separate proof. -/
inductive CorrespondenceWitness where
  | identicalPosition
  deriving DecidableEq, Repr, Inhabited

inductive UnknownReason where
  | originalUnavailable | targetUnavailable | unsupportedCorrespondence
  | incompleteSearch | noTextMatch | scopeMismatch
  | originalInvalid (reason : InvalidReason)
  deriving DecidableEq, Repr, Inhabited

inductive CompareResult where
  | sameBinding
  | uniqueJustified (successor : Binding) (witness : CorrespondenceWitness)
  | multiple (candidates : List Selector) (coverage : Coverage)
  | unknown (reason : UnknownReason) (candidates : List Selector) (coverage : Coverage)
  | denied
  deriving DecidableEq, Repr, Inhabited

structure AliasScope where
  realm : RealmId
  principal : ActorId
  session : SessionId
  release : SnapshotId
  context : ContextId
  generation : Nat
  deriving DecidableEq, Repr, Inhabited

structure AliasEntry where
  handle : Handle
  binding : Binding
  issuedAt : Nat
  expiresAt : Nat
  deriving DecidableEq, Repr, Inhabited

structure AliasTable where
  scope : AliasScope
  entries : List AliasEntry
  deriving DecidableEq, Repr, Inhabited

inductive AliasResult where
  | durable (binding : Binding)
  | expired | foreign | unavailable | denied
  deriving DecidableEq, Repr, Inhabited

inductive NoticeKind where
  | evidence | contrary | stale | ambiguous | omission
  deriving DecidableEq, Repr, Inhabited

structure Notice where
  kind : NoticeKind
  text : String
  deriving DecidableEq, Repr, Inhabited

/-- Citation metadata is not part of KB, Context, Witness or validation receipts. -/
structure Data where
  representations : Store
  aliases : List AliasTable
  acquisitionHints : List (RepresentationRef × Nat)
  deriving DecidableEq, Repr, Inhabited

end Kctx.Citation
