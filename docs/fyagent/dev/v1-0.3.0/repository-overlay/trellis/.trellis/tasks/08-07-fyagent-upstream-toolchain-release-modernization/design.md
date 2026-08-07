# Parent Design

The parent is an orchestration and evidence boundary, not a large implementation commit. Child 1 creates the upstream merge baseline. Children 2–5 may then proceed in controlled commits; Child 6 closes documentation and long-term contract drift after behavior is stable.

Dependency graph:

```text
merge upstream
   ├─ remove cross builds
   ├─ redesign mise/uv environment
   ├─ modernize CI/Release
   └─ remediate DEP0040
          └─ migrate docs/Trellis and archive superseded tasks
```

CI implementation details are delegated to the CI child while preserving the confirmed runner, pinning, permission, Required-gate, and release-asset constraints. The parent tracks only cross-child invariants, GO gates, and final evidence.
