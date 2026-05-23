#!/usr/bin/env node

/**
 * mem-lite binary download script (postinstall hook)
 * Pre-downloads the binary for the current platform to speed up first run.
 */

import { execFileSync } from 'child_process';
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
      return architecture === 'x64' ? 'x86_64-unknown-linux-gnu' : null;
    case 'darwin':
      if (architecture === 'x64') return 'x86_64-apple-darwin';
      if (architecture === 'arm64') return 'aarch64-apple-darwin';
      return null;
    case 'win32':
      return architecture === 'x64' ? 'x86_64-pc-windows-msvc' : null;
    default:
      return null;
  }
}

const platformArch = getPlatformArchitecture();
if (!platformArch) {
  console.log('mem-lite: unsupported platform, binary will be downloaded on first use.');
  process.exit(0);
}

const cacheDir = getCacheDirectory(homedir());
const binaryName = platformArch.includes('windows') ? 'mem-lite.exe' : 'mem-lite';
const binaryPath = join(cacheDir, binaryName);

if (existsSync(binaryPath)) {
  console.log(`mem-lite: binary already cached at ${binaryPath}`);
  process.exit(0);
}

mkdirSync(cacheDir, { recursive: true });

const url = buildDownloadUrl(platformArch);
const archiveName = getTempArchiveName(platformArch);
const archivePath = join(cacheDir, archiveName);

try {
  console.log(`mem-lite: downloading ${url}`);
  execFileSync('curl', ['-fsSL', '-o', archivePath, url], { stdio: 'inherit' });

  const { file, args } = getExtractionCommand(platformArch, archivePath, cacheDir);
  execFileSync(file, args, { stdio: 'inherit' });

  rmSync(archivePath, { force: true });
  console.log(`mem-lite: binary ready at ${binaryPath}`);
} catch (err) {
  console.log(`mem-lite: pre-download failed (${err.message}), will retry on first use.`);
  rmSync(archivePath, { force: true });
}
