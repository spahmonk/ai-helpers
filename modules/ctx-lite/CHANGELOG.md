# Changelog

## [1.1.0](https://github.com/spahmonk/ai-helpers/compare/ctx-lite-v1.0.9...ctx-lite-v1.1.0) (2026-05-25)


### Features

* add comprehensive integration test suite for ctx-lite ([c870b0f](https://github.com/spahmonk/ai-helpers/commit/c870b0f22a1444970b26a8a8ad9523deef0b6258))
* Add pre-compression and format optimization (opt-3.3) ([42a9e52](https://github.com/spahmonk/ai-helpers/commit/42a9e5229570446566dae699be63a805b9e70001))
* add smoke tests and release verification ([296b551](https://github.com/spahmonk/ai-helpers/commit/296b551c4e0e22566b0156e249529284024fcfea))
* **ctx-lite:** capability-policy runtime surfaces ([a3838ac](https://github.com/spahmonk/ai-helpers/commit/a3838ac510b226616a2ba88178617a78df504288))
* implement adaptive mode selection policy (opt-2.2) ([d11b0f9](https://github.com/spahmonk/ai-helpers/commit/d11b0f9852b823ced139b9d21d212c5d2c78792c))
* implement CLI adapter for ctx-lite ([266ef10](https://github.com/spahmonk/ai-helpers/commit/266ef1023a845d82ec524bb59bf9787882721155))
* Implement Diff Mode for incremental file compression (opt-3.1) ([fffa649](https://github.com/spahmonk/ai-helpers/commit/fffa6490f2649016ea74caf5f1e4e9e69476e89d))
* implement Doctor Service for configuration validation ([6da8bcd](https://github.com/spahmonk/ai-helpers/commit/6da8bcd94837d0a3acdcb478b11582829ba0f5f0))
* implement MCP adapter with 21 comprehensive tests ([70d6aaa](https://github.com/spahmonk/ai-helpers/commit/70d6aaa0a9ca8605df21b6804c0ab018070a9b9e))
* Implement ML-based mode selection (opt-3.2) ([329da32](https://github.com/spahmonk/ai-helpers/commit/329da32f60ab5c275d2d11a9ddf2432d11805389))
* implement Search Service for file content search ([6d959a4](https://github.com/spahmonk/ai-helpers/commit/6d959a4bdaa9e93e1178a1390df3752fb99c64be))
* MCP server integration + automatic setup + npm support ([20b7e37](https://github.com/spahmonk/ai-helpers/commit/20b7e37471880d91816831609ee405565d2cd37f))
* **setup-mcp:** add capability policy args to generated configs ([68e5801](https://github.com/spahmonk/ai-helpers/commit/68e5801f4f96a71f8de67f4d09a99c8263dc8b29))


### Bug Fixes

* allow Windows-specific environment variables in shell tests ([59370dd](https://github.com/spahmonk/ai-helpers/commit/59370dd09b2df267e3091e9c8a384efb6cbfaefa))
* bump version to 1.0.6, show 'No results found' on empty search ([1bf17cb](https://github.com/spahmonk/ai-helpers/commit/1bf17cb8d61c4e37583ca8845e9b988ac605ffdf))
* canonicalize fixture root after directory creation ([d9ddb59](https://github.com/spahmonk/ai-helpers/commit/d9ddb59c75d43de6494c93b9807cdc86389c5d61))
* canonicalize fixture root path to resolve Windows path_jail test failures ([caafede](https://github.com/spahmonk/ai-helpers/commit/caafedec66a22dff245f28d7ad4ee9f0b080ccb1))
* canonicalize temp dir in search tests (macOS /tmp symlink) ([80c4a9e](https://github.com/spahmonk/ai-helpers/commit/80c4a9e1feb6b4f838234bf7c5ca2876795fbff5))
* canonicalize test fixture paths in path_jail integration tests ([d881260](https://github.com/spahmonk/ai-helpers/commit/d8812605ee6700319c9d5496485d53ff7c433889))
* correct cache API usage in integration tests and budget threshold logic ([c37f939](https://github.com/spahmonk/ai-helpers/commit/c37f939a538bb7b84549e837611823fb368e57e9))
* cross-platform installer hardening (v1.0.7) ([#9](https://github.com/spahmonk/ai-helpers/issues/9)) ([50ba1bc](https://github.com/spahmonk/ai-helpers/commit/50ba1bc1d6b369296e7b16ea3849b692f814f4e9))
* detect directory in open_file() before attempting to open ([fdd2027](https://github.com/spahmonk/ai-helpers/commit/fdd2027ef05d375cbbfe9fc78391b37c5f568e2a))
* format ctx-lite sources for CI ([4d8ffe9](https://github.com/spahmonk/ai-helpers/commit/4d8ffe9b6d19f7978e676dd9b3a9cabdfa776194))
* handle mcp errors without terminating session ([3e785b0](https://github.com/spahmonk/ai-helpers/commit/3e785b00231b9b019a20e220de2151764a3898e3))
* harden MCP parse errors ([cc90c63](https://github.com/spahmonk/ai-helpers/commit/cc90c637ad6051e5a808453bcedde63010243900))
* improve all user-facing error messages for clarity ([b80931e](https://github.com/spahmonk/ai-helpers/commit/b80931ead8be22bc82ad319eff2d620901afde8e))
* **path_jail:** strip \\?\  from incoming paths in resolve() on Windows ([f941064](https://github.com/spahmonk/ai-helpers/commit/f94106481dcb03648e91727230ab77d091025606))
* **path-jail:** allow absolute paths on Windows ([e3d03e7](https://github.com/spahmonk/ai-helpers/commit/e3d03e7b6432452216d5af6e733ec7a7808c7fde))
* preserve explicit safe profile in setup args ([037108f](https://github.com/spahmonk/ai-helpers/commit/037108f8fb72727757e1bdd8f874b02a827dacfd))
* propagate doctor severity and harden mcp tests ([bfab426](https://github.com/spahmonk/ai-helpers/commit/bfab42633011a38e1ec28e3820bfad63a30324ff))
* remove extra blank lines formatting in path_jail tests ([8d16b7a](https://github.com/spahmonk/ai-helpers/commit/8d16b7a1c2ae510d339a0ab00c08262b80488573))
* resolve GitHub Actions CI failures ([fdb003a](https://github.com/spahmonk/ai-helpers/commit/fdb003ae6885434e49ea4dd9f61413e1b15b4847))
* resolve Windows path handling in shell executor tests ([0d29f42](https://github.com/spahmonk/ai-helpers/commit/0d29f421eb654f76b7bbfe6293e8e5c44843cb61))
* search path scoping, installer docs, QUICK_START English translation ([41801a4](https://github.com/spahmonk/ai-helpers/commit/41801a40514803393d40932410ac0c9af1f3578f))
* **tests:** strip \\?\ UNC prefix in integration tests on Windows ([9e8d95d](https://github.com/spahmonk/ai-helpers/commit/9e8d95dc9c6bcb26427db249411a84e41d096a99))
* **test:** strip \\?\ prefix from root path in search containment test ([a07795c](https://github.com/spahmonk/ai-helpers/commit/a07795cb4d0866bc26169d3c85522eca88b4330c))
* **ux:** improve path-outside-root error with resolved path and allowed roots ([049a711](https://github.com/spahmonk/ai-helpers/commit/049a71113c33bf8340da52b8e03c6192c21d9a07))
* version consistency, auto-detect installer, add aarch64-linux release target ([c95a21b](https://github.com/spahmonk/ai-helpers/commit/c95a21bf7236147d88aaa9dd7ad2949376b2b304))
