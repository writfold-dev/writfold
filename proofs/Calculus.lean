import Calculus.Word
import Calculus.Account
import Calculus.Evidence
import Calculus.Site
import Calculus.Referent
import Calculus.Knowledge

/-!
# The calculus (docs/rfcs/2026-11-27-calculus-in-lean.md)

A specification in plain Lean, independent of the Rust and its
translation: the words a judgment says and the laws they obey. No file
under `Calculus/` imports anything but `Calculus.*`; the proofs gate
refuses any other import. Kernels are linked to it from #206 on.
-/
