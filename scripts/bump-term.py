# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term-mobile — https://github.com/bboxy450/bboxy450-term-mobile
# Credit copied, adapted and translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term-mobile/blob/main/ATTRIBUTION.md

from pathlib import Path
import os
import re
import subprocess
import sys

# Inspired by bboxy450-term/scripts/bump-harness.py: fetch, verify the pin,
# refresh the lock and stage them together with rollback.
# https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
root = Path(__file__).resolve().parents[1]
member = "vendor/bboxy450-term"
child = root / member
lock = root / "Cargo.lock"
url = "https://github.com/bboxy450/bboxy450-term"
env = {**os.environ, "GIT_TERMINAL_PROMPT": "0", "GCM_INTERACTIVE": "never"}


def git(*arguments, cwd=root):
    return subprocess.check_output(["git", *arguments], cwd=cwd, env=env, text=True, timeout=180).strip()


def bump(revision):
    if not (child / ".git").exists():
        git("submodule", "update", "--init", "--", member)
    if git("remote", "get-url", "origin", cwd=child) != url:
        raise ValueError("shared kit origin does not match .gitmodules")
    if git("status", "--porcelain", cwd=child):
        raise ValueError("shared kit checkout is dirty")
    if git("diff", "--name-only", "--", member, "Cargo.lock"):
        raise ValueError("kit pin and Cargo.lock must match the index")
    git("fetch", "origin", "main:refs/remotes/origin/main", cwd=child)
    if revision and not re.fullmatch(r"[0-9a-fA-F]{7,40}", revision):
        raise ValueError("REV must be a commit SHA")
    if revision and git("rev-parse", "--is-shallow-repository", cwd=child) == "true":
        main = git("rev-parse", "origin/main", cwd=child)
        if not main.startswith(revision.lower()):
            git("fetch", "--unshallow", "origin", "main:refs/remotes/origin/main", cwd=child)
    sha = git("rev-parse", "--verify", f"{revision or 'origin/main'}^{{commit}}", cwd=child)
    git("merge-base", "--is-ancestor", sha, "origin/main", cwd=child)
    before = git("rev-parse", "HEAD", cwd=child)
    original_lock = lock.read_bytes()
    git("checkout", "--detach", sha, cwd=child)
    try:
        subprocess.run(["cargo", "metadata", "--format-version", "1", "--filter-platform",
                        "aarch64-apple-darwin"], cwd=root, env=env, check=True, timeout=180, stdout=subprocess.DEVNULL)
        git("add", "--", member, lock.name)
    except (OSError, subprocess.SubprocessError):
        git("checkout", "--detach", before, cwd=child)
        lock.write_bytes(original_lock)
        raise
    print(f"term kit: staged {sha} and Cargo.lock")


if __name__ == "__main__":
    try:
        bump(sys.argv[1])
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        sys.exit(f"bump-term: {error}")
