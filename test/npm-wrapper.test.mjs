import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  buildDownloadUrl,
  getCacheDirectory,
  getArchiveExtension,
  getExtractionCommand,
  getPackageVersion,
} from '../bin/release-assets.js';

const packageJson = JSON.parse(
  readFileSync(new URL('../package.json', import.meta.url), 'utf8'),
);

test('download URL uses the current package.json version', () => {
  const version = getPackageVersion();
  const url = buildDownloadUrl('x86_64-unknown-linux-gnu');

  assert.equal(version, packageJson.version);
  assert.equal(
    url,
    `https://github.com/spahmonk/ai-helpers/releases/download/v${packageJson.version}/ctx-lite-${packageJson.version}-x86_64-unknown-linux-gnu.tar.gz`,
  );
});

test('windows assets use zip archives', () => {
  assert.equal(getArchiveExtension('x86_64-pc-windows-msvc'), 'zip');
  assert.equal(
    buildDownloadUrl('x86_64-pc-windows-msvc'),
    `https://github.com/spahmonk/ai-helpers/releases/download/v${packageJson.version}/ctx-lite-${packageJson.version}-x86_64-pc-windows-msvc.zip`,
  );
});

test('windows extraction uses Expand-Archive with double-quoted paths', () => {
  const command = getExtractionCommand(
    'x86_64-pc-windows-msvc',
    'C:\\temp\\ctx-lite.zip',
    'C:\\cache',
  );

  assert.equal(command.file, 'powershell');
  assert.deepEqual(command.args, [
    '-NoProfile',
    '-Command',
    'Expand-Archive -LiteralPath "C:\\temp\\ctx-lite.zip" -DestinationPath "C:\\cache" -Force',
  ]);
});

test('unix extraction keeps tar.gz behavior', () => {
  const command = getExtractionCommand(
    'x86_64-unknown-linux-gnu',
    '/tmp/ctx-lite.tar.gz',
    '/tmp/cache',
  );

  assert.equal(command.file, 'tar');
  assert.deepEqual(command.args, ['-xzf', '/tmp/ctx-lite.tar.gz', '-C', '/tmp/cache']);
});

test('cache directory is versioned to avoid stale binaries across package upgrades', () => {
  const cacheDir = getCacheDirectory('/tmp/home');

  assert.equal(cacheDir, `/tmp/home/.ctx-lite-cache/${packageJson.version}`);
});
