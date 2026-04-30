#!/usr/bin/env node

/**
 * ctx-lite npx wrapper
 * 
 * This script serves as the entry point for npx @spahmonk/ctx-lite
 * It detects the platform, ensures the binary is available (downloading if needed),
 * and executes it with the provided arguments.
 */

import { execFileSync, spawnSync } from 'child_process';
import { existsSync, mkdirSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';
import { platform, arch } from 'os';

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
  return platformArch.includes('windows') ? 'ctx-lite.exe' : 'ctx-lite';
}

function getCacheDir() {
  return join(homedir(), '.ctx-lite-cache');
}

function getBinaryPath(platformArch) {
  const cacheDir = getCacheDir();
  const binaryName = getBinaryName(platformArch);
  return join(cacheDir, binaryName);
}

function binaryExists(binaryPath) {
  try {
    return existsSync(binaryPath);
  } catch {
    return false;
  }
}

async function downloadBinary(platformArch) {
  const version = '1.0.0';
  const repo = 'spahmonk/ai-helpers';
  const downloadUrl = `https://github.com/${repo}/releases/download/v${version}/ctx-lite-${version}-${platformArch}.tar.gz`;
  const binaryPath = getBinaryPath(platformArch);
  const cacheDir = getCacheDir();

  console.log(`⬇️  Downloading ctx-lite for ${platformArch}...`);

  if (!existsSync(cacheDir)) {
    mkdirSync(cacheDir, { recursive: true });
  }

  try {
    const tempFile = join(cacheDir, 'ctx-lite.tar.gz');
    execFileSync('curl', ['-fsSL', downloadUrl, '-o', tempFile], { stdio: 'inherit' });
    execFileSync('tar', ['-xzf', tempFile, '-C', cacheDir], { stdio: 'inherit' });
    execFileSync('rm', [tempFile], { stdio: 'inherit' });

    if (!platformArch.includes('windows')) {
      execFileSync('chmod', ['+x', binaryPath], { stdio: 'inherit' });
    }

    console.log(`✓ Downloaded and cached at ${binaryPath}`);
  } catch (error) {
    console.error(`✗ Failed to download binary from ${downloadUrl}`);
    console.error(`Please ensure version ${version} is released on GitHub.`);
    process.exit(1);
  }
}

async function main() {
  try {
    const platformArch = getPlatformArchitecture();
    const binaryPath = getBinaryPath(platformArch);

    if (!binaryExists(binaryPath)) {
      await downloadBinary(platformArch);
    }

    const args = process.argv.slice(2);
    const result = spawnSync(binaryPath, args, {
      stdio: 'inherit',
      shell: false
    });

    process.exit(result.status ?? 0);
  } catch (error) {
    console.error(`✗ Error: ${error.message}`);
    process.exit(1);
  }
}

main();
