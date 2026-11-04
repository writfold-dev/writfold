import Kctx.Citation.CorrespondenceProofs

set_option autoImplicit false
namespace Kctx.Citation.ResolutionExamples

/-- UTF-8 fixture: 6c 69 6e 65 0a; copied fixture repeats those exact five bytes. -/
def lineBytes : List UInt8 := [108, 105, 110, 101, 10]
def copiedBytes : List UInt8 := lineBytes ++ lineBytes

def originalRef : RepresentationRef :=
  ⟨⟨1⟩, ⟨"repo", by decide⟩, ⟨"commit-a", by decide⟩, "file.txt",
    ⟨"blob-a", by decide⟩, .gitBlob⟩
def copiedRef : RepresentationRef :=
  { originalRef with revision := ⟨"commit-b", by decide⟩, content := ⟨"blob-b", by decide⟩ }
def originalObject : Representation := ⟨originalRef, lineBytes⟩
def copiedObject : Representation := ⟨copiedRef, copiedBytes⟩
def original : Binding :=
  ⟨⟨"citation-a", by decide⟩, ⟨"release-a", by decide⟩, ⟨"context", by decide⟩,
    originalRef, ⟨0, 5, .byte⟩, lineBytes⟩
def later : Binding :=
  { original with id := ⟨"citation-b", by decide⟩, release := ⟨"release-b", by decide⟩ }
def copied : Binding := { later with representation := copiedRef }
def store : Store := [originalObject, copiedObject]
def allow : Access := fun _ => true
def deny : Access := fun _ => false

def exactResolution : Bool :=
  resolve store allow original == .exact lineBytes originalRef

def emptyRange : Bool :=
  resolve store allow { original with selector := ⟨5, 5, .byte⟩, expected := [] } ==
    .exact [] originalRef

def startBoundary : Bool :=
  resolve store allow { original with selector := ⟨0, 0, .byte⟩, expected := [] } ==
    .exact [] originalRef

def reversedBounds : Bool :=
  resolve store allow { original with selector := ⟨5, 4, .byte⟩ } == .invalid .invalidBounds

def endBeyondLength : Bool :=
  resolve store allow { original with selector := ⟨0, 6, .byte⟩ } == .invalid .invalidBounds

def machineOverflowEquivalent : Bool :=
  resolve store allow { original with selector := ⟨0, 4294967296, .byte⟩ } ==
    .invalid .invalidBounds

def mixedUnits : Bool :=
  resolve store allow { original with selector := ⟨0, 5, .utf16⟩ } == .invalid .mixedUnits

def unsupportedProfile : Bool :=
  resolve store allow { original with representation :=
    { originalRef with profile := .unsupportedNativeUtf16 } } == .invalid .unsupportedProfile

def wrongRealm : Bool :=
  resolve store allow { original with representation :=
    { originalRef with realm := ⟨2⟩ } } == .unavailable

def wrongSource : Bool :=
  resolve store allow { original with representation :=
    { originalRef with repository := ⟨"another-repo", by decide⟩ } } == .unavailable

def wrongPath : Bool :=
  resolve store allow { original with representation :=
    { originalRef with path := "other.txt" } } == .unavailable

def wrongRevision : Bool :=
  resolve store allow { original with representation :=
    { originalRef with revision := ⟨"HEAD", by decide⟩ } } == .unavailable

def wrongContent : Bool :=
  resolve store allow { original with representation :=
    { originalRef with content := ⟨"another-blob", by decide⟩ } } == .unavailable

def expectedMismatch : Bool :=
  resolve store allow { original with expected := [76, 73, 78, 69, 10] } ==
    .invalid .selectionMismatch

def missingObject : Bool := resolve [] allow original == .unavailable

def deniedRead : Bool := resolve store deny original == .denied

def deniedBeforeDisclosure : Bool :=
  resolve [] deny { original with selector := ⟨5, 0, .utf16⟩ } == .denied

def rejectedRebinding : Bool :=
  retain store { originalObject with bytes := copiedBytes } == none

def successfulAppend : Bool :=
  retain [originalObject] copiedObject == some store

def copiedLineAmbiguous : Bool :=
  compare store allow original copied none 20 ==
    .multiple [⟨0, 5, .byte⟩, ⟨5, 10, .byte⟩] (.measured 6 6)

def singletonTextUnknown : Bool :=
  compare store allow original later none 20 ==
    .unknown .unsupportedCorrespondence [⟨0, 5, .byte⟩] (.measured 1 1)

def truncatedTextUnknown : Bool :=
  compare store allow original copied none 1 ==
    .unknown .incompleteSearch [⟨0, 5, .byte⟩] (.measured 1 6)

def zeroBudgetUnknown : Bool :=
  compare store allow original copied none 0 == .unknown .incompleteSearch [] (.measured 0 6)

def identityWitnessSuccess : Bool :=
  compare store allow original later (some .identicalPosition) 0 ==
    .uniqueJustified later .identicalPosition

def sameBindingSuccess : Bool := compare store allow original original none 0 == .sameBinding

def forgedPositionRefused : Bool :=
  compare store allow original { later with selector := ⟨5, 5, .byte⟩, expected := [] }
    (some .identicalPosition) 20 ==
    .unknown .unsupportedCorrespondence [⟨0, 5, .byte⟩] (.measured 1 1)

def foreignContextRefused : Bool :=
  compare store allow original { later with context := ⟨"foreign", by decide⟩ }
    (some .identicalPosition) 20 == .unknown .scopeMismatch [] .unavailable

