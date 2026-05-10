#!/usr/bin/env node

/**
 * mem-lite npx wrapper
 *
 * Entry point for `npx @spahmonk/mem-lite` and `mem-lite` global install.
 * Detects platform, ensures binary is available (downloads if missing), and
 * executes it with the provided arguments.
 */

import { execFileSync, spawnSync } from 'child_process';
import { existsSync, mkdirSync, rmSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';
import { platform, arch } from 'os';
import {
  buildDownloadUrl,
  getCacheDirectory,
  getExtractionCommand,
  getTempArchiveName,
} from './release-assets.js';

function getPlatformArchitecture() {
  const plat = platform();
  const architecture = arch();
  switch (plat) {
    case 'linux':
      if (architecture === 'x64') return 'x86_64-unknown-linux-gnu';
      if (architecture === 'arm64') return 'aarch64-unknown-linux-gnu';
      throw new Error(`Unsupported Linux architecture: ${architecture}`);
    case 'darwin':
      if (architecture === 'x64') return 'x86_64-apple-darwin';
      if (architecture === 'arm64') return 'aarch64-apple-darwin';
      throw new Error(`Unsupported macOS architecture: ${architecture}`);
    case 'win32':
      if (architecture === 'x64') return 'x86_64-pc-windows-msvc';
      throw new Error(`Unsupported Windows architecture: ${architecture}`);
    default:
      throw new Error(`Unsupported platform: ${plat}`);
  }
}

function getBinaryName(platformArch) {
  return platformArch.includes('windows') ? 'mem-lite.exe' : 'mem-lite';
}

function ensureBinary(platformArch) {
  const cacheDir = getCacheDirectory(homedir());
  const binaryName = getBinaryName(platformArch);
  const binaryPath = join(cacheDir, binaryName);

  if (existsSync(binaryPath)) return binaryPath;

  mkdirSync(cacheDir, { recursive: true });

  const url = buildDownloadUrl(platformArch);
  const archiveName = getTempArchiveName(platformArch);
  const archivePath = join(cacheDir, archiveName);

  process.stderr.write(`mem-lite: downloading binary from ${url}\n`);
  execFileSync('curl', ['-fsSL', '-o', archivePath, url]);

  const { file, args } = getExtractionCommand(platformArch, archivePath, cacheDir);
  execFileSync(file, args);
  rmSync(archivePath, { force: true });

  if (!existsSync(binaryPath)) {
    throw new Error(`Binary not found after extraction: ${binaryPath}`);
  }

  return binaryPath;
}

let platformArch;
try {
  platformArch = getPlatformArchitecture();
} catch (err) {
  process.stderr.write(`Error: ${err.message}\n`);
  process.exit(1);
}

let binaryPath;
try {
  binaryPath = ensureBinary(platformArch);
} catch (err) {
  process.stderr.write(`Error: failed to download mem-lite binary: ${err.message}\n`);
  process.exit(1);
}

const result = spawnSync(binaryPath, process.argv.slice(2), { stdio: 'inherit' });
process.exit(result.status ?? 1);
