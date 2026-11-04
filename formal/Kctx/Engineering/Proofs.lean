import Kctx.Engineering.Conservation
import Kctx.Engineering.ReuseSafety
import Kctx.Engineering.ConsumerSafety
import Kctx.Engineering.OperationalProofsPreservation
import Kctx.Engineering.OperationalProofsBudgetReachability
import Kctx.Engineering.HistorySafetyOrigin
import Kctx.Engineering.HistorySafetyRetention
import Kctx.Engineering.PromotionSafetyEvidence
import Kctx.Engineering.PromotionSafetyReachability
import Kctx.Engineering.CutSafety
import Kctx.Engineering.EffectsSafetyOwnershipTransitions
import Kctx.Engineering.EffectsSafetyOncePreservation
import Kctx.Engineering.EffectsSafetyReachable
import Kctx.Engineering.CompatibilitySafety
import Kctx.Engineering.BindingSafety
import Kctx.Engineering.ContinuationSafety
import Kctx.Engineering.ControlProofs

/-! The full engineering proof closure. Declarations are grouped by invariant
ownership; formal/engineering-theorems.json binds E01-E12 to exact statements.
Kernel acceptance, theorem fidelity, and implementation refinement are separate
obligations. No current-source result is implied by importing this aggregate. -/