def targetDeniedRefused : Bool :=
  compare store (fun binding => decide (binding = original)) original later
    (some .identicalPosition) 20 == .denied

def originalMustResolve : Bool :=
  compare [copiedObject] allow original copied none 20 ==
    .unknown .originalUnavailable [] .unavailable

def absentRef : RepresentationRef :=
  { originalRef with revision := ⟨"commit-c", by decide⟩, content := ⟨"blob-c", by decide⟩ }
def absentObject : Representation := ⟨absentRef, [110, 111, 110, 101, 10]⟩
def absentTarget : Binding :=
  { later with representation := absentRef, expected := [110, 111, 110, 101, 10] }

def exhaustiveNoMatchUnknown : Bool :=
  compare (store ++ [absentObject]) allow original absentTarget none 20 ==
    .unknown .noTextMatch [] (.measured 1 1)

def missingTargetUnknown : Bool :=
  compare [originalObject] allow original copied none 20 ==
    .unknown .targetUnavailable [] .unavailable

def targetMixedUnitsRefused : Bool :=
  compare store allow original { later with selector := ⟨0, 5, .utf16⟩ } none 20 ==
    .unknown .unsupportedCorrespondence [] .unavailable

def targetProfileRefused : Bool :=
  compare store allow original { later with representation :=
    { originalRef with profile := .unsupportedNativeUtf16 } } none 20 ==
    .unknown .unsupportedCorrespondence [] .unavailable

def foreignRealmRefused : Bool :=
  compare store allow original { later with representation :=
    { originalRef with realm := ⟨2⟩ } } (some .identicalPosition) 20 ==
    .unknown .scopeMismatch [] .unavailable

def foreignSourceRefused : Bool :=
  compare store allow original { later with representation :=
    { originalRef with repository := ⟨"another-repo", by decide⟩ } }
    (some .identicalPosition) 20 == .unknown .scopeMismatch [] .unavailable

def invalidOriginalPreserved : Bool :=
  compare store allow { original with expected := [0] } later none 20 ==
    .unknown (.originalInvalid .selectionMismatch) [] .unavailable

theorem invalidOriginalPreserved_nonvacuous : invalidOriginalPreserved = true := by decide

theorem exactResolution_nonvacuous : exactResolution = true := by decide
theorem emptyRange_nonvacuous : emptyRange = true := by decide
theorem startBoundary_nonvacuous : startBoundary = true := by decide
theorem reversedBounds_nonvacuous : reversedBounds = true := by decide
theorem endBeyondLength_nonvacuous : endBeyondLength = true := by decide
theorem machineOverflowEquivalent_nonvacuous : machineOverflowEquivalent = true := by decide
theorem mixedUnits_nonvacuous : mixedUnits = true := by decide
theorem unsupportedProfile_nonvacuous : unsupportedProfile = true := by decide
theorem wrongRealm_nonvacuous : wrongRealm = true := by decide
theorem wrongSource_nonvacuous : wrongSource = true := by decide
theorem wrongPath_nonvacuous : wrongPath = true := by decide
theorem wrongRevision_nonvacuous : wrongRevision = true := by decide
theorem wrongContent_nonvacuous : wrongContent = true := by decide
theorem expectedMismatch_nonvacuous : expectedMismatch = true := by decide
theorem missingObject_nonvacuous : missingObject = true := by decide
theorem deniedRead_nonvacuous : deniedRead = true := by decide
theorem deniedBeforeDisclosure_nonvacuous : deniedBeforeDisclosure = true := by decide
theorem rejectedRebinding_nonvacuous : rejectedRebinding = true := by decide
theorem successfulAppend_nonvacuous : successfulAppend = true := by decide
theorem copiedLineAmbiguous_nonvacuous : copiedLineAmbiguous = true := by decide
theorem singletonTextUnknown_nonvacuous : singletonTextUnknown = true := by decide
theorem truncatedTextUnknown_nonvacuous : truncatedTextUnknown = true := by decide
theorem zeroBudgetUnknown_nonvacuous : zeroBudgetUnknown = true := by decide
theorem identityWitnessSuccess_nonvacuous : identityWitnessSuccess = true := by decide
theorem sameBindingSuccess_nonvacuous : sameBindingSuccess = true := by decide
theorem forgedPositionRefused_nonvacuous : forgedPositionRefused = true := by decide
theorem foreignContextRefused_nonvacuous : foreignContextRefused = true := by decide
theorem targetDeniedRefused_nonvacuous : targetDeniedRefused = true := by decide
theorem originalMustResolve_nonvacuous : originalMustResolve = true := by decide
theorem exhaustiveNoMatchUnknown_nonvacuous : exhaustiveNoMatchUnknown = true := by decide
theorem missingTargetUnknown_nonvacuous : missingTargetUnknown = true := by decide
theorem targetMixedUnitsRefused_nonvacuous : targetMixedUnitsRefused = true := by decide
theorem targetProfileRefused_nonvacuous : targetProfileRefused = true := by decide
theorem foreignRealmRefused_nonvacuous : foreignRealmRefused = true := by decide
theorem foreignSourceRefused_nonvacuous : foreignSourceRefused = true := by decide

/-- Arbitrary finite append histories, not just the fixture's one appended object. -/
theorem arbitrary_retention_preserves_fixture (next : Store)
    (history : RetainedEvolution [originalObject] next) :
    resolve next allow original = .exact lineBytes originalRef :=
  evolution_preserves_resolution history allow original lineBytes originalRef (by decide)

end Kctx.Citation.ResolutionExamples
