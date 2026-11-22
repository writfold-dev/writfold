#!/usr/bin/env python3
"""Offline Rust quality producer; every required semantic result is retained."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import shutil
import tempfile
import time
import tomllib


class Producer:
    def __init__(self, root: Path, evidence: Path, timeout: int) -> None:
        self.root = root.resolve()
        self.evidence = evidence.resolve()
        self.evidence.mkdir(parents=True, exist_ok=True)
        self.timeout = timeout
        self.checks: list[dict] = []
        self.fixtures: list[Path] = []
        self.preserved_fixtures: set[Path] = set()
        self.env = dict(os.environ)
        self.env.setdefault("CARGO_TARGET_DIR", "/cache/target/quality")
        self.target = Path(self.env["CARGO_TARGET_DIR"]).resolve()
        self.binary = self.target / "debug/writfold-xtask"

    def record(self, name: str, passed: bool, **details: object) -> None:
        self.checks.append(dict(name=name, required=True, status="pass" if passed else "fail", **details))
        self.flush()

    def flush(self) -> None:
        # Only freshly generated synthetic workspaces are made operator-readable.
        # Never change permissions on source mounts or prior sealed attempts.
        for root in self.fixtures:
            root.chmod(0o755)
            for path in root.rglob("*"):
                if path.is_symlink():
                    raise OSError(f"unexpected synthetic fixture symlink: {path}")
                if path.is_dir():
                    path.chmod(0o755)
                elif root not in self.preserved_fixtures:
                    path.chmod(0o644)
        artifacts = []
        for path in sorted(self.evidence.rglob("*")):
            if path.is_file() and path.name != "obligations.json":
                artifacts.append({"path": str(path.relative_to(self.evidence)),
                                  "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
        receipt = dict(checks=self.checks, skips=[], artifacts=artifacts,
                       fixtures=[str(root.relative_to(self.evidence)) for root in self.fixtures])
        (self.evidence / "obligations.json").write_text(json.dumps(receipt, indent=2) + "\n")

    def command(self, name: str, args: list[str], cwd: Path | None = None) -> tuple[int, str]:
        started = time.monotonic()
        stdout = self.evidence / f"{name}.stdout"
        stderr = self.evidence / f"{name}.stderr"
        code = 125
        with stdout.open("wb") as out, stderr.open("wb") as err:
            try:
                result = subprocess.run(args, cwd=cwd or self.root, env=self.env,
                                        stdout=out, stderr=err, timeout=self.timeout, check=False)
                code = result.returncode
            except (OSError, subprocess.TimeoutExpired) as error:
                err.write((str(error) + "\n").encode())
        (self.evidence / f"{name}.command.json").write_text(json.dumps(
            dict(argv=args, cwd=str(cwd or self.root), exit_status=code,
                 elapsed_seconds=time.monotonic() - started), indent=2) + "\n")
        return code, stdout.read_text(errors="replace")

    def simple(self, name: str, args: list[str]) -> bool:
        code, _ = self.command(name, args)
        self.record(name, code == 0, exit_status=code)
        return code == 0

    def metadata(self, name: str, root: Path, locked: bool) -> Path | None:
        args = ["cargo", "metadata", "--format-version", "1", "--all-features", "--offline"]
        if locked:
            args.append("--locked")
        code, output = self.command(name, args, root)
        try:
            parsed = json.loads(output)
            valid = code == 0 and bool(parsed["packages"]) and bool(parsed["resolve"]["nodes"])
        except (ValueError, KeyError, TypeError):
            valid = False
        self.record(name, valid, exit_status=code)
        return self.evidence / f"{name}.stdout" if valid else None

    def inspect(self, name: str, root: Path, metadata: Path, expected: str | None = None) -> None:
        report = self.evidence / f"{name}.report.json"
        report.unlink(missing_ok=True)
        code, _ = self.command(name, [str(self.binary), "quality", "--root", str(root),
                                      "--metadata", str(metadata), "--report", str(report)])
        try:
            result = json.loads(report.read_text())
            rules = [entry["rule"] for entry in result["diagnostics"] if entry["severity"] == "error"]
            passed = result["complete"] is True and (
                (code == 0 and result["success"] is True and not rules) if expected is None else
                (code == 1 and result["success"] is False and expected in rules
                 and not ({"syntax", "macro-parse"} - {expected}).intersection(rules)))
            if expected in {"syntax", "macro-parse"}:
                matched = [entry for entry in result["diagnostics"] if entry["rule"] == expected]
                passed = passed and all(entry.get("line", 0) > 0 and entry.get("column", 0) > 0
                                        and entry.get("end_line", 0) >= entry["line"]
                                        for entry in matched)
        except (OSError, ValueError, KeyError, TypeError):
            passed, rules = False, []
        self.record(name, passed, exit_status=code, expected_rule=expected, observed_rules=rules)

    def fixture(self, name: str, source: str, package: str = "writfold-domain", main: bool = False) -> Path:
        root = Path(tempfile.mkdtemp(prefix=f"{name}-", dir=self.evidence))
        self.fixtures.append(root)
        (root / "src").mkdir()
        (root / "src" / ("main.rs" if main else "lib.rs")).write_text(source)
        (root / "Cargo.toml").write_text(
            f'[workspace]\n[package]\nname="{package}"\nversion="0.0.0"\nedition="2024"\n')
        return root

    def syntax_calibrations(self) -> None:
        cases = [
            ("safe-control", 'pub fn safe() -> &\'static str { "https://host//not-a-comment" }\n', None),
            ("comments-control", "// ignored\n" * 450 + "pub fn safe() {}\n", None),
            ("platform-guard", '#[cfg(not(target_os="linux"))] compile_error!("unsupported platform"); pub fn allowed() {}', None),
            ("test-control", '#[cfg(test)] mod tests { #[test] fn allowed() { Some(1).unwrap(); } }', None),
            ("rust-seq-reject", 'fn forbidden() { let _ = seq![1, 2]; }', "opaque-macro"),
            ("rust-opaque-bare-macro-argument", 'macro_rules! record { ($meta:meta) => { #[$meta] fn hidden() {} }; } record!(verifier::opaque);', "macro-argument"),
            ("rust-verifier-field-macro", 'macro_rules! record { ($field:ident : $ty:ty) => { struct Row { $field: $ty } }; } record!(verifier: u64);', None),
            ("rust-proof-identifiers", 'struct Audit { ghost: bool, tracked: bool } fn proof(spec: Audit) -> bool { let ghost = spec.ghost; let tracked = spec.tracked; ghost || tracked }', None),
            ("rust-proof-expression", 'fn proof() -> bool { true } fn runtime() -> bool { proof() }', None),
            ("rust-proof-macro", 'macro_rules! record { () => { struct Audit { ghost: bool } fn proof(ghost: Audit) -> bool { ghost.ghost } }; } record!();', None),
            ("rust-proof-json", 'fn proof() -> bool { true } fn runtime() { let _ = json!({"proof": proof()}); }', None),
            ("rust-proof-runtime", 'struct Audit { ghost: Option<u8> } fn proof(spec: Audit) { spec.ghost.unwrap(); }', "production-panic"),
            ("rust-proof-macro-runtime", 'macro_rules! record { () => { struct Audit { ghost: Option<u8> } fn proof(spec: Audit) { spec.ghost.unwrap(); } }; } record!();', "production-panic"),
            ("rust-proof-json-runtime", 'fn runtime() { let ghost = Some(1); let _ = json!({"proof": ghost.unwrap()}); }', "production-panic"),
            ("rust-proof-syntax-reject", 'proof fn not_rust() {}', "syntax"),
            ("derive-control", '#[derive(Clone, Debug)] pub struct Identity(String);', None),
            ("test-lint-control", '#[cfg(test)] mod tests { #[allow(clippy::unwrap_used)] #[test] fn allowed() { Some(1).unwrap(); } }', None),
            ("module-budget", "\n".join(f"const N{i}: u8 = 0;" for i in range(401)), "module-budget"),
            ("function-budget", "pub fn oversized() {\n" + "let _ = 0;\n" * 81 + "}\n", "function-budget"),
            ("nested-function", "fn outer() { fn inner() {\n" + "let _ = 0;\n" * 81 + "} }", "function-budget"),
            ("unsafe", "pub fn forbidden() { unsafe {} }", "unsafe"),
            ("unsafe-allowance", "#![allow(unsafe_code)]\npub fn safe() {}", "unsafe-allowance"),
            ("domain-deserialize", "#[derive(serde::Deserialize)] pub struct Identity(String);", "domain-deserialize"),
            ("conditional-deserialize", '#[cfg_attr(feature="untrusted", derive(serde::Deserialize))] pub struct Identity(String);', "domain-deserialize"),
            ("macro-deserialize", 'macro_rules! record { ($(#[$meta:meta])* $name:ident) => { $(#[$meta])* struct $name(String); }; } record!(#[derive(serde::Deserialize)] Identity);', "domain-deserialize"),
            ("core-io", "use std::{fs as filesystem}; pub fn leak() { let _ = filesystem::read(\"x\"); }", "core-io"),
            ("macro-runtime", "macro_rules! hidden { () => { fn hidden() { Some(1).unwrap(); } }; } hidden!();", "production-panic"),
            ("macro-budget", "macro_rules! hidden { () => { fn hidden() {\n" + "let _ = 0;\n" * 81 + "} }; } hidden!();", "function-budget"),
            ("json-runtime", 'fn hidden() { json!({"key": Some(1).unwrap()}); }', "production-panic"),
            ("vec-closure-budget", "fn outer() { let _ = vec![|| {\n" + "let _ = 0;\n" * 81 + "}; 2]; }", "function-budget"),
            ("opaque-declarations", "unknown_generator! { fn hidden() {} }", "opaque-macro"),
            ("expect-attribute-control", '#[expect(dead_code, reason = "calibration")] fn allowed() {}', None),
            ("expect-attribute-allowance", '#[expect(clippy::unwrap_used, reason = "calibration")] pub fn hidden() {}', "unsafe-allowance"),
            ("expect-path-runtime", 'pub fn hidden() -> fn(Option<u8>, &str) -> u8 { Option::expect }', "production-panic"),
        ]
        for name, source, rule in cases:
            root = self.fixture(name, source)
            metadata = self.metadata(f"calibration-{name}-metadata", root, False)
            if metadata:
                self.inspect(f"calibration-{name}", root, metadata, rule)
        root = self.fixture("orchestration", "\n".join(f"const N{i}: u8 = 0;" for i in range(251)), "writfold", True)
        metadata = self.metadata("calibration-orchestration-metadata", root, False)
        if metadata:
            self.inspect("calibration-orchestration", root, metadata, "module-budget")
        for level in ("forbid", "allow"):
            root = self.fixture(f"manifest-{level}", "pub fn safe() {}")
            with (root / "Cargo.toml").open("a") as manifest:
                manifest.write(f'\n[lints.rust]\nunsafe_code="{level}"\n')
            metadata = self.metadata(f"calibration-manifest-{level}-metadata", root, False)
            if metadata:
                self.lint_policy(f"calibration-manifest-{level}", root, metadata, reject=level == "allow")

    def module_calibrations(self) -> None:
        for name, escaped, panic in [
            ("module-owned", False, False),
            ("module-owned-panic", False, True),
            ("module-escape", True, False),
        ]:
            root = self.fixture(name, "")
            if escaped:
                selected = root.parent / (root.name + "-outside.rs")
                relative = "../../" + selected.name
            else:
                selected = root / "src/nested/selected.rs"
                relative = "nested/selected.rs"
                selected.parent.mkdir()
            selected.write_text("pub fn operation() { Some(1).unwrap(); }" if panic
                                else "pub fn operation() {}")
            (root / "src/lib.rs").write_text(
                f'#[path={json.dumps(relative)}] mod selected;\n'
                'pub fn caller() { selected::operation(); }\n')
            metadata = self.metadata(f"calibration-{name}-metadata", root, False)
            if metadata:
                rule = "opaque-module" if escaped else "production-panic" if panic else None
                self.inspect(f"calibration-{name}", root, metadata, rule)

    def graph_calibrations(self) -> None:
        for kind, table, optional in [
            ("renamed", "dependencies", ""),
            ("optional", "dependencies", ", optional=true"),
            ("build", "build-dependencies", ""),
            ("dev", "dev-dependencies", ""),
            ("target", "target.'cfg(windows)'.dependencies", ""),
        ]:
            root = self.fixture(f"architecture-{kind}", "pub fn safe() {}")
            dependency = root / "runtime"
            (dependency / "src").mkdir(parents=True)
            (dependency / "src/lib.rs").write_text("pub fn effect() {}")
            (dependency / "Cargo.toml").write_text('[package]\nname="writfold-runtime"\nversion="0.0.0"\nedition="2024"\n')
            with (root / "Cargo.toml").open("a") as manifest:
                manifest.write(f'\n[{table}]\nrenamed={{package="writfold-runtime",path="runtime"{optional}}}\n')
            metadata = self.metadata(f"calibration-architecture-{kind}-metadata", root, False)
            if metadata:
                self.inspect(f"calibration-architecture-{kind}", root, metadata, "architecture")
        for package in ("openssl", "native-tls"):
            root = self.fixture(f"tls-{package}", "pub fn safe() {}")
            dependency = root / "forbidden"
            (dependency / "src").mkdir(parents=True)
            (dependency / "src/lib.rs").write_text("pub fn marker() {}")
            (dependency / "Cargo.toml").write_text(
                f'[package]\nname="{package}"\nversion="0.0.0"\nedition="2024"\n')
            with (root / "Cargo.toml").open("a") as manifest:
                manifest.write(f'\n[dependencies]\nrenamed={{package="{package}",path="forbidden"}}\n')
            metadata = self.metadata(f"calibration-tls-{package}-metadata", root, False)
            if metadata:
                self.inspect(f"calibration-tls-{package}", root, metadata, "tls")

    def nominal_calibrations(self) -> None:
        domain = self.root / "crates/writfold-domain"
        pairs = [("wrong-id", "ContentDigest", "GitObjectId"),
                 ("wrong-revision", "DesignRevision", "ProposalMaterialRevision"),
                 ("wrong-unit", "ByteCount", "GitObjectFormat")]
        for name, expected, supplied in pairs:
            for negative in (False, True):
                suffix = "reject" if negative else "control"
                actual = supplied if negative else expected
                source = (f"fn accept(_: writfold_domain::{expected}) {{}}\n"
                          f"fn exercise(value: writfold_domain::{actual}) {{ accept(value); }}\nfn main() {{}}\n")
                # Separate roots must not share a cached successful consumer.
                package = f"nominal-{name}-{suffix}-{time.monotonic_ns()}"
                root = self.fixture(f"{name}-{suffix}", source, package, True)
                with (root / "Cargo.toml").open("a") as manifest:
                    manifest.write(f'\n[dependencies]\nwritfold-domain={{path={json.dumps(str(domain))}}}\n')
                lock_code, _ = self.command(f"{name}-{suffix}-lock", ["cargo", "generate-lockfile", "--offline"], root)
                self.record(f"{name}-{suffix}-lock", lock_code == 0, exit_status=lock_code)
                if lock_code != 0:
                    continue
                code, output = self.command(f"{name}-{suffix}", ["cargo", "check", "--locked", "--offline", "--message-format=json"], root)
                try:
                    messages = [json.loads(line) for line in output.splitlines() if line.strip()]
                    finished = [message for message in messages if message.get("reason") == "build-finished"]
                    completed = len(finished) == 1 and finished[0].get("success") is (not negative)
                    errors = [message["message"] for message in messages
                              if message.get("reason") == "compiler-message" and message["message"]["level"] == "error"]
                    artifacts = [message for message in messages if message.get("reason") == "compiler-artifact"
                                 and message.get("target", {}).get("name") == package]
                    fresh_control = (len(artifacts) == 1 and artifacts[0].get("fresh") is False
                                     and artifacts[0]["target"].get("src_path") == str(root / "src/main.rs"))
                    intended = len(errors) == 1 and errors[0].get("code", {}).get("code") == "E0308"
                    if intended:
                        diagnostic = errors[0]
                        intended = expected in diagnostic.get("rendered", "") and supplied in diagnostic.get("rendered", "") and any(
                            span["is_primary"] and span["file_name"] == "src/main.rs" and span["line_start"] == 2
                            for span in diagnostic["spans"])
                    passed = completed and ((code == 101 and intended) if negative
                                            else (code == 0 and not errors and fresh_control))
                except (ValueError, KeyError, TypeError, AttributeError):
                    passed = False
                self.record(f"{name}-{suffix}", passed, exit_status=code,
                            expected_error="E0308" if negative else None,
                            consumer_package=package, source_sha256=hashlib.sha256(source.encode()).hexdigest())

    def lint_policy(self, name: str, root: Path, metadata: Path, reject: bool = False) -> None:
        workspace = tomllib.loads((root / "Cargo.toml").read_text()).get("workspace", {})
        packages = json.loads(metadata.read_text())
        violations = []
        for package in packages["packages"]:
            if package["id"] not in packages["workspace_members"]:
                continue
            manifest = tomllib.loads(Path(package["manifest_path"]).read_text())
            lints = manifest.get("lints", {})
            if lints.get("workspace") is True:
                lints = workspace.get("lints", {})
            level = lints.get("rust", {}).get("unsafe_code")
            if isinstance(level, dict):
                level = level.get("level")
            if level != "forbid":
                violations.append({"package": package["name"], "rule": "unsafe-manifest", "level": level})
        report = self.evidence / f"{name}.report.json"
        report.write_text(json.dumps({"violations": violations}, indent=2) + "\n")
        self.record(name, bool(violations) if reject else not violations, violations=violations)

    def run(self) -> None:
        self.simple("rustfmt", ["cargo", "fmt", "--all", "--", "--check"])
        self.simple("clippy-default", ["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--offline", "--", "-Dwarnings"])
        self.simple("clippy-all-features", ["cargo", "clippy", "--workspace", "--all-targets", "--all-features", "--locked", "--offline", "--", "-Dwarnings"])
        built = self.simple("xtask-build", ["cargo", "build", "-p", "writfold-xtask", "--locked", "--offline"])
        metadata = self.metadata("workspace-metadata", self.root, True)
        if metadata:
            self.lint_policy("workspace-unsafe-manifests", self.root, metadata)
        else:
            self.record("workspace-unsafe-manifests", False, reason="metadata prerequisite failed")
        if built and metadata:
            self.inspect("workspace-syntax-architecture", self.root, metadata)
        else:
            self.record("workspace-syntax-architecture", False, reason="required build/metadata prerequisite failed")
        if built:
            self.syntax_calibrations()
            self.module_calibrations()
            self.graph_calibrations()
        else:
            self.record("checker-calibration", False, reason="xtask build failed; no calibration executed")
        self.nominal_calibrations()
        # Every mutant's site matches its kernel file exactly once (#92):
        # seconds here, instead of a refused claims run.
        self.simple("mutant-sites", [sys.executable, "proofs/mutants.py", "--check-sites"])
        # The translation splitter keeps every declaration (#95).
        self.simple("split-translation", [sys.executable, "proofs/split.py", "--self-test"])
        self.flush()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--evidence", type=Path, default=Path("/evidence"))
    parser.add_argument("--timeout", type=int, default=900)
    args = parser.parse_args()
    producer = Producer(args.root, args.evidence, args.timeout)
    try:
        producer.run()
    except Exception as error:
        producer.record("producer-completion", False, reason=f"{type(error).__name__}: {error}")
    return 0 if producer.checks and all(check["status"] == "pass" for check in producer.checks) else 1


if __name__ == "__main__":
    raise SystemExit(main())
