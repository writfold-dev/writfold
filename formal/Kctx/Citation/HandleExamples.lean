import Kctx.Citation.HandlesProofs
import Kctx.Citation.PresentationProofs

set_option autoImplicit false
set_option maxRecDepth 10000
set_option maxHeartbeats 0
namespace Kctx.Citation.HandleExamples

def scope : AliasScope :=
  ⟨⟨1⟩, ⟨"reader-a", by decide⟩, ⟨1⟩, ⟨"release-a", by decide⟩,
    ⟨"context-a", by decide⟩, 1⟩

def binding : Binding :=
  ⟨⟨"citation-a", by decide⟩, scope.release, scope.context,
    ⟨scope.realm, ⟨"repository-a", by decide⟩, ⟨"revision-a", by decide⟩,
      "source.txt", ⟨"content-a", by decide⟩, .gitBlob⟩,
    ⟨0, 1, .byte⟩, [65]⟩

def handle : Handle := ⟨1⟩
def allow : Access := fun _ => true
def deny : Access := fun _ => false
def empty : AliasTable := ⟨scope, []⟩
def entry : AliasEntry := ⟨handle, binding, 10, 20⟩
def retained : AliasTable := ⟨scope, [entry]⟩

def successfulIssueExpand : Bool :=
  match issueAlias empty handle binding 10 20 allow with
  | none => false
  | some table => decide (table = retained ∧
      expandAlias table scope handle 15 allow = .durable binding)

def duplicateHandleRefused : Bool :=
  decide (issueAlias retained handle binding 10 30 allow = none)

def issueWrongRealmRefused : Bool :=
  decide (issueAlias empty handle
    { binding with representation := { binding.representation with realm := ⟨2⟩ } }
    10 20 allow = none)

def issueWrongContextRefused : Bool :=
  decide (issueAlias empty handle { binding with context := ⟨"context-b", by decide⟩ }
    10 20 allow = none)

def issueWrongReleaseRefused : Bool :=
  decide (issueAlias empty handle { binding with release := ⟨"release-b", by decide⟩ }
    10 20 allow = none)

def issueRevokedRefused : Bool := decide (issueAlias empty handle binding 10 20 deny = none)
def issueReversedLifetimeRefused : Bool :=
  decide (issueAlias empty handle binding 20 10 allow = none)

def generationCollision : Bool :=
  decide (expandAlias retained { scope with generation := 2 } handle 15 allow = .expired)
def sessionCollision : Bool :=
  decide (expandAlias retained { scope with session := ⟨2⟩ } handle 15 allow = .foreign)
def realmCollision : Bool :=
  decide (expandAlias retained { scope with realm := ⟨2⟩ } handle 15 allow = .foreign)
def contextCollision : Bool :=
  decide (expandAlias retained { scope with context := ⟨"context-b", by decide⟩ }
    handle 15 allow = .foreign)
def principalCollision : Bool :=
  decide (expandAlias retained { scope with principal := ⟨"reader-b", by decide⟩ }
    handle 15 allow = .foreign)
def releaseCollision : Bool :=
  decide (expandAlias retained { scope with release := ⟨"release-b", by decide⟩ }
    handle 15 allow = .foreign)
def revokedPermission : Bool :=
  decide (expandAlias retained scope handle 15 deny = .denied)
def inclusiveExpiryBoundary : Bool :=
  decide (expandAlias retained scope handle 20 allow = .durable binding)
def afterExpiryBoundary : Bool :=
  decide (expandAlias retained scope handle 21 allow = .expired)
def beforeIssue : Bool :=
  decide (expandAlias retained scope handle 9 allow = .expired)
def inclusiveIssueBoundary : Bool :=
  decide (expandAlias retained scope handle 10 allow = .durable binding)
def missingHandle : Bool :=
  decide (expandAlias retained scope ⟨2⟩ 15 allow = .unavailable)
def foreignBeforeDisclosure : Bool :=
  decide (expandAlias empty { scope with session := ⟨2⟩ } handle 15 allow = .foreign)
def forgedEntryScope : Bool :=
  decide (expandAlias { retained with entries :=
    [{ entry with binding := { binding with context := ⟨"context-b", by decide⟩ } }] }
    scope handle 15 allow = .foreign)
def forgedEntryLifetime : Bool :=
  decide (expandAlias { retained with entries := [{ entry with issuedAt := 30 }] }
    scope handle 15 allow = .expired)
def inheritedMappingPreserved : Bool :=
  match issueAlias retained ⟨2⟩ { binding with id := ⟨"citation-b", by decide⟩ } 10 30 allow with
  | none => false
  | some table => decide (expandAlias table scope handle 15 allow = .durable binding ∧
      lookupAlias table.entries handle = some entry ∧ table.entries.length = 2)

def evidence : Notice := ⟨.evidence, "supporting evidence"⟩
def stale : Notice := ⟨.stale, "stale representation"⟩
def contrary : Notice := ⟨.contrary, "contrary evidence"⟩
def ambiguity : Notice := ⟨.ambiguous, "multiple candidates"⟩
def omission : Notice := ⟨.omission, "unavailable source"⟩
def query : NoticeQuery :=
  ⟨scope, ⟨"query-a", by decide⟩, ⟨"ordering-a", by decide⟩,
    2, [evidence, stale, contrary, ambiguity, omission], .measured 5 5⟩
def cursor : NoticeCursor := ⟨query, 2⟩

