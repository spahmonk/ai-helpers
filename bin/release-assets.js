import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const REPO = 'spahmonk/ai-helpers';

export function getPackageVersion() {
  const packageJson = JSON.parse(
    readFileSync(new URL('../package.json', import.meta.url), 'utf8'),
  );
  return packageJson.version;
}

export function getArchiveExtension(platformArch) {
  return platformArch.includes('windows') ? 'zip' : 'tar.gz';
}

export function getCacheDirectory(homeDirectory) {
  return join(homeDirectory, '.ctx-lite-cache', getPackageVersion());
}

export function buildDownloadUrl(platformArch) {
  const version = getPackageVersion();
  const extension = getArchiveExtension(platformArch);

  return `https://github.com/${REPO}/releases/download/v${version}/ctx-lite-${version}-${platformArch}.${extension}`;
}

export function getTempArchiveName(platformArch) {
  return `ctx-lite.${getArchiveExtension(platformArch)}`;
}

export function getExtractionCommand(platformArch, archivePath, destinationDir) {
  if (platformArch.includes('windows')) {
    return {
      file: 'powershell',
      args: [
        '-NoProfile',
        '-Command',
        `Expand-Archive -LiteralPath '${archivePath}' -DestinationPath '${destinationDir}' -Force`,
      ],
    };
  }

  return {
    file: 'tar',
    args: ['-xzf', archivePath, '-C', destinationDir],
  };
}
