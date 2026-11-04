import Init

/- Standalone candidate model. The ranker is arbitrary, including hostile outputs.
   A binding identifies an immutable case revision and excerpt inside a named cut.
   The trusted host supplies the checked predicates; theorems do not prove them. -/
namespace Writfold.CaseDiscovery

structure RealmId where
  value : Nat
  deriving DecidableEq

structure ReleaseId where
  value : Nat
  deriving DecidableEq

structure ContextId where
  value : Nat
  deriving DecidableEq

structure AssetId where
  value : Nat
  deriving DecidableEq

structure RevisionId where
  value : Nat
  deriving DecidableEq

structure RepresentationId where
  value : Nat
  deriving DecidableEq

structure SpanId where
  value : Nat
  deriving DecidableEq

structure RoleId where
  value : Nat
  deriving DecidableEq

structure Binding where
  realm : RealmId
  release : ReleaseId
  context : ContextId
  asset : AssetId
  revision : RevisionId
  representation : RepresentationId
  span : SpanId
  role : RoleId
  deriving DecidableEq

structure Scope where
  realm : RealmId
  release : ReleaseId
  context : ContextId

structure Policy where
  visible : Binding → Prop
  retained : Binding → Prop

def Eligible (s : Scope) (p : Policy) (b : Binding) : Prop :=
  b.realm = s.realm ∧ b.release = s.release ∧ b.context = s.context ∧
  p.visible b ∧ p.retained b

/- Deduplication, budgeting and ANN may drop any candidate. They never issue
   a handle until the engine checks membership and the scoped exact binding. -/
inductive Hit (s : Scope) (p : Policy) (ranked : List Binding) : Binding → Prop where
  | issue {b} : b ∈ ranked → Eligible s p b → Hit s p ranked b

theorem hit_is_eligible {s p ranked b} (h : Hit s p ranked b) : Eligible s p b := by
  cases h with
  | issue _ e => exact e

inductive Opened (s : Scope) (atFind now : Policy) (ranked : List Binding) : Binding → Prop where
  | peek {b} : Hit s atFind ranked b → Eligible s now b → Opened s atFind now ranked b

theorem revoked_cannot_open {s atFind now ranked b} (revoked : ¬ now.visible b) :
    ¬ Opened s atFind now ranked b := by
  intro h
  cases h with
  | peek _ e => exact revoked e.2.2.2.1

theorem another_binding_cannot_open_from_singleton {s p q a b} (different : b ≠ a) :
    ¬ Opened s p q [a] b := by
  intro h
  cases h with
  | peek hit _ =>
    cases hit with
    | issue mem _ => exact different (by simpa using mem)

/- Issuing candidate handles cannot mint an exact witness, change a corpus
   release or admit a proposed fix. Induct over arbitrary observation traces. -/
inductive Step (W : Type) : Type where
  | discover : List Binding → Step W
  | inspect : Binding → Step W
  | submit : W → Step W

def applyStep {S W : Type} (admit : S → W → S) (s : S) : Step W → S
  | .discover _ => s
  | .inspect _ => s
  | .submit w => admit s w

def run {S W : Type} (admit : S → W → S) : S → List (Step W) → S
  | s, [] => s
  | s, op :: rest => run admit (applyStep admit s op) rest

def witnesses {W : Type} : List (Step W) → List W
  | [] => []
  | .submit w :: xs => w :: witnesses xs
  | .discover _ :: xs => witnesses xs
  | .inspect _ :: xs => witnesses xs

theorem erase_discovery {S W : Type} (admit : S → W → S)
    (ops : List (Step W)) (s : S) :
    run admit s ops = (witnesses ops).foldl admit s := by
  induction ops generalizing s with
  | nil => rfl
  | cons op rest ih => cases op <;> simp only [run, applyStep, witnesses, List.foldl] <;> apply ih

theorem equal_witnesses_equal_authority {S W : Type} (admit : S → W → S)
    (a b : List (Step W)) (s : S) (h : witnesses a = witnesses b) :
    run admit s a = run admit s b := by
  rw [erase_discovery, erase_discovery, h]

/- Concrete non-vacuity and an explicit counterexample to inference of absence. -/
def exampleScope : Scope := ⟨⟨1⟩, ⟨42⟩, ⟨7⟩⟩
def exampleCase : Binding := ⟨⟨1⟩, ⟨42⟩, ⟨7⟩, ⟨17⟩, ⟨2⟩, ⟨8⟩, ⟨9⟩, ⟨1⟩⟩
def examplePolicy : Policy := ⟨fun _ => True, fun _ => True⟩

theorem example_eligible : Eligible exampleScope examplePolicy exampleCase := by
  exact ⟨rfl, rfl, rfl, trivial, trivial⟩

theorem empty_ranking_can_miss_real_case :
    Eligible exampleScope examplePolicy exampleCase ∧
    ¬ Hit exampleScope examplePolicy [] exampleCase := by
  refine ⟨example_eligible, ?_⟩
  intro h
  cases h with
  | issue mem _ => cases mem

/- Once the ranker can see only a projected corpus, invisible rows cannot
   influence even the ordering for a fixed profile, budget and random seed.
   To use this theorem the backend must also scope IDF, vector windows and
   any reason/count metadata consumed by the reader. -/
def publicSearch {World View Result : Type} (project : World → View)
    (rank : View → Result) (w : World) : Result := rank (project w)

theorem same_visible_input_same_response {World View Result : Type}
    (project : World → View) (rank : View → Result) (a b : World)
    (h : project a = project b) :
    publicSearch project rank a = publicSearch project rank b := congrArg rank h

theorem example_open : Opened exampleScope examplePolicy examplePolicy [exampleCase] exampleCase := by
  exact .peek (.issue (by simp) example_eligible) example_eligible

#print axioms hit_is_eligible
#print axioms revoked_cannot_open
#print axioms another_binding_cannot_open_from_singleton
#print axioms erase_discovery
#print axioms equal_witnesses_equal_authority
#print axioms empty_ranking_can_miss_real_case
#print axioms same_visible_input_same_response
#print axioms example_open
end Writfold.CaseDiscovery
