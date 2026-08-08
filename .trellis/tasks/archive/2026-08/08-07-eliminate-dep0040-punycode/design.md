# Eliminate DEP0040 punycode root cause — Design

## Architecture

Delete the unnecessary compatibility layer because Node `24.19.0` is the strict project baseline. Validate successful, non-2xx text-error, empty-response, and cross-realm behavior rather than existence only. Distinguish legitimate newer `whatwg-url`/`tr46` dependencies by version and reverse origin; the forbidden chain is `cross-fetch → node-fetch@2 → whatwg-url@5 → tr46@0.0.3 → punycode`.

## Failure Policy

The task is fail-closed if native Fetch globals are missing, behavior probes fail, the obsolete lock path remains, any warning suppression is present, or ordinary/pending deprecations survive their required throw gates.

## Rollback

Keep the work in a reviewable child commit/series. Revert only this child when possible; document any dependency on the isolated upstream merge or earlier child output.
