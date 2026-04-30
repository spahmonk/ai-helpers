#!/usr/bin/env node

/**
 * ctx-lite binary download script
 * 
 * This script is called during npm install (postinstall hook)
 * to pre-download the binary for the current platform.
 * This makes the first run of npx much faster.
 */

import { execSync } from 'child_process';
import { existsSync, mkdirSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';
import { platform, arch } from 'os';

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

function getBinaryName(platformArch) {
  return platformArch.includes('windows') ? 'ctx-lite.exe' : 'ctx-lite';
}

function getCacheDir() {
  return join(homedir(), '.ctx-lite-cache');
}

function getBinaryPath(platformArch) {
  return join(getCacheDir(), getBinaryName(platformArch));
}

function downloadBinary(platformArch) {
  const version = '1.0.0';
  const repo = 'spahmonk/ai-helpers';
  const downloadUrl = `https://github.com/${repo}/releases/download/v${version}/ctx-lite-${version}-${platformArch}.tar.gz`;
  const binaryPath = getBinaryPath(platformArch);
  const cacheDir = getCacheDir();

  if (!existsSync(cacheDir)) {
    mkdirSync(cacheDir, { recursive: true });
  }

  if (existsSync(binaryPath)) {
    console.log(`✓ ctx-lite already cached at ${cacheDir}`);
    return;
  }

  try {
    console.log(`ℹ️  Pre-downloading ctx-lite for faster first run...`);
    const tempFile = join(cacheDir, 'ctx-lite.tar.gz');
    execSync(`curl -fsSL "${downloadUrl}" -o "${tempFile}"`, { stdio: 'pipe' });
    execSync(`tar -xzf "${tempFile}" -C "${cacheDir}"`, { stdio: 'pipe' });
    execSync(`rm "${tempFile}"`, { stdio: 'pipe' });

    if (!platformArch.includes('windows')) {
      execSync(`chmod +x "${binaryPath}"`, { stdio: 'pipe' });
    }

    console.log(`✓ ctx-lite cached successfully`);
  } catch (error) {
    console.warn(`⚠️  Could not pre-download binary (will download on first use)`);
    // Don't fail installation if download fails
  }
}

// Main
const platformArch = getPlatformArchitecture();
if (platformArch) {
  downloadBinary(platformArch);
} else {
  console.warn(`⚠️  Unsupported platform or architecture (will download on first use)`);
}
