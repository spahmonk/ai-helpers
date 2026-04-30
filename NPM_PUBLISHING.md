# Publishing to npm Registry

## Overview

The `ctx-lite` package is published to [npmjs.com](https://www.npmjs.com/package/@spahmonk/ctx-lite) via GitHub Actions CI/CD pipeline.

When a new GitHub Release is created, the npm package is automatically published.

## Setup (One-Time)

### Step 1: Create npm Account
1. Go to [npmjs.com](https://www.npmjs.com)
2. Sign up for a free account
3. Confirm your email

### Step 2: Generate npm Access Token
1. Login to npmjs.com
2. Go to **Account Settings** → **Access Tokens**
3. Click **Generate New Token**
4. Select **Granular Access Token**
5. Configure permissions:
   - **Organization/Package**: Select "ctx-lite" or specific package
   - **Permissions**: Check "Publish" and "Read"
6. Copy the generated token (starts with `npm_`)

### Step 3: Add Token to GitHub Secrets
1. Go to your GitHub repository
2. Navigate to **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Name: `NPM_TOKEN`
5. Value: Paste the npm token from Step 2
6. Click **Add secret**

## Publishing

### Automatic (Recommended)
1. Create a new GitHub Release:
   ```bash
   git tag v1.0.1
   git push origin v1.0.1
   ```
   Or use GitHub UI: **Releases** → **Draft a new release**

2. The `publish-npm.yml` workflow automatically:
   - Waits for release to be published
   - Updates `package.json` version
   - Publishes to npm registry
   - Adds comment to release with npm install command

### Manual Publishing (If Needed)
```bash
# Install package locally
npm install

# Authenticate to npm
npm login

# Publish
npm publish

# Or with specific token
npm config set //registry.npmjs.org/:_authToken=$NPM_TOKEN
npm publish
```

## Verification

After publishing, verify the package:

```bash
# Check on npmjs.com
https://www.npmjs.com/package/@spahmonk/ctx-lite

# Install locally
npm install -g @spahmonk/ctx-lite

# Or via npx
npx @spahmonk/ctx-lite --help
```

## Troubleshooting

### "npm ERR! 404 Not Found"
- Package name mismatch (check `name` in package.json)
- Not published yet (check GitHub Actions workflow)

### "npm ERR! need auth"
- NPM_TOKEN not set in GitHub Secrets
- Token has expired (regenerate on npmjs.com)
- Token doesn't have publish permission

### "npm ERR! version not updated"
- Update `version` in package.json manually
- The workflow tries to update it automatically from git tag

## Package Structure

```
/
├── package.json           # npm metadata
├── bin/
│   ├── index.js          # CLI wrapper
│   └── download-binary.js # postinstall hook
├── .npmignore            # Files to exclude from npm
└── modules/ctx-lite/     # Source code (not included in npm)
```

The npm package downloads pre-built binaries during `npm install`, making installation instant.

## CI/CD Pipeline

The complete pipeline:

1. **Git Push/Release** → Triggers GitHub Actions
2. **Build Job** → Builds binaries for all platforms
3. **Create Release Job** → Creates GitHub Release with binary artifacts
4. **Publish npm Job** → Publishes to npm (automatic on release published)
5. **npm Registry** → Package is live!

```
git tag v1.0.1
    ↓
GitHub Actions: build
    ↓
GitHub Actions: create-release
    ↓
GitHub Release Published
    ↓
GitHub Actions: publish-npm (triggers on release published)
    ↓
npm registry
    ↓
✅ Users can npm install @spahmonk/ctx-lite
```

## See Also

- [package.json](../package.json) - npm configuration
- [bin/index.js](../bin/index.js) - CLI wrapper implementation
- [.github/workflows/publish-npm.yml](../workflows/publish-npm.yml) - npm publishing workflow
- [.github/workflows/release.yml](../workflows/release.yml) - binary build workflow
