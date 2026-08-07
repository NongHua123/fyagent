# Desktop Visual-Baseline Policy

This directory intentionally has no accepted PNG baseline while local execution
is mock-only. A candidate runner captures each region twice with the fixed
fixture, uses fake IPC with external network blocked, and compares only within
the same platform, architecture, scale, theme and locale identity.

Read-only checks:

```bash
mise run test:desktop:visual:preflight
mise run test:desktop:mock
```

They do not create, overwrite or accept images and do not prove a native
installer/runtime result.

A baseline change is explicit and modifying:

```bash
mise run test:desktop:visual:update <reviewed-evidence>
```

The task must verify candidate metadata and require human confirmation. The
baseline change is reviewed separately; ordinary tests, bootstrap and CI must
never auto-accept it. PNG assets remain tracked with Git LFS where configured.
