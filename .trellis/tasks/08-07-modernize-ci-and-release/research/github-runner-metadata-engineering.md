# Research: GitHub runner and job-container metadata engineering contract

- Query: Investigate the native Linux x64/ARM64 job-container failure `write-platform-metadata.mjs: ImageOS missing`; determine a documented, fail-closed metadata contract for runner routing/context, the exact OCI child digest, `/etc/os-release`, `uname -m`, and artifact attestations; compare schema migration and test options; recommend exact workflow, writer, type, validator, spec, and test changes.
- Scope: mixed (local candidate patch and contracts; official GitHub Actions documentation and source; OCI/Docker and Linux primary documentation)
- Date: 2026-08-08

## Findings

### Decision

1. `ImageOS` and `ImageVersion` must not be required, emitted, retained as nullable compatibility fields, or renamed as if they were authoritative. They are hosted runner-image implementation details, not fields in GitHub's documented default-variable or runner-context contract. In a Linux job container they also describe the host image, not the Ubuntu 22.04 build user space, when they happen to exist.
2. The current interrupted candidate patch correctly removes the immediate undocumented dependency and maps `${{ runner.os }}` / `${{ runner.arch }}` into workflow-owned variables. That is the minimum safe fix, but it is not yet the complete engineering-grade contract: its top-level `runnerLabel` can be mistaken for an observed label, `containerDigest` is a configured value presented without its source, and the container checks are not recorded in the metadata.
3. Represent provenance by source and semantics, not by an undifferentiated collection of values:
   - `runner.requestedLabel` is the exact workflow routing request from the matrix. It is not a runtime-discovered host label or host image identity.
   - `runner.context.os` and `runner.context.arch` are the exact documented GitHub runner-context values supplied to the metadata step through workflow-owned environment variables.
   - `container.configuredImage.reference` and `container.configuredImage.manifestDigest` are the exact digest-pinned OCI child image requested by `jobs.<job_id>.container.image`. They are configuration evidence, not a digest measured from inside the container.
   - `container.observed.osRelease.id`, `container.observed.osRelease.versionId`, and `container.observed.unameMachine` are values measured immediately before metadata emission inside the job container.
   - non-container Windows/macOS records use `container: null` and must reject partial container fields.
4. Do not add `verified: true`, `actualImageDigest`, or `hostImageVersion`. A boolean does not carry evidence; `/etc/os-release` and `uname -m` do not independently prove an OCI digest; and the workflow has no documented, stable API that exposes an exact GitHub-hosted runner image version to a job container.
5. Keep `build-metadata.json` as an attestation subject. GitHub's attestation binds the resulting bytes and workflow provenance, but does not independently validate the truth of custom JSON fields or synthesize the container digest. The writer and aggregate validator therefore remain responsible for the semantic checks below.
6. Local evidence says neither failed preflight reached aggregation/attestation/publication and no tag or Release was created (`research/ci-release-local-evidence.md:65-70`), with formal Release still `NO-GO` (`research/ci-release-local-evidence.md:98-100`). On that evidence, the preferred migration is to finalize the unpublished `fyagent-platform-build/v1` and `fyagent-build-metadata/v1` shapes in place before their first formal publication. This is conditional: if a read-only release/artifact inventory later finds any published or consumed v1 record, bump both schemas together to `/v2` and accept only v2 in the v0.3.0 formal path.

### Files found

