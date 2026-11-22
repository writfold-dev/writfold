import Aeneas

/-!
# Checked slice access

`slice.get(i)` (Rust's `<[T]>::get` at a `usize` index) as Aeneas
translates it: never fails, and is the list's `getElem?`. The kernel uses
it where an index may lie past the end (`idle::newer`,
`verdict::recorded::better`, `scope::char_step`, `scope::path_step`), so
those functions are total (docs/writfold/tauten-adoption.md).
-/

open Aeneas Aeneas.Std Result

namespace Writfold.Core

/-- `get` at a `usize` index is the list's `getElem?`, and never fails. -/
theorem slice_get_ok {T : Type} (s : Slice T) (i : Usize) :
    core.slice.Slice.get (core.slice.index.SliceIndexUsizeSlice T) s i = ok s.val[i.val]? := by
  simp [core.slice.Slice.get]

end Writfold.Core
