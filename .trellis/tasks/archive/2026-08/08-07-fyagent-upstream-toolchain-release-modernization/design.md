# Parent Design

The parent is an orchestration and evidence boundary, not a large implementation commit. Child 1 creates the upstream merge baseline. Children 2–5 then proceed in the approved order and in independently reviewable commits. Child 6 closes documentation and long-term contract drift after behavior is stable, while release-dependent evidence remains open until the closeout PR.

Dependency graph:

```text
merge upstream
   └─ remove cross builds
       └─ redesign mise/uv environment and version contract
           └─ modernize CI/Labeler/unsigned Release
               └─ remediate DEP0040
                   └─ migrate docs/Trellis and archive superseded tasks
                       └─ implementation PR → main CI → preflight → v0.3.0
                           └─ closeout PR → archive children → archive parent → journal
```

CI implementation details are delegated to the CI child while preserving the confirmed runner, pinning, permission, Required-gate, automatic Labeler, unsigned-only, attestation, and release-asset constraints. The parent tracks cross-child invariants, GO/NO-GO gates, the accepted workflow-only source-protection risk, and final remote evidence.
