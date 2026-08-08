export type ReleasePlatform = "macos" | "windows" | "linux";
export type ReleaseArchitecture = "universal" | "x64" | "arm64";
export type ReleaseTargetGroup =
  | "macos-universal"
  | "windows-x64"
  | "windows-arm64"
  | "linux-x64"
  | "linux-arm64";

export interface InstallerRule {
  readonly suffix: string;
  readonly platform: ReleasePlatform;
  readonly kind: "dmg" | "zip" | "msi" | "appimage" | "deb" | "rpm";
  readonly architecture: ReleaseArchitecture;
}

export interface ExpectedTarget {
  readonly targetGroup: ReleaseTargetGroup;
  readonly platform: ReleasePlatform;
  readonly architecture: ReleaseArchitecture;
  readonly runnerLabel: string;
  readonly expectedRunnerArch: "X64" | "ARM64" | null;
  readonly containerDigest: string | null;
}

export interface ReleaseIdentity {
  productVersion: string;
  tag: string;
  sourceSha: string;
  repository: string;
  repositoryId: string;
  workflowPath: string;
  workflowRef: string;
  workflowSha: string;
  runId: string;
  runAttempt: string;
  event: string;
  mode: string;
  ciWorkflowPath: string | null;
  ciRunId: string | null;
  ciRunAttempt: string | null;
}

export interface DownloadManifestAsset {
  name: string;
  platform: ReleasePlatform;
  architecture: ReleaseArchitecture;
  format: InstallerRule["kind"];
  sizeBytes: number;
  sha256: string;
  url: string;
}

export interface DownloadManifest {
  schema: "fyagent-download-manifest/v2";
  product: "FyAgent";
  version: string;
  tag: string;
  sourceSha: string;
  publishedAt: string;
  assets: DownloadManifestAsset[];
}

export interface PlatformBuildMetadata {
  schema: "fyagent-platform-build/v1";
  targetGroup: ReleaseTargetGroup;
  platform: ReleasePlatform;
  architecture: ReleaseArchitecture;
  runnerLabel: string;
  runner: {
    runnerOs: string;
    runnerArch: string;
    imageOs: string;
    imageVersion: string;
  };
  containerDigest: string | null;
  toolchain: {
    node: string;
    pnpm: string;
    rustc: string;
  };
}

export interface BuildMetadata {
  schema: "fyagent-build-metadata/v1";
  product: "FyAgent";
  version: string;
  tag: string;
  sourceSha: string;
  repository: {
    nameWithOwner: string;
    id: string;
  };
  workflow: {
    path: string;
    ref: string;
    sha: string;
    runId: string;
    runAttempt: string;
    event: string;
    mode: string;
  };
  requiredCi: {
    path: string;
    runId: string;
    runAttempt: string;
    job: "CI / Required";
    conclusion: "success";
  } | null;
  generatedAt: string;
  targets: PlatformBuildMetadata[];
}

export const PRODUCT_NAME: "FyAgent";
export const FORMAL_VERSION: "0.3.0";
export const FORMAL_TAG: "v0.3.0";
export const EXPECTED_REPOSITORY: "NongHua123/fyagent";
export const EXPECTED_REPOSITORY_ID: "1313497021";
export const RELEASE_WORKFLOW_PATH: ".github/workflows/release.yml";
export const CI_WORKFLOW_PATH: ".github/workflows/ci.yml";
export const DOWNLOAD_MANIFEST_NAME: "download-manifest.json";
export const BUILD_METADATA_NAME: "build-metadata.json";
export const ATTESTATION_BUNDLE_NAME: "artifact-attestation.sigstore.json";

export const INSTALLER_RULES: readonly InstallerRule[];
export const EXPECTED_TARGETS: readonly ExpectedTarget[];
export const EXPECTED_INSTALLERS_BY_TARGET: Readonly<
  Record<ReleaseTargetGroup, readonly number[]>
>;

export function assertReleaseIdentity(identity: {
  version: string;
  tag: string;
  sourceSha: string;
}): void;
export function expectedInstallerNames(version: string): string[];
export function expectedAttestationSubjectNames(version: string): string[];
export function expectedReleaseAttachmentNames(version: string): string[];
export function assertExactFileSet(
  directory: string,
  expectedNames: readonly string[],
  label: string,
): string[];
export function assertExactDirectorySet(
  directory: string,
  expectedNames: readonly string[],
  label: string,
): void;
export function assertExactInstallerSet(
  directory: string,
  version: string,
): string[];
export function sha256File(filePath: string): Promise<string>;
export function buildDownloadManifest(input: {
  assetsDirectory: string;
  version: string;
  tag: string;
  sourceSha: string;
  baseUrl: string;
  publishedAt: string;
}): Promise<DownloadManifest>;
export function buildBuildMetadata(input: {
  metadataDirectory: string;
  identity: ReleaseIdentity;
  generatedAt: string;
}): BuildMetadata;
