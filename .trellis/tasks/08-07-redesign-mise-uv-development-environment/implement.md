# Redesign mise and uv development environment — Implementation Plan

1. introduce the exact standard version files, locked uv declaration, and non-package uv project
2. split task definitions by domain
3. implement cross-platform check scripts
4. migrate Trellis/hooks to locked/no-sync/offline uv execution and exercise fallback/failure protocols
5. update the version tool, apply the atomic `0.3.0` bump, and regenerate/structurally validate locks
6. generate task docs and run strict local plus available platform checks
7. [implemented locally; remote pending] add a native `windows-11-arm` closeout
   CI smoke that resolves the locked uv and Python versions, synchronizes with
   `uv sync --locked`, verifies `win-arm64`, and executes the Trellis wrapper
   contract before archiving this child

## Completion Evidence

Attach exact commands, relevant logs/artifacts, changed-file list, platform
scope, unresolved limitations, and the owning spec updates. Release evidence is
complete and GO, but the original Windows ARM64 uv/Python/Trellis acceptance
criterion remains open until a native Actions smoke passes; lock resolution and
successful ARM64 packaging do not substitute for that execution evidence. The
accepted workflow-only protection risk does not waive any version, lock, task,
hook, or platform-support gate.
