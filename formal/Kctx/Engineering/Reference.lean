import Lean
import Kctx.Engineering.ExamplesRetry
import Kctx.Engineering.CalibrationExamples
import Kctx.Engineering.AuthorityExamples
import Kctx.Engineering.GrantAuthorizationExamples
import Kctx.Engineering.QuiescenceExamples

set_option autoImplicit false
namespace Kctx.Engineering.Reference
open Lean Examples

structure Case where
  id : String
  obligation : String
  polarity : String
  passed : Bool
  observed : Json

/-- Observations come from reducer results, not source strings or fabricated
execution claims. Historical records retain qualified inputs, not external truth. -/
def observe (context : Option State) (predicate : Bool) : Json :=
  Json.mkObj [("predicate_result", .bool predicate), ("trace_context", match context with
    | none => .null
    | some s => Json.mkObj
      [("heads", .str (reprStr (Ledger.orderedHeads s.ledger))),
       ("inventory_epoch", toJson s.ledger.inventoryEpoch),
       ("alive", .bool s.alive), ("incarnation", toJson s.incarnation),
       ("knowledge", .str (reprStr s.knowledge)),
       ("knowledge_journal", .str (reprStr s.knowledgeJournal)),
       ("corpus_pin", .str (reprStr s.corpusPin)),
       ("activity_pin", .str (reprStr s.activityPin)),
       ("catalog", .str (reprStr s.catalog)), ("runs", .str (reprStr s.runs)),
       ("authority", .str (reprStr s.authority)),
       ("budgets", .str (reprStr s.budgets)), ("effects", .str (reprStr s.effects)),
       ("claims", .str (reprStr s.claims)),
       ("ownership", .str (reprStr s.ownership)), ("retained", .str (reprStr s.retained)),
       ("captures", .str (reprStr s.captures)), ("returned", .str (reprStr s.returned)),
       ("receipts", .str (reprStr s.receipts)), ("reused", .str (reprStr s.reused)),
       ("world", .str (reprStr s.world)), ("observations", .str (reprStr s.observations)),
       ("acknowledged", .str (reprStr s.acknowledged)),
       ("cut_manifests", .str (reprStr s.ledger.cuts))])]

