# `@localix-ai/agent-plugins/core`

The core Agent Plugins form (Agent Plugins 1.1.0, `core §n`): deciding a plugin root's form, loading a core package, the pure readers of its files, its subprocess configuration and the dual-package check. A session runs core packages through the root entry (`PluginHostOptions.corePackages`, docs/API.md, "Core packages in a session"). Converting between the forms (Extended §18.4) is here too (docs/API.md, "Converting between the forms").

One row per export; `scripts/check-docs.ts` (DOC3) keeps this table equal to the entry's exports.

| Name | Kind | What it is |
| --- | --- | --- |
| `checkCoreSkill` | function | A `SKILL.md` of `skills/<directory>/` by the Agent Skills specification: why it is skipped, and the fields outside it (core §7.1) |
| `checkDualManifests` | function | The two parsed manifests of a dual package: what of the shared fields loads in one form only (Extended §18.3.1) |
| `checkDualPackage` | function | `checkDualManifests` over a folder; `dual: false` and no finding for a root that is not a dual package |
| `claudeCodeToCore` | function | A loaded Claude Code plugin as the files of a core package and the notes of what does not convert (Extended §18.4.3); pure |
| `ClaudeCodeToCoreOptions` | type | `ConvertOptions` and `mods`: the plugin's hooks file declares `modules`, which `Plugin` does not carry |
| `commandProblem` | function | Why a stdio server's or hook handler's `command` is not one bare name or `./` path (core §7.2.1, §7.3.1) |
| `Conversion` | type | What a converter returns: the target form, `stopped`, the files to write, the notes, the adapter's runtime |
| `CONVERSION_OUTCOMES` | constant | The outcomes of a note: `stopped`, `not-converted`, `reported` |
| `ConversionNote` | type | One thing a conversion did not carry over or carried with a difference: outcome, path, field, message |
| `ConversionOutcome` | type | One of `CONVERSION_OUTCOMES` |
| `ConvertedFile` | type | A file to write: its path relative to the plugin root and its text |
| `ConvertOptions` | type | `hookAdapter`: route hook handlers through an adapter (default) or report each as not converted |
| `ConvertPackageInput` | type | The input of `convertPluginPackage`: the files, `ConvertOptions`, a signal |
| `convertPluginPackage` | function | Converts a plugin root into the other form through the files port, the output judged by the target before it is written; never writes, never throws |
| `CORE_DOCUMENTS` | constant | The documents the core defines a schema for: `plugin`, `mcp`, `hooks` |
| `CORE_HOOK_EVENTS` | constant | The five core hook events (core §7.3) |
| `CORE_TRANSPORTS` | constant | The three closed server variants of `mcp.json`: `stdio`, `streamable-http`, `sse` |
| `CORE_VARIABLES` | constant | `PLUGIN_ROOT` and `PLUGIN_DATA` (core §9) |
| `CORE_VERSIONS` | constant | The Agent Plugins versions the kit implements |
| `CoreAuthor` | type | The closed `author` object of a core manifest |
| `CoreDocument` | type | One of `CORE_DOCUMENTS` |
| `coreEnvironment` | function | A core subprocess's environment: the host's base, a server's `env`, then `PLUGIN_ROOT` and `PLUGIN_DATA` by the platform's name rules (core §9.1) |
| `CoreExtensions` | type | The `extensions` data of the namespaces a host implements |
| `coreHandlerProcess` | function | What a host launches for a core hook handler: command, expanded `args`, the plugin root as working directory, the environment |
| `CoreHookEvent` | type | One of `CORE_HOOK_EVENTS` |
| `CoreHookHandler` | type | A closed core hook handler: `command`, `args`, `extensions` |
| `CoreHookPositions` | type | The index in `hooks.json` of each kept handler, by event: `positions.<Event>[i]` is where `hooks.<Event>[i]` sits in the file |
| `CoreHooks` | type | The valid handlers of `hooks.json` by event |
| `CoreHooksResult` | type | What `readCoreHooks` returns: the handlers, their positions in the file, whether hooks are disabled, the issues |
| `CoreLoadOptions` | type | What a host implements: extension namespaces, MCP transports, icon formats besides PNG |
| `CoreManifest` | type | A valid core manifest, root `plugin.json` (core §5) |
| `CoreManifestResult` | type | What `readCoreManifest` returns: the manifest or `undefined`, the version it targets, the issues |
| `CoreMcpOptions` | type | The options of `readCoreMcp`: implemented namespaces and supported transports |
| `CoreMcpResult` | type | What `readCoreMcp` returns: the valid servers, whether MCP is disabled, the issues |
| `CoreMcpServer` | type | A valid server of `mcp.json`, one of the three variants |
| `coreNameProblems` | function | Why a core package name breaks core §5.5, one reason per failed constraint |
| `CorePackage` | type | A loaded core package: manifest, version, skills, servers, hooks, icon candidates, extension directories |
| `CorePaths` | type | The resolved plugin root and data directory a launch expands against |
| `CoreProcess` | type | A core subprocess as a host launches it: command, arguments, working directory, environment |
| `CoreProcessInput` | type | The paths, base environment and platform of a launch |
| `CoreRemoteServer` | type | A `streamable-http` or `sse` server: `url`, `headers`, `extensions` |
| `coreSchemaId` | function | The canonical `$schema` identifier of a document of a release |
| `coreSchemaVersion` | function | The version a `$schema` value names for a document, or `undefined` when it is no canonical identifier |
| `coreServerProcess` | function | What a host launches for a stdio server: `./` command resolved, `args`, `env` and `cwd` expanded, `cwd` contained; or why `cwd` leaves its base |
| `CoreSkillCheck` | type | What `checkCoreSkill` returns |
| `CoreStdioServer` | type | A `stdio` server: `command`, `args`, `env`, `cwd`, `extensions` |
| `coreToClaudeCode` | function | A loaded core package as the files of a Claude Code plugin and the notes of what does not convert (Extended §18.4.2); pure |
| `CoreTransport` | type | One of `CORE_TRANSPORTS` |
| `CoreVariable` | type | One of `CORE_VARIABLES` |
| `CoreVersion` | type | One of `CORE_VERSIONS` |
| `detectPluginForm` | function | The form of a plugin root by the table of Extended §1.3, and whether it is a dual package |
| `expandPlaceholders` | function | One non-recursive pass replacing `${PLUGIN_ROOT}` and `${PLUGIN_DATA}`; anything else stays literal (core §9.2) |
| `extensionsProblem` | function | Why an `extensions` value is not an object of objects (core §8.1) |
| `headersProblem` | function | Why `headers` are not valid HTTP fields, or name one header twice under different casing (core §7.2.1) |
| `iconCandidates` | function | The icon candidates among the entries of `assets/`, PNG first (core §4.3.1) |
| `loadCorePackage` | function | Loads a core package by core §4–§10, each failure on its narrowest boundary; never throws |
| `LoadCorePackageInput` | type | The input of `loadCorePackage`: the files, the host's options, a signal |
| `LoadedCorePackage` | type | What `loadCorePackage` returns: the package or `undefined`, the issues |
| `LoadedPackage` | type | What `loadPluginPackage` returns: the form, whether dual, and what that form's loader returned |
| `loadPluginPackage` | function | The loader of an Extended client: decides the form, then loads by `loadPlugin` or `loadCorePackage` |
| `LoadPluginPackageInput` | type | `LoadPluginInput` and `CoreLoadOptions` |
| `PLUGIN_FORMS` | constant | The two forms of a plugin root: `claude-code`, `core` |
| `PluginForm` | type | One of `PLUGIN_FORMS` |
| `PluginFormDecision` | type | What `detectPluginForm` returns |
| `readCoreHooks` | function | Reads a parsed `hooks.json` against the manifest's version, with the failure boundaries of core §7.3.2 |
| `readCoreManifest` | function | Reads a parsed root `plugin.json` by core §5: unknown fields warned and ignored, every other violation fatal |
| `readCoreMcp` | function | Reads a parsed `mcp.json` against the manifest's version, with the failure boundaries of core §7.2.2 |
| `readCoreServer` | function | One server entry of `mcp.json` as a typed variant, or why it is invalid |
| `resolveCwd` | function | Expands, resolves and contains a stdio server's `cwd`, or says why the entry is invalid (core §7.2.1) |
| `ResolvedCwd` | type | A resolved `cwd`: its base, the path below it, the absolute path |
| `urlProblem` | function | Why a remote server's `url` is not an absolute https URL (http on loopback only) without user information or fragment |
