import Lean
import Kctx.Citation

set_option autoImplicit false
namespace Kctx.Citation.Reference
open Lean ResolutionExamples

structure Case where
  id : String
  obligation : String
  polarity : String
  passed : Bool

def resolutionCases : List Case :=
  [⟨"exact_resolution", "CIT01", "positive", exactResolution⟩,
   ⟨"empty_range", "CIT01", "positive", emptyRange⟩,
   ⟨"start_boundary", "CIT01", "positive", startBoundary⟩,
   ⟨"reversed_bounds", "CIT01", "negative", reversedBounds⟩,
   ⟨"end_beyond_length", "CIT01", "negative", endBeyondLength⟩,
   ⟨"machine_overflow_equivalent", "CIT01", "negative", machineOverflowEquivalent⟩,
   ⟨"mixed_units", "CIT01", "negative", mixedUnits⟩,
   ⟨"unsupported_profile", "CIT01", "negative", unsupportedProfile⟩,
   ⟨"wrong_realm", "CIT01", "negative", wrongRealm⟩,
   ⟨"wrong_source", "CIT01", "negative", wrongSource⟩,
   ⟨"wrong_path", "CIT01", "negative", wrongPath⟩,
   ⟨"wrong_revision", "CIT01", "negative", wrongRevision⟩,
   ⟨"wrong_content", "CIT01", "negative", wrongContent⟩,
   ⟨"expected_mismatch", "CIT01", "negative", expectedMismatch⟩,
   ⟨"missing_object", "CIT01", "negative", missingObject⟩,
   ⟨"denied_read", "CIT01", "negative", deniedRead⟩,
   ⟨"denied_before_disclosure", "CIT01", "negative", deniedBeforeDisclosure⟩,
   ⟨"rejected_rebinding", "CIT03", "negative", rejectedRebinding⟩,
   ⟨"successful_append", "CIT03", "positive", successfulAppend⟩,
   ⟨"copied_line_ambiguous", "CIT04", "negative", copiedLineAmbiguous⟩,
   ⟨"singleton_text_unknown", "CIT04", "negative", singletonTextUnknown⟩,
   ⟨"truncated_text_unknown", "CIT04", "negative", truncatedTextUnknown⟩,
   ⟨"zero_search_budget", "CIT04", "negative", zeroBudgetUnknown⟩,
   ⟨"identity_witness", "CIT04", "positive", identityWitnessSuccess⟩,
   ⟨"same_binding", "CIT04", "positive", sameBindingSuccess⟩,
   ⟨"forged_position", "CIT04", "negative", forgedPositionRefused⟩,
   ⟨"foreign_comparison_context", "CIT04", "negative", foreignContextRefused⟩,
   ⟨"target_denied", "CIT04", "negative", targetDeniedRefused⟩,
   ⟨"original_must_resolve", "CIT04", "negative", originalMustResolve⟩,
   ⟨"invalid_original_preserved", "CIT04", "negative", invalidOriginalPreserved⟩,
   ⟨"exhaustive_no_match_unknown", "CIT04", "negative", exhaustiveNoMatchUnknown⟩,
   ⟨"missing_target_unknown", "CIT04", "negative", missingTargetUnknown⟩,
   ⟨"target_mixed_units", "CIT04", "negative", targetMixedUnitsRefused⟩,
   ⟨"target_unsupported_profile", "CIT04", "negative", targetProfileRefused⟩,
   ⟨"foreign_comparison_realm", "CIT04", "negative", foreignRealmRefused⟩,
   ⟨"foreign_comparison_source", "CIT04", "negative", foreignSourceRefused⟩]

def handlePositive : List String :=
  ["successful_issue_expand", "inclusive_expiry_boundary", "inclusive_issue_boundary",
   "inherited_mapping_preserved", "paginated_contrary_evidence", "empty_complete_page"]

def cases : List Case := resolutionCases ++
  (HandleExamples.allCases.map (fun (name, passed) =>
    ⟨"alias_" ++ name, "CIT02", if handlePositive.contains name then "positive" else "negative", passed⟩)) ++
  (LegacyExamples.allCases.map (fun (name, obligation, polarity, passed) =>
    ⟨"legacy_" ++ name, obligation, polarity, passed⟩))

def Case.json (row : Case) : Json := Json.mkObj
  [("id", .str row.id), ("obligation", .str row.obligation),
   ("polarity", .str row.polarity), ("passed", .bool row.passed),
   ("scope", .str "executed finite citation calculus case; no production or full-gate qualification")]

end Kctx.Citation.Reference

def main : IO UInt32 := do
  let output ← IO.getStdout
  let results := Kctx.Citation.Reference.cases
  for row in results do
    output.putStrLn row.json.compress
  return if results.all (·.passed) then 0 else 1