| File                                                                            | Relevant evidence                                                                                                                                                       |
| ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.github/workflows/release.yml:540-554`                                         | Windows maps the configured matrix runner and documented runner context into the writer.                                                                                |
| `.github/workflows/release.yml:572-607`                                         | Linux selects explicit x64/ARM64 host labels and exact per-architecture Ubuntu 22.04 child digests; the digest is workflow configuration.                               |
| `.github/workflows/release.yml:609-625`                                         | Linux currently verifies `RUNNER_ARCH`, `/etc/os-release` `ID`/`VERSION_ID`, and `uname -m` before installing dependencies.                                             |
| `.github/workflows/release.yml:674-691`                                         | Linux rechecks `uname -m` and records measured toolchain values, but not the OS-release observation.                                                                    |
| `.github/workflows/release.yml:742-752`                                         | The Linux metadata step passes runner context but does not remeasure or pass `/etc/os-release` and `uname -m`.                                                          |
| `scripts/release/write-platform-metadata.mjs:13-18`                             | Missing or blank owned inputs fail closed through `required()`.                                                                                                         |
| `scripts/release/write-platform-metadata.mjs:41-74`                             | Candidate schema already removed `imageOs`/`imageVersion`, but still emits ambiguous `runnerLabel` and `containerDigest`.                                               |
| `scripts/release/write-platform-metadata.mjs:75-77`                             | `flag: "wx"` correctly prevents silent replacement of an existing record.                                                                                               |
| `scripts/release/release-contract.mjs:88-131`                                   | Expected targets bind runner labels, selected runner architectures, and Linux child digests, but not expected runner OS or measured container facts.                    |
| `scripts/release/release-contract.mjs:322-395`                                  | Aggregate validation checks the digest exactly but only requires a non-empty runner OS; it has no exact-key or observed-container validation.                           |
| `scripts/release/release-contract.mjs:487-501`                                  | The aggregator returns each parsed record and strips only `identity`; unknown root fields can therefore pass through into attested aggregate metadata.                  |
| `scripts/release/release-contract.d.mts:17-24`                                  | `ExpectedTarget` has no expected runner OS or structured configured/observed container contract.                                                                        |
| `scripts/release/release-contract.d.mts:64-80`                                  | The declared platform shape mirrors the ambiguous fields and does not describe the writer's `identity` input record separately from an aggregate target.                |
| `tests/releaseAssets.test.ts:60-95`                                             | Aggregate fixtures are handwritten; they do not exercise the writer process or its environment behavior.                                                                |
| `tests/releaseAssets.test.ts:184-222`                                           | The happy-path aggregate test checks only the two runner keys, not exact root/container shapes.                                                                         |
| `tests/releaseAssets.test.ts:297-316`                                           | Current negative coverage changes only the Linux digest.                                                                                                                |
| `tests/releaseWorkflow.test.ts:377-408`                                         | Static workflow coverage binds digest, `RUNNER_ARCH`, `uname -m`, and the no-QEMU decision.                                                                             |
| `tests/releaseWorkflow.test.ts:410-458`                                         | Candidate coverage proves owned runner-context mapping and absence of retired image variable names, but not runtime writer behavior or recorded container observations. |
| `.trellis/spec/backend/github-release-workflow.md:133-169`                      | The spec defines host labels, child digests, native checks, and the candidate removal of undocumented hosted-image variables.                                           |
| `.trellis/spec/backend/github-release-workflow.md:284-294`                      | Exactly five records feed the aggregate; the aggregate and installer manifest are attestation subjects.                                                                 |
| `docs/fyagent/dev/v1-0.3.0/06-CI-AND-RELEASE-DESIGN.md:178-181`                 | Public design already says `ImageOS`/`ImageVersion` are outside the schema.                                                                                             |
| `docs/fyagent/dev/v1-0.3.0/implementation-map/RELEASE-ASSET-CONTRACT.md:91-104` | Public asset contract names v1 and requires five runner/digest-bound inputs.                                                                                            |

### Root cause and why both architectures fail identically

The failure is a contract error, not an x64/ARM64 image-selection error:

1. GitHub documents `runner.os` and `runner.arch` in the runner context, and `RUNNER_OS` / `RUNNER_ARCH` in the default-variable catalog. The public catalogs do not list `ImageOS` or `ImageVersion`.
2. The authoritative `actions/runner-images` Ubuntu provisioning script writes `ImageVersion` and `ImageOS` into the hosted VM's `/etc/environment`. That is runner-image construction behavior. The runner-images README also treats the image version as information shown in the `Set up job` log and warns that images are regularly updated; it is not an immutable property of `ubuntu-24.04`.
3. GitHub's job-container contract says shell steps run in the specified container. The runner implementation creates/pulls that container and assembles the environment supplied to container steps. It does not establish the host image's `/etc/environment` as a public job-container API.
4. Consequently, `ImageOS` can be present in a native hosted-runner step yet absent in the same job's container. Both Linux architectures use the same job-container boundary and therefore fail the same requirement before architecture-specific provenance matters.
5. Even if the host value were propagated, recording it as the build OS would be semantically false here: the requested host label is Ubuntu 24.04 while the build user space is the digest-pinned Ubuntu 22.04 child image.

The candidate's use of `${{ runner.os }}` and `${{ runner.arch }}` is therefore the correct documented source. Passing them as `ACTUAL_RUNNER_OS` and `ACTUAL_RUNNER_ARCH` also makes the writer independent of ambient process-environment naming. Poisoning ambient `RUNNER_OS`, `RUNNER_ARCH`, `ImageOS`, and `ImageVersion` in a direct test should prove that boundary.

### Evidence taxonomy

| Fact                                        | Status and source                                                                                               | Truthful claim                                                                                                                        | Must not be claimed                                                                                                    |
| ------------------------------------------- | --------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `matrix.runner` / `runs-on` value           | Documented workflow configuration                                                                               | The workflow requested this runner label.                                                                                             | That the label is a measured runtime property or exact host image identity.                                            |
| `${{ runner.os }}`                          | Documented runner context; enumerated values include `Linux`, `Windows`, `macOS`                                | GitHub exposed this high-level runner OS for the job.                                                                                 | Host distribution/version or container distribution/version.                                                           |
| `${{ runner.arch }}`                        | Documented runner context; enumerated values include `X86`, `X64`, `ARM`, `ARM64`                               | GitHub exposed this runner architecture for the job.                                                                                  | Output package architecture, container manifest digest, or universal binary architecture.                              |
| `${{ runner.environment }}`                 | Documented runner context (`github-hosted` or `self-hosted`)                                                    | Available if the product later needs to bind hosting class.                                                                           | Exact runner image version. It is not needed to resolve this task and should not be added without a contract decision. |
| `RUNNER_OS` / `RUNNER_ARCH`                 | Documented default variables                                                                                    | High-level runner facts when read directly.                                                                                           | An advantage over explicit context-to-owned-variable mapping; using both sources adds ambiguity.                       |
| `ImageOS` / `ImageVersion`                  | `actions/runner-images` implementation and runner release history; absent from public variable/context catalogs | At most diagnostic hosted-image details when available on a particular runner implementation.                                         | Required portable metadata, job-container build OS, stable provenance, or cross-run identity.                          |
| `ubuntu-24.04` / `ubuntu-24.04-arm`         | Documented hosted-runner labels and explicit workflow request                                                   | Which runner pool label was requested.                                                                                                | An immutable VM image; hosted runner images update over time.                                                          |
| `docker.io/library/ubuntu:22.04@sha256:...` | Exact `container.image` workflow configuration; digest semantics defined by Docker/OCI                          | The job requested a repository image by an immutable content digest; the two matrix entries select platform-specific child manifests. | A digest independently measured by the metadata script inside the running container.                                   |
| `/etc/os-release` `ID` / `VERSION_ID`       | Measured inside the container; Linux `os-release(5)` machine-readable OS identity                               | The running build user space reported `ubuntu` / `22.04` at measurement time.                                                         | Host OS identity or proof of the image digest.                                                                         |
| `uname -m`                                  | Measured inside the container; `uname(2)` machine/hardware identifier from the active kernel/UTS view           | The running step observed `x86_64` or `aarch64`, corroborating the native architecture gate.                                          | User-space distribution identity or independent proof of the OCI manifest.                                             |
| GitHub artifact attestation                 | GitHub-generated signed statement binding subjects and workflow provenance                                      | The attested `build-metadata.json` bytes came from the recorded workflow execution and subject digest.                                | Independent semantic verification of custom JSON claims or a GitHub-generated container-image assertion.               |

The runner context schema is stable as a documented interface, but its values are observations for one run. A label may later route to a different image, and a label-to-architecture mapping can evolve. The aggregate validator must therefore check the exact acceptance mapping for this release rather than treat a label as permanent platform metadata.

### Recommended metadata shape

Use one source-explicit shape for each platform record. The Linux x64 example is:

```json
{
  "schema": "fyagent-platform-build/v1",
  "targetGroup": "linux-x64",
  "platform": "linux",
  "architecture": "x64",
  "runner": {
    "requestedLabel": "ubuntu-24.04",
    "context": {
      "os": "Linux",
      "arch": "X64"
    }
  },
  "container": {
    "configuredImage": {
      "reference": "docker.io/library/ubuntu:22.04@sha256:0199853f6d6b20b0424f3c5694a72a62764f01e6a771b1eb48a4197848986c7e",
      "manifestDigest": "sha256:0199853f6d6b20b0424f3c5694a72a62764f01e6a771b1eb48a4197848986c7e"
    },
    "observed": {
      "osRelease": {
        "id": "ubuntu",
        "versionId": "22.04"
      },
      "unameMachine": "x86_64"
    }
  },
  "toolchain": {
    "node": "v24.19.0",
    "pnpm": "10.12.3",
    "rustc": "rustc 1.97.1 (...)"
  },
  "identity": {
    "...": "existing exact identity fields"
  }
}
```

For Windows and macOS, retain the same runner shape and emit exactly `"container": null`. Do not omit the key: explicit null distinguishes a native/non-container target from a malformed record in which collection failed.

Fully qualifying `docker.io/library/ubuntu` is recommended so the metadata does not rely on an implicit registry/namespace. The workflow and expected contract must use the same exact reference; the metadata writer must not invent a normalized reference that differs from workflow configuration.

The exact Linux acceptance map is:

| Target        | Requested label    | Runner context    | Configured OCI child digest                                               | Observed OS release | Observed `uname -m` |
| ------------- | ------------------ | ----------------- | ------------------------------------------------------------------------- | ------------------- | ------------------- |
| `linux-x64`   | `ubuntu-24.04`     | `Linux` / `X64`   | `sha256:0199853f6d6b20b0424f3c5694a72a62764f01e6a771b1eb48a4197848986c7e` | `ubuntu` / `22.04`  | `x86_64`            |
| `linux-arm64` | `ubuntu-24.04-arm` | `Linux` / `ARM64` | `sha256:a8cdd2158a73d7e5c02aa351fe269f48f57cf710a241db86e9ede371fc150149` | `ubuntu` / `22.04`  | `aarch64`           |

The exact runner architecture checks represented in `EXPECTED_TARGETS` must cover every target. GitHub's current hosted-runner table maps the pinned `macos-15` label to ARM64, so `macos-universal` must require `runner.context.arch=ARM64`. The output architecture remains `universal`; that output fact is distinct from, and cannot weaken, the runner-source fact.

### Required invariants

The workflow writer and aggregate validator should enforce these invariants:

1. The only root keys in an input record are `schema`, `targetGroup`, `platform`, `architecture`, `runner`, `container`, `toolchain`, and `identity`.
2. `runner` has exactly `requestedLabel` and `context`; `context` has exactly `os` and `arch`.
3. Runner OS equals the target platform mapping (`Linux`, `Windows`, or `macOS`), and runner arch equals the exact frozen target expectation for all five targets while also belonging to GitHub's documented enum.
4. A Linux record has a complete container object. Missing, blank, partial, extra, or null container evidence fails.
5. A non-Linux record has exactly `container: null`. Supplying any Linux container environment variable for a non-Linux target fails the writer instead of silently dropping a partial claim.
6. The manifest digest matches `^sha256:[0-9a-f]{64}$`, equals the exact expected target digest, and is the digest suffix of the exact configured image reference.
7. Linux observed values equal the target map above. `osRelease` records only `ID` and `VERSION_ID`; do not persist presentation strings such as `PRETTY_NAME`, and do not persist `uname -a` host/kernel noise when only `uname -m` is an acceptance input.
8. `toolchain` and `identity` retain their current exact checks and also use exact key sets.
9. Unknown keys are rejected at every nested level. After validation, the aggregator constructs a new allowlisted target object instead of spreading the parsed record. This closes the current `validate -> return metadata -> remove identity only` path at `release-contract.mjs:495-501`, through which arbitrary root fields could enter the signed aggregate.
10. `ImageOS`, `ImageVersion`, `imageOs`, and `imageVersion` are absent from the workflow inputs, writer output, declarations, aggregate output, and public/spec contracts. Ambient variables with those names are ignored, not treated as evidence and not rejected merely for existing on native hosted runners.

### Schema migration options

| Option                                                                                                         | Compatibility                              | Engineering result                                                                                                                                                                                   | Decision                                                                  |
| -------------------------------------------------------------------------------------------------------------- | ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Keep required `imageOs`/`imageVersion`                                                                         | Preserves an earlier draft shape           | Systematically fails job containers and mislabels host-image data as build-user-space provenance.                                                                                                    | Reject.                                                                   |
| Keep the fields but allow `null`                                                                               | Superficially minimizes consumer changes   | `null` conflates unavailable, not applicable, and collection failure; field names still promise an image identity the workflow cannot portably establish.                                            | Reject.                                                                   |
| Remove the fields and keep current `runnerLabel` / `runner.runnerOs` / `runner.runnerArch` / `containerDigest` | Smallest immediate patch                   | Correctly resolves the observed failure and avoids fabricated host-image data, but does not identify requested versus observed facts or preserve the already verified container user-space evidence. | Acceptable emergency minimum only; not the final contract requested here. |
| Replace with source-explicit `runner` and structured `container`                                               | Shape-breaking if v1 already has consumers | Gives each value one defensible meaning, supports exact cross-validation, and makes attested JSON interpretable without external tribal knowledge.                                                   | Recommended.                                                              |

Version handling:

- Preferred under the current local evidence: update the not-yet-published v1 input and aggregate shapes in place, update every local consumer/spec in the same patch, and do not emit the superseded draft shape.
- Mandatory fallback if any v1 is already public or consumed: change both schema identifiers to `fyagent-platform-build/v2` and `fyagent-build-metadata/v2`; update all five writers, aggregator, declarations, tests, and public docs atomically. The formal v0.3.0 validator accepts only v2. Do not silently read v1 and fill missing facts with nulls/defaults, because that would fabricate equivalence.
- Do not bump only the per-platform schema while leaving aggregate `/v1`: the aggregate `targets` shape changes too.
- Preflight artifacts that never reached a public Release do not by themselves require a compatibility reader; retain them as historical logs, not accepted release inputs.

### Exact recommended changes

#### `.github/workflows/release.yml`

1. Rename metadata-step input `RUNNER_LABEL` to `REQUESTED_RUNNER_LABEL` for all Windows, Linux, and macOS writers. Keep the owned mappings:

   ```yaml
   ACTUAL_RUNNER_OS: ${{ runner.os }}
   ACTUAL_RUNNER_ARCH: ${{ runner.arch }}
   ```

2. Add `container_image: docker.io/library/ubuntu:22.04` to both Linux matrix rows. Configure the job with the exact same value used as evidence:

   ```yaml
   container:
     image: ${{ matrix.container_image }}@${{ matrix.container_digest }}
   ```

3. Rename job/step metadata input `CONTAINER_DIGEST` to `CONTAINER_MANIFEST_DIGEST`, and pass `CONTAINER_IMAGE_REFERENCE` as the exact `${{ matrix.container_image }}@${{ matrix.container_digest }}` string. Do not call either value `ACTUAL_*`; they come from reviewed workflow configuration.
4. Change `Record Linux build metadata` to an explicit `shell: bash` step with `set -euo pipefail`. Immediately before invoking the writer:
   - source `/etc/os-release`;
   - require `ID=ubuntu` and `VERSION_ID=22.04`;
   - capture `actual_uname_machine="$(uname -m)"` and compare it to `${{ matrix.uname_arch }}`;
   - compare the owned runner context to `Linux` and `${{ matrix.expected_runner_arch }}`;
   - pass `ACTUAL_CONTAINER_OS_ID`, `ACTUAL_CONTAINER_OS_VERSION_ID`, and `ACTUAL_CONTAINER_UNAME_MACHINE` to the writer for that invocation.
5. Keep the early bootstrap checks at `release.yml:609-625`. The late check is not redundant: the early gate prevents expensive work in the wrong environment, while the late measurement ensures the attested record contains facts observed at emission time.
6. Windows/macOS steps pass no container variables. They still emit `container: null` through the writer.
7. Do not inspect the host Docker socket, container ID, runner `Set up job` log, or `/etc/environment` from the container. Those are implementation-specific paths and do not improve the portable contract.

#### `scripts/release/write-platform-metadata.mjs`

1. Keep it as a CLI and retain `writeFileSync(..., { flag: "wx" })`; no new runtime dependency or abstraction is needed for direct testing.
2. Read the target fields first, locate the target in the centrally defined `EXPECTED_TARGETS`, and fail on an unknown/mismatched target. Reuse the extended expected-target contract rather than copy the target/digest/OS/uname map into a second independent table.
3. Replace:

   ```text
   RUNNER_LABEL -> REQUESTED_RUNNER_LABEL
   CONTAINER_DIGEST -> CONTAINER_MANIFEST_DIGEST
   ```

   and construct the recommended `runner` / `container` objects.

4. Require owned runner context for every platform. Validate exact runner OS, documented arch enum, and the target's exact arch for all five targets.
5. For Linux, require all five configured/observed inputs: image reference, manifest digest, OS ID, OS version ID, and uname machine. Validate digest syntax, exact image/digest/target mapping, and exact observations before writing.
6. For non-Linux, reject any nonblank container input and emit `container: null`.
7. Never read ambient `RUNNER_OS`, `RUNNER_ARCH`, `ImageOS`, or `ImageVersion`. Never copy unknown environment variables into JSON.
8. Preserve the existing release-mode/Required-CI binding logic at lines 24-40 and existing exact identity/toolchain fields.

#### `scripts/release/release-contract.mjs`

1. Extend each `EXPECTED_TARGETS` entry with:
   - `expectedRunnerOs` for every target;
   - exact `expectedRunnerArch`, including `ARM64` for `macos-15`;
   - `expectedContainer: null` for Windows/macOS;
   - for Linux, exact `imageReference`, `manifestDigest`, `osReleaseId`, `osReleaseVersionId`, and `unameMachine`.
2. Replace the permissive `runnerOs` non-empty check with exact runner-context validation. Validate the configured and observed container object against the target map and enforce the digest/reference consistency invariant.
3. Add a reusable exact-key assertion and apply it to the record root, runner, runner context, container, configured image, observed facts, OS-release facts, toolchain, and identity.
4. Return a newly constructed canonical target object after validation. Do not return the parsed object and spread away only `identity`.
5. Preserve the exact-five-file check and all identity/toolchain gates. There is no compatibility/defaulting branch for the superseded draft shape.

#### `scripts/release/release-contract.d.mts`

1. Add documented unions for runner context values:

   ```ts
   type GitHubRunnerOS = "Linux" | "Windows" | "macOS";
   type GitHubRunnerArch = "X86" | "X64" | "ARM" | "ARM64";
   ```

2. Extend `ExpectedTarget` with exact runner OS/architecture and the nullable/structured expected container contract.
3. Replace the ambiguous fields in the target metadata type with the recommended nested runner/container shape.
4. Distinguish the writer input from the aggregate target:
   - `PlatformBuildMetadataRecord` includes exact `identity: ReleaseIdentity`;
   - `PlatformBuildTargetMetadata` contains the allowlisted target fields without identity;
   - `BuildMetadata.targets` is `PlatformBuildTargetMetadata[]`.
5. If the publication inventory forces v2, change both literal schema types atomically.

#### `.trellis/spec/backend/github-release-workflow.md`

Update the metadata contract around lines 162-169 and aggregation contract around lines 284-288 to state:

- requested runner label versus documented runner context;
- configured digest-pinned image versus container-observed OS/user-space and `uname -m` facts;
- exact x64/ARM64 mapping and non-container `null` rule;
- removal, not nullability, of `ImageOS`/`ImageVersion`;
- the limitation that `/etc/os-release`, `uname -m`, and artifact attestation do not independently prove the OCI digest;
- exact-key rejection and canonical aggregate construction;
- conditional v1-in-place/v2 migration gate described above.

Do not weaken the existing no-QEMU, native-runner, early verification, exact-five-record, or attestation-subject requirements.

#### Public release documentation

Update both current v0.3.0 documents in the same change:

- `docs/fyagent/dev/v1-0.3.0/06-CI-AND-RELEASE-DESIGN.md:178-181`: replace the generic “runner label/OS/arch/Linux child digest” wording with the requested/configured/observed distinction and the measured `ubuntu`/`22.04`/`uname -m` values.
- `docs/fyagent/dev/v1-0.3.0/implementation-map/RELEASE-ASSET-CONTRACT.md:91-104`: document the exact nested target shape, null rule, exact-key rejection, and final schema identifier selected after the publication-inventory gate.
- If `SOURCE-REGISTER.md` is the project's canonical source register, add the official runner context/default variables/job-container, OCI descriptor, and Linux `os-release`/`uname` references listed below. Do not cite implementation source as if it were a supported GitHub API.

### Test strategy

#### New direct writer behavior suite

Add `tests/writePlatformMetadata.test.ts`. Invoke the real CLI with `spawnSync(process.execPath, [writerPath, outputPath], { env })`; do not reproduce writer logic in a fixture helper.

Required successful cases:

1. Linux x64 with the exact label/context/image/digest/OS-release/uname/toolchain/identity inputs writes the exact recommended object.
2. Linux ARM64 writes the ARM64 digest and `aarch64` observation.
3. Windows and macOS write exactly `container: null` and no retired fields.
4. Preflight and formal identity modes preserve the current Required-CI binding rules.
5. An existing output path fails and remains unchanged, proving `wx` behavior.

Required failure table (use table-driven tests and assert nonzero exit plus the relevant stderr field name/reason):

- missing and whitespace-only `ACTUAL_RUNNER_OS` or `ACTUAL_RUNNER_ARCH`;
- missing each Linux configured/observed container input one at a time;
- only some container inputs present;
- malformed uppercase/short/non-SHA-256 digest;
- image-reference suffix inconsistent with the digest;
- x64 target paired with ARM64 runner context, ARM child digest, or `aarch64` uname;
- ARM64 target paired with X64 context, amd64 child digest, or `x86_64` uname;
- Linux OS ID/version drift;
- container inputs on Windows/macOS;
- existing formal/preflight Required-CI mismatch cases.

Ambient poison case:

- provide contradictory `RUNNER_OS`, `RUNNER_ARCH`, `ImageOS`, and `ImageVersion` while the owned inputs are correct;
- assert the command succeeds, the output follows only owned inputs, and recursively contains none of `imageOs`, `imageVersion`, `ImageOS`, or `ImageVersion`.

This direct suite is the missing regression test for the original failure. The current static source-string assertions cannot prove process-environment behavior.

#### Update `tests/releaseAssets.test.ts`

1. Update the five fixture records to the structured runner/container shape and exact observed Linux values.
2. Assert exact root and nested key sets in the successful aggregate.
3. Add table-driven rejection for:
   - wrong runner OS/arch;
   - null/partial/extra Linux container data;
   - non-null native-platform container data;
   - image reference/digest mismatch;
   - OS ID/version or uname drift;
   - retired `imageOs`/`imageVersion` and an arbitrary unknown key at each relevant level.
4. Assert the generated aggregate contains only canonical allowlisted fields, not input `identity` or unknown input fields.
5. Preserve exact-five-record, identity, toolchain, and Required-CI tests.

#### Update `tests/releaseWorkflow.test.ts`

1. Preserve the three exact `${{ runner.os }}` / `${{ runner.arch }}` mappings and absence of retired hosted-image variables.
2. Assert all three writers pass `REQUESTED_RUNNER_LABEL` and only Linux passes configured/observed container inputs.
3. Assert the Linux metadata step has explicit Bash fail-closed behavior, sources `/etc/os-release`, measures `uname -m` immediately, verifies both facts and runner context, then invokes the writer.
4. Assert `container.image` and `CONTAINER_IMAGE_REFERENCE` are formed from the same fully qualified matrix image and exact digest.
5. Preserve both child digests, host labels, native arch mappings, and no-QEMU/no-privileged-container assertions.

#### Host-native validation after implementation

Run at minimum:

```text
pnpm test:unit tests/writePlatformMetadata.test.ts tests/releaseAssets.test.ts tests/releaseWorkflow.test.ts
pnpm typecheck
pnpm exec prettier --check .github/workflows/release.yml scripts/release/write-platform-metadata.mjs scripts/release/release-contract.mjs scripts/release/release-contract.d.mts tests/writePlatformMetadata.test.ts tests/releaseAssets.test.ts tests/releaseWorkflow.test.ts .trellis/spec/backend/github-release-workflow.md docs/fyagent/dev/v1-0.3.0/06-CI-AND-RELEASE-DESIGN.md docs/fyagent/dev/v1-0.3.0/implementation-map/RELEASE-ASSET-CONTRACT.md
```

Then run the repository's existing release-contract gate and `actionlint` through its canonical host-native task. A successful native x64/ARM64 job-container preflight remains mandatory remote evidence: static/direct writer tests prove the contract behavior but cannot prove GitHub's two actual runners can pull the retained child digests and complete packaging.

### External references

All web references below were checked on 2026-08-08. GitHub repository `main` links are implementation snapshots, not supported public APIs.

1. [GitHub Docs — runner context](https://docs.github.com/en/actions/reference/workflows-and-actions/contexts#runner-context): documents runner `os`, `arch`, and `environment` and their enumerated values.
2. [GitHub Docs — default variables](https://docs.github.com/en/actions/reference/workflows-and-actions/variables): documents `RUNNER_OS`, `RUNNER_ARCH`, and protected `GITHUB_*`/`RUNNER_*` defaults; does not list `ImageOS` or `ImageVersion`.
3. [GitHub Docs — choose a runner](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job): defines `runs-on` as runner selection by label/group and lists the explicit hosted-runner labels.
4. [GitHub Docs — run jobs in a container](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/run-jobs-in-a-container): defines `jobs.<job_id>.container.image` and states that shell steps run in the job container.
5. [GitHub `actions/runner-images` README](https://github.com/actions/runner-images): authoritative hosted-image lifecycle/source; describes recurring image updates and image-version visibility in the setup log.
6. [GitHub runner-images Ubuntu environment provisioning source](https://raw.githubusercontent.com/actions/runner-images/refs/heads/main/images/ubuntu/scripts/build/configure-environment.sh): implementation source that writes `ImageVersion` and `ImageOS` to the VM's `/etc/environment`.
7. [GitHub Actions runner v2.312.0 release](https://github.com/actions/runner/releases/tag/v2.312.0) and [runner PR #2878](https://github.com/actions/runner/pull/2878): implementation history for exposing `ImageOS`; evidence of implementation provenance, not a portable workflow-variable contract.
8. [GitHub runner `ContainerInfo.cs`](https://raw.githubusercontent.com/actions/runner/main/src/Runner.Worker/Container/ContainerInfo.cs), [`ContainerOperationProvider.cs`](https://raw.githubusercontent.com/actions/runner/main/src/Runner.Worker/ContainerOperationProvider.cs), and [`StepHost.cs`](https://raw.githubusercontent.com/actions/runner/main/src/Runner.Worker/Handlers/StepHost.cs): authoritative implementation snapshots for container creation and step execution/environment assembly.
9. [Docker CLI — pull an image by digest](https://docs.docker.com/reference/cli/docker/image/pull/): a digest-pinned reference selects immutable image content; it must be deliberately updated to receive later image/security changes.
10. [OCI Image Spec — descriptor](https://github.com/opencontainers/image-spec/blob/main/descriptor.md) and [image configuration](https://github.com/opencontainers/image-spec/blob/main/config.md): define content digests and image `os`/`architecture` semantics.
11. [Linux `os-release(5)`](https://man7.org/linux/man-pages/man5/os-release.5.html): defines `/etc/os-release`, machine-readable `ID`, and `VERSION_ID` for identifying the current user space.
12. [Linux `uname(2)`](https://man7.org/linux/man-pages/man2/uname.2.html): defines the `machine` result used by `uname -m`; it is kernel/UTS machine evidence, not distribution or OCI-digest evidence.
13. [GitHub Docs — workflow artifact attestations](https://docs.github.com/en/actions/concepts/workflows-and-actions/workflow-artifacts#artifact-attestations) and [official `actions/attest` README](https://github.com/actions/attest): explain subject-digest/workflow provenance binding and the default SLSA provenance mode.

### Related specs and task decisions

- `.trellis/spec/backend/github-release-workflow.md:133-169` — native runner matrix, Ubuntu 22.04 child digests, no-QEMU checks, and current metadata wording.
- `.trellis/spec/backend/github-release-workflow.md:284-294` — exact-five aggregation and exact attestation subjects.
- `.trellis/tasks/08-07-modernize-ci-and-release/design.md` — Ubuntu 24.04 host plus same-architecture Ubuntu 22.04 digest-pinned user-space boundary.
- `.trellis/tasks/08-07-modernize-ci-and-release/implement.md` — exact platform metadata and attestation delivery requirement.
- `.trellis/tasks/08-07-modernize-ci-and-release/research/ci-release-local-evidence.md:65-70,75-100` — failed preflights did not aggregate/publish and formal Release remains gated.
- `.trellis/workflow.md` — host-native-only local validation and remote-native Actions evidence boundaries.

## Caveats / Not Found

1. Per dispatch, no GitHub Actions run, artifact, log, check, tag, or Release was queried or triggered. The v1-in-place recommendation relies on local task evidence that no aggregate/attestation/publication completed; it must be reversed to the v2 path if a later read-only inventory finds an external v1 consumer or published v1 record.
2. The exact current Docker Hub tag-to-manifest mapping and continued remote availability of the two already-reviewed child digests were not queried. A digest pin is intentionally independent of the moving `22.04` tag, but the next real native preflight must still prove both digests can be pulled on the corresponding runners. Digest refresh is a separate reviewed supply-chain change and must update workflow, expected target map, specs, and tests atomically.
3. `/etc/os-release` and `uname -m` are runtime observations, not cryptographic proof of a root filesystem or native hardware. Together with the GitHub-hosted runner context, no-QEMU contract, per-architecture digest pin, and package architecture checks, they provide the truthful evidence available to this workflow without inventing a stronger claim.
4. Standard artifact attestation authenticates subject bytes and workflow provenance under GitHub's trust model. It does not protect against a trusted workflow deliberately writing false custom values; fail-closed writer/validator review and branch/release controls remain part of the trust boundary.
5. GitHub runner/runner-images implementation links track `main` and can drift. They are used only to explain the observed absence and classify `ImageOS`/`ImageVersion` as implementation details. The recommended contract depends on public GitHub context/container documentation, OCI digest semantics, and values measured by the workflow—not on those internal source paths.
6. No implementation, spec, test, workflow, or public document was modified, and no test was executed in this research-only subtask. The prescribed commands and the native Actions preflight remain verification work for the implement/check phases.
