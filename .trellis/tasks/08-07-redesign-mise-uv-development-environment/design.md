# Redesign mise and uv development environment — Design

## Architecture

Separate standard declarations, task orchestration, and complex implementation scripts. `.node-version`, `packageManager`, `rust-toolchain.toml`, and `.python-version` are the standard facts. `mise.toml` declares only locked uv plus task includes/settings; uv exclusively owns Python `3.14.7`, `.venv`, and the non-package project lock. One rule engine provides human/JSON strict environment evidence. Mutation and interactive tasks are explicit, default to dry-run/confirmation, and stay outside routine `check`.

Codex hooks execute through mise and `uv run --locked --no-sync --offline`: an unprepared environment returns a visible non-blocking fallback, while a damaged script or protocol fails closed. The version tool reads the Cargo workspace as the single product source and requires `--apply` for the atomic `0.3.0` update.

## Failure Policy

The task is fail-closed on exact tool versions, lock integrity, managed-Python ownership, task metadata/DAG/docs, hook protocol integrity, and version consistency. Unsupported platforms may not be silently presented as supported.

## Rollback

Keep the work in a reviewable child commit/series. Revert only this child when possible; document any dependency on the isolated upstream merge or earlier child output.
