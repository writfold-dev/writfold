---
kind: edit
tool: splicery
profile: writfold-splicery-literal-edit/1
kernel: b14c67a7aca9f416c4facdf189ef340d5666351f
run: run-20261101T090000Z-a1b2c3
repo: demo-service
worktree: <WORLD>/demo-service
base: bcacd8cac84e6c72aa7e6b0de16f2593027da566
head: bcacd8cac84e6c72aa7e6b0de16f2593027da566
recipes: 2
files: 2
splices: 3
recorded: 2026-11-11T00:00:00Z
---

# Edit at demo-service@bcacd8cac84e

Applied in the run's worktree by `writfold kb edit apply`. The result is an ordinary uncommitted Git change there; gates judge it afterwards.

## Files

- `src/retry.rs` before=1dc692bcea050d9869e9f69db8837a8cc65e567c after=0dcc64bb0039d7b9638c9ddac33fe01cc3c5c9a9 splices=1 removed=12 inserted=12 recipe=retry.spl
  - line 2 bytes [25,37) -> 12 bytes, recipe [24,75)
- `src/ledger.rs` before=fbbd140bd8320a8694df1ad1ae73358ebc5af15c after=8e221df690985ac8a62bbe9a58b254f8addf266e splices=2 removed=20 inserted=51 recipe=ledger.spl
  - line 1 bytes [16,16) -> 30 bytes, recipe [191,242)
  - line 4 bytes [68,88) -> 21 bytes, recipe [98,146)

## Recipe retry.spl

sha256: 26651cf66726b83b1f9ffa9751942f2826f8304b02da981a253f07f96c31c4c6

````splicery
edit "src/retry.rs" {
  replace one text "attempts = 5" with "attempts = 7"
}
````

## Recipe ledger.spl

sha256: 6e9da0051bb025536e9728e5511a6baefc1470a9bcac4521a928877675e01eb1

````splicery
edit "src/ledger.rs" {
  each @r in line_matches rx"^(?<lead>[ ]+)append\(row\);$" expect 1 {
    replace @r with [keep(@r.lead), "append(row)?;"]
  }
  @doc := one line "// ledger writes"
  after @doc insert "\n// every row is appended once"
}
````

## Preview

````diff
--- a/src/retry.rs
+++ b/src/retry.rs
@@ -1,4 +1,4 @@
 pub fn retry() {
-    let attempts = 5;
+    let attempts = 7;
     backoff(attempts);
 }
--- a/src/ledger.rs
+++ b/src/ledger.rs
@@ -1,6 +1,7 @@
 // ledger writes
+// every row is appended once
 pub fn write(rows: &[Row]) {
     for row in rows {
-        append(row);
+        append(row)?;
     }
 }
````
