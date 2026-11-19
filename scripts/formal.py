# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term-mobile — https://github.com/bboxy450/bboxy450-term-mobile
# Credit copied, adapted and translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term-mobile/blob/main/ATTRIBUTION.md

import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tomllib

root = Path(__file__).resolve().parents[1]
vendor = root / "vendor/bboxy450-term"
target = root / "target/formal"
models = target / "models"
# Use the pinned predecessor's TLC distribution, verdict and exact graph reader.
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("term_formal", vendor / "scripts/formal.py")
formal = importlib.util.module_from_spec(spec)
spec.loader.exec_module(formal)
expected = {"InboxLifecycle": "pass", "BrokenSurfaceRetention": "safety"}


def run():
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"], cwd=root
    ))
    coverage_path = root / "verification/coverage.toml"
    inventory = tomllib.loads(coverage_path.read_text())["crate"]
    packages = {item["name"] for item in metadata["packages"]}
    if len(inventory) != len(packages) or {row["name"] for row in inventory} != packages:
        raise ValueError("formal inventory must cover every mobile workspace crate")
    for row in inventory:
        if not row["source"] or not row["implementation_only"]:
            raise ValueError(f"missing source mapping or implementation coverage for {row['name']}")
    if {model for row in inventory for model in row["models"]} != expected.keys():
        raise ValueError("formal inventory must cover both the model and its negative control")

    rows = [line.split("\t") for line in (models / "manifest.tsv").read_text().splitlines()]
    if len(rows) != len(expected) or {name: verdict for name, verdict, _, _ in rows} != expected:
        raise ValueError("unexpected native model corpus")
    java, jar, pins = formal.tools(target)
    results = {}
    for name, verdict, count, width in rows:
        actual = formal.verdict(formal.tlc(java, jar, models, name, graph=False))
        if actual != verdict:
            raise ValueError(f"{name}: expected {verdict}, TLC returned {actual}")
        if formal.verdict(formal.tlc(java, jar, models, name, graph=True)) != "pass":
            raise ValueError(f"{name}: TLC did not finish exploring the full graph")
        tlc_graph = formal.dot_graph(models / f"{name}.dot", int(width))
        if len(tlc_graph[0]) != int(count) or tlc_graph != formal.native_graph(models, name):
            raise ValueError(f"{name}: Yarn and TLC graphs differ")
        results[name] = {"verdict": actual, "states": len(tlc_graph[0]), "edges": len(tlc_graph[1])}
        print(f"{name}: {actual}; {len(tlc_graph[0])} states, {len(tlc_graph[1])} edges (Yarn = TLC)")
    inputs = [coverage_path, Path(__file__), root / "Cargo.lock", root / "Cargo.toml",
              root / "rust-toolchain.toml", *root.glob("crates/*/Cargo.toml"),
              *root.glob("crates/*/examples/verify.rs"),
              *(root / p for row in inventory for p in row["source"])]
    evidence = {
        "git_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
        "shared_kit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=vendor, text=True).strip(),
        "files_sha256": {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
                         for p in inputs},
        "pins": pins,
        "results": results,
    }
    (target / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")


if __name__ == "__main__":
    run()
