# Migrate documentation and Trellis contracts — Design

## Architecture

Use current specs as long-term contracts, task artifacts as implementation evidence, and concise multilingual README onboarding linked to one generated task catalog. Apply the repository overlay only through a three-way comparison after the upstream/configuration tree is stable. Exclude archives and generic template references from current-command scans.

The first documentation commit records implemented behavior and archives the five old tasks as `superseded`. Release-dependent validation remains pending. After the public Release, a closeout PR records real run URLs, asset summaries, digests, attestations, and unsigned checks, refreshes `MANIFEST.sha256`, and enables the final child/parent archive sequence.

## Failure Policy

The task is fail-closed on active-command drift, version/identity/provenance inconsistency, unsafe unsigned-install advice, false protection/signing claims, archive semantics, or missing remote release evidence.

## Rollback

Keep the work in a reviewable child commit/series. Revert only this child when possible; document any dependency on the isolated upstream merge or earlier child output.