def paginatedContraryEvidence : Bool :=
  match presentNotices query none with
  | .error _ => false
  | .ok first =>
      match presentNotices query first.next with
      | .error _ => false
      | .ok second => decide (first.notices = [evidence, stale] ∧
          second.notices = [contrary, ambiguity] ∧ second.offset = 2 ∧
          second.omittedCount = 3 ∧ second.presentationComplete = false ∧
          second.coverage = query.coverage)

def finalPageStillPartial : Bool :=
  match presentNotices query (some ⟨query, 4⟩) with
  | .error _ => false
  | .ok page => decide (page.notices = [omission] ∧ page.next = none ∧
      page.omittedCount = 4 ∧ page.presentationComplete = false)

def missingNoticesPartial : Bool :=
  match presentNotices { query with budget := 1 } none with
  | .error _ => false
  | .ok page => decide (page.notices = [evidence] ∧ page.omittedCount = 4 ∧
      page.presentationComplete = false ∧ page.coverage.complete = true)

def partialSearchCompletePage : Bool :=
  match presentNotices { query with budget := 5, coverage := .measured 1 10 } none with
  | .error _ => false
  | .ok page => decide (page.notices = query.notices ∧ page.omittedCount = 0 ∧
      page.presentationComplete = true ∧ page.coverage.complete = false)

def failsWith (expected : PresentationError) : Except PresentationError NoticePage → Bool
  | .error actual => decide (actual = expected)
  | .ok _ => false

def badCursorScope : Bool := failsWith .cursorMismatch
  (presentNotices query (some { cursor with query :=
    { query with scope := { scope with session := ⟨2⟩ } } }))
def badCursorQuery : Bool := failsWith .cursorMismatch
  (presentNotices query (some { cursor with query :=
    { query with identity := ⟨"query-b", by decide⟩ } }))
def badCursorProfile : Bool := failsWith .cursorMismatch
  (presentNotices query (some { cursor with query :=
    { query with orderingProfile := ⟨"ordering-b", by decide⟩ } }))
def badCursorBudget : Bool := failsWith .cursorMismatch
  (presentNotices query (some { cursor with query :=
    { query with budget := 3 } }))
def badCursorSource : Bool := failsWith .cursorMismatch
  (presentNotices { query with notices := [evidence] } (some cursor))
def badCursorCoverage : Bool := failsWith .cursorMismatch
  (presentNotices { query with coverage := .measured 1 10 } (some cursor))
def zeroBudget : Bool := failsWith .zeroBudget
  (presentNotices { query with budget := 0 } none)
def badCursorOffset : Bool := failsWith .invalidOffset
  (presentNotices query (some ⟨query, 6⟩))
def emptyCompletePage : Bool :=
  match presentNotices { query with notices := [], coverage := .measured 0 0 } none with
  | .error _ => false
  | .ok page => decide (page.presentationComplete = true ∧ page.omittedCount = 0 ∧
      page.notices = [] ∧ page.next = none)

/-- Stable executable case names for the combined reference registry. -/
def allCases : List (String × Bool) :=
  [("successful_issue_expand", successfulIssueExpand),
   ("duplicate_handle_refused", duplicateHandleRefused),
   ("issue_wrong_realm_refused", issueWrongRealmRefused),
   ("issue_wrong_context_refused", issueWrongContextRefused),
   ("issue_wrong_release_refused", issueWrongReleaseRefused),
   ("issue_revoked_refused", issueRevokedRefused),
   ("issue_reversed_lifetime_refused", issueReversedLifetimeRefused),
   ("generation_collision", generationCollision),
   ("session_collision", sessionCollision),
   ("realm_collision", realmCollision),
   ("context_collision", contextCollision),
   ("principal_collision", principalCollision),
   ("release_collision", releaseCollision),
   ("revoked_permission", revokedPermission),
   ("inclusive_expiry_boundary", inclusiveExpiryBoundary),
   ("after_expiry_boundary", afterExpiryBoundary),
   ("before_issue", beforeIssue),
   ("inclusive_issue_boundary", inclusiveIssueBoundary),
   ("missing_handle", missingHandle),
   ("foreign_before_disclosure", foreignBeforeDisclosure),
   ("forged_entry_scope", forgedEntryScope),
   ("forged_entry_lifetime", forgedEntryLifetime),
   ("inherited_mapping_preserved", inheritedMappingPreserved),
   ("paginated_contrary_evidence", paginatedContraryEvidence),
   ("final_page_still_partial", finalPageStillPartial),
   ("missing_notices_partial", missingNoticesPartial),
   ("partial_search_complete_page", partialSearchCompletePage),
   ("bad_cursor_scope", badCursorScope),
   ("bad_cursor_query", badCursorQuery),
   ("bad_cursor_profile", badCursorProfile),
   ("bad_cursor_budget", badCursorBudget),
   ("bad_cursor_source", badCursorSource),
   ("bad_cursor_coverage", badCursorCoverage),
   ("zero_budget", zeroBudget),
   ("bad_cursor_offset", badCursorOffset),
   ("empty_complete_page", emptyCompletePage)]

theorem allCases_nonvacuous : allCases.all Prod.snd = true := by decide
theorem successfulIssueExpand_nonvacuous : successfulIssueExpand = true := by decide
theorem inheritedMappingPreserved_nonvacuous : inheritedMappingPreserved = true := by decide
theorem paginatedContraryEvidence_nonvacuous : paginatedContraryEvidence = true := by decide
theorem partialSearchCompletePage_nonvacuous : partialSearchCompletePage = true := by decide

end Kctx.Citation.HandleExamples
