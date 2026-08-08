#!/usr/bin/env node

import {
  assertExactFileSet,
  expectedAttestationSubjectNames,
  expectedInstallerNames,
  expectedReleaseAttachmentNames,
} from "./release-contract.mjs";

const [mode, directory, version] = process.argv.slice(2);
const expectedByMode = {
  installers: expectedInstallerNames,
  subjects: expectedAttestationSubjectNames,
  attachments: expectedReleaseAttachmentNames,
};

if (!expectedByMode[mode] || !directory || !version) {
  console.error(
    "Usage: node scripts/release/verify-release-files.mjs <installers|subjects|attachments> <directory> <version>",
  );
  process.exit(1);
}

try {
  const expected = expectedByMode[mode](version);
  assertExactFileSet(directory, expected, `${mode} directory`);
  console.log(
    `${mode} directory contains exactly ${expected.length} approved files`,
  );
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
