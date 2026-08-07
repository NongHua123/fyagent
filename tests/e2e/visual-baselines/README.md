# Desktop Visual-Baseline Policy

This directory intentionally has no accepted PNG baseline while local execution
is mock-only. A candidate runner captures each region twice with the fixed
fixture, uses fake IPC with external network blocked, and compares only within
the same platform, architecture, scale, theme, and locale identity.

Read-only checks:

```bash
mise run test:desktop:visual:preflight
mise run test:desktop:mock
```

They do not create, overwrite, or accept images and do not prove a native
installer or runtime result. Preflight validates policy and metadata only.

A baseline change is an explicit source-modifying operation:

```bash
mise run test:desktop:visual:update <reviewed-evidence>
```

The task validates candidate evidence but does not accept PNGs automatically.
A human-reviewed change must separately add or replace the image, and its
evidence must identify platform, architecture, scale, theme, locale, fixture,
and repeated-capture result. PNG assets remain tracked with Git LFS where
configured. Bootstrap, ordinary checks, and CI must never auto-accept a
baseline.
