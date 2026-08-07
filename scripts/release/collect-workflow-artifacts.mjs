#!/usr/bin/env node

import { constants, copyFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import {
  EXPECTED_INSTALLERS_BY_TARGET,
  EXPECTED_TARGETS,
  assertExactDirectorySet,
  assertExactFileSet,
  expectedInstallerNames,
} from "./release-contract.mjs";

const [mode, inputRoot, outputDirectory, version] = process.argv.slice(2);
if (!mode || !inputRoot || !outputDirectory || !version) {
  console.error(
    "Usage: node scripts/release/collect-workflow-artifacts.mjs <installers|metadata> <download-root> <output-dir> <version>",
  );
  process.exit(1);
}

try {
  if (!(mode === "installers" || mode === "metadata")) {
    throw new Error(`Unsupported artifact collection mode: ${mode}`);
  }
  const installerNames = expectedInstallerNames(version);
  const targetGroups = EXPECTED_TARGETS.map(({ targetGroup }) => targetGroup);
  const artifactNames = targetGroups.map(
    (targetGroup) => `${mode}-${targetGroup}`,
  );
  assertExactDirectorySet(
    inputRoot,
    artifactNames,
    `${mode} artifact download root`,
  );
  mkdirSync(outputDirectory);

  for (const targetGroup of targetGroups) {
    const artifactDirectory = join(inputRoot, `${mode}-${targetGroup}`);
    const expected =
      mode === "installers"
        ? EXPECTED_INSTALLERS_BY_TARGET[targetGroup].map(
            (index) => installerNames[index],
          )
        : [`${targetGroup}.json`];
    assertExactFileSet(
      artifactDirectory,
      expected,
      `${mode}-${targetGroup} artifact`,
    );
    for (const name of expected) {
      copyFileSync(
        join(artifactDirectory, name),
        join(outputDirectory, name),
        constants.COPYFILE_EXCL,
      );
    }
  }

  const expectedOutput =
    mode === "installers"
      ? installerNames
      : targetGroups.map((targetGroup) => `${targetGroup}.json`);
  assertExactFileSet(
    outputDirectory,
    expectedOutput,
    `${mode} collected output`,
  );
  console.log(
    `Collected exactly ${expectedOutput.length} ${mode} files without overwrite`,
  );
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