def cases : List Case :=
  ([ ("E01", E01_positive, E01_negative, promoted, initial),
     ("E02", E02_positive, E02_negative, twoRoots, started),
     ("E03", E03_positive, E03_negative, joined, called),
     ("E04", E04_positive, E04_negative, reused, started),
     ("E05", E05_positive, E05_negative, twoRoots, twoRoots),
     ("E06", E06_positive, E06_negative, released, issued),
     ("E07", E07_positive, E07_negative, lateReconciled, claimCrash),
     ("E08", E08_positive, E08_negative, promoted, admissionReady),
     ("E09", E09_positive, E09_negative, cutAfter, started),
     ("E10", E10_positive, E10_negative, nested, nested),
     ("E11", E11_positive, E11_negative,
       issued.bind (fun s => environment contracts s (.completeExternalEffect event 1 0)), issued),
     ("E12", E12_positive, E12_negative, migrated, recovered) ].flatMap
    (fun (id, positive, negative, positiveContext, negativeContext) =>
      [⟨id ++ "-positive", id, "positive", positive, observe positiveContext positive⟩,
       ⟨id ++ "-negative", id, "negative", negative, observe negativeContext negative⟩])) ++
  ([ ("E04-wrong-role", "E04", "negative", wrongRoleRefused, started),
     ("E03-learned-floor", "E03", "negative", learnedFloorRefused, learningProcedureReviewed procV2),
     ("E04-stale-reuse", "E04", "negative", staleReuseRefused, prepared),
     ("E04-dependency-reuse", "E04", "negative", changedDependencyReuseRefused, otherBindingReady binding),
     ("E04-old-child-return", "E04", "negative", oldChildReturnRefused, newActivationPending),
     ("E08-publication-retention", "E08", "negative", publicationRetentionRefused, admissionReady),
     ("E03-child-publish", "E03", "negative", childPublishRefused, broadCalled),
     ("E12-expired-grant", "E12", "negative", expiredGrantRefused, shortGrantClaimed),
     ("E12-withdrawn-procedure", "E12", "negative", withdrawnProcedureRefused, claimed),
     ("E12-correction-generation", "E12", "negative", correctedContinuationRefused, claimed),
     ("E12-post-init-grant", "E12", "positive", postInitGrantDispatch, installedGrantIssued),
     ("E12-unqualified-grant-installation", "E12", "negative", unqualifiedGrantInstallationRefused, grantReady),
     ("E12-foreign-grant-realm", "E12", "negative", foreignGrantRealmRefused, grantReady),
     ("E12-stale-grant-policy", "E12", "negative", staleGrantPolicyRefused, grantReady),
     ("E12-missing-grant-envelope", "E12", "negative", missingGrantEnvelopeRefused, grantReady),
     ("E12-invalid-grant-interval", "E12", "negative", invalidGrantIntervalRefused, grantReady),
     ("E12-revoked-grant-id", "E12", "negative", revokedGrantIdCannotBeReinstalled, installedGrantClaimed),
     ("E12-renewed-grant-stale-claim", "E12", "negative", renewedGrantCannotRefreshClaim, renewedGrantClaimed),
     ("E11-changed-provider-binding", "E11", "negative", changedProviderBindingRefused, effectInputsReady),
     ("E06-prepared-release", "E06", "negative", preparedReleaseRefused, preparedOwner),
     ("E07-reply-loss-unknown", "E07", "positive", replyLossUnknown, replyLost),
     ("E07-reply-reconciled", "E07", "positive", replyLossReconciliation, replyReconciled),
     ("E06-stale-process-generation", "E06", "negative", staleQuiescenceRefused, settledTree),
     ("E07-no-effect-prepared", "E07", "negative", noEffectPrepared, prepared),
     ("E07-no-effect-claimed", "E07", "negative", noEffectClaimed, claimed),
     ("E08-exact-material", "E08", "negative", exactMaterialRefused, initial),
     ("E08-competing-promotion", "E08", "negative", competingPromotion, admissionReady),
     ("E08-lost-publication-ack", "E08", "positive", lostPublicationAcknowledgement, published),
     ("E10-pinned-revisions", "E10", "positive", revisionsPinned, started),
     ("E10-alias-drift", "E10", "negative", aliasDriftRefused, called),
     ("E10-recursion", "E10", "negative", recursionRefused, initial),
     ("E02-qualified-capture", "E02", "positive", captureIntroducesObject, captured),
     ("E02-duplicate-capture", "E02", "negative", duplicateCaptureRefused, captured),
     ("E02-phantom-object", "E02", "negative", phantomObjectRefused, initial),
     ("E02-unqualified-capture", "E02", "negative", unqualifiedCaptureRefused, initial),
     ("E03-failed-join", "E03", "negative", failedJoinRefused, unsuccessfulReturn .failed),
     ("E03-cancelled-join", "E03", "negative", cancelledJoinRefused, unsuccessfulReturn .cancelled),
     ("E03-unknown-join", "E03", "negative", unknownJoinRefused, unsuccessfulReturn .unknownEffect),
     ("E12-revoked-dispatch", "E12", "negative", revokedDispatchRefused, claimed),
     ("E07-retry-unclaimed", "E07", "positive", retryWithoutOldClaim, retriedUnclaimed),
     ("E07-retry-claimed-before-io", "E07", "positive", retryAfterClaimBeforeIO, retriedClaimed),
     ("E07-retry-fresh-claim", "E07", "negative", retryNeedsFreshClaim, retriedClaimed),
     ("E07-retry-idempotent", "E07", "positive", idempotentRetryOneApplication, duplicateApplied),
     ("E07-retry-outside-change", "E07", "negative", idempotentReplyPreservesOutsideChange, duplicateAfterOutsideChange),
     ("E07-retry-proven-absence", "E07", "positive", provenAbsenceRetryApplies, appliedAfterAbsence),
     ("E07-retry-old-rejection", "E07", "negative", oldRejectionCannotClassifyRetry, retriedAbsence),
     ("E10-resume-context-reload", "E10", "positive", resumeRequiresContextReload, contextReloaded),
     ("E12-resume-authority", "E12", "negative", unqualifiedResumeRefused, claimCrash),
     ("E06-same-activation-conflict", "E06", "negative", sameActivationConflictRefused, conflictPrepared),
     ("E01-context-isolation", "E01", "positive", legacyContextIsolation, initial) ].map
    (fun (id, obligation, polarity, passed, context) =>
      ⟨id, obligation, polarity, passed, observe context passed⟩))

def Case.json (c : Case) : Json := Json.mkObj
  [("id", .str c.id), ("obligation", .str c.obligation), ("polarity", .str c.polarity),
   ("passed", .bool c.passed), ("observed", c.observed),
   ("scope", .str "finite calculus witness under explicit qualified inputs; not Rust/protocol or WF31 qualification")]

end Kctx.Engineering.Reference

def main : IO UInt32 := do
  let output ← IO.getStdout
  let results := Kctx.Engineering.Reference.cases
  for c in results do
    output.putStrLn c.json.compress
  return if results.all (·.passed) then 0 else 1
