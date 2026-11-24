# AGENTS.md

Instructions for AI agents contributing to **MkXelz**.

## 1. Project identity and priorities

MkXelz is a procedural, non-AI tile and atlas generator for three-quarter-view games. A versioned JSON specification, named switches, and a seed produce a PNG atlas and metadata JSON. The generator is intended to be standalone and usable independently of any particular game.

Its priorities are:

- Deterministic output: the same generator version, specification, and seed must produce identical pixels and metadata.
- Useful, coherent tile sets with clear terrain, wall, edge, and autotiling behaviour.
- A small platform-independent generation core.
- Separate CLI and GUI front ends that use the same core.
- Android as a first-class target for the GUI, without compromising desktop usability.
- Explicit, versioned output and specification contracts.

The project is a procedural asset tool, not an AI image generator or a game engine. Keep it focused on generating reusable assets and metadata.

## 2. Documentation authority

Before changing code, inspect the relevant implementation, tests, bundled specifications, examples, and documentation.

Use repository documentation according to its responsibility:

- `README.md`: public capabilities, usage, spec reference, output format, platform notes, and known limitations.
- `CHANGELOG.md`: record of completed changes, following the existing format.
- `specs/`: bundled example specifications and presets; treat them as real inputs that must remain valid.
- Tests and fixtures: executable expectations for deterministic generation, schema handling, atlas layout, and output metadata.
- CI workflows: authoritative automated validation and packaging behaviour.

Do not assume an old issue, conversation, or plan overrides the current repository state. When documentation conflicts with implementation or tests, investigate the discrepancy rather than silently choosing one.

## 3. Development workflow

For every development task:

1. Inspect the latest repository state, relevant documentation, tests, bundled specs, and related issues.
2. Understand the current core, CLI, GUI, and platform boundaries before changing them.
3. State a focused implementation plan and identify meaningful trade-offs.
4. Implement the requested change without unnecessarily changing unrelated systems.
5. Add or update automated tests for the behaviour being changed.
6. Validate bundled specifications and presets remain consistent with the implementation.
7. Review all modified files for correctness, determinism, compatibility, and unintended effects.
8. Update relevant documentation and `CHANGELOG.md` when warranted.
9. Prepare a focused branch and pull request.

Prefer small, independently understandable changes and commits. Preserve existing functionality unless the requested change intentionally modifies it.

If a requirement would change the visual direction, specification semantics, output contract, or platform behaviour and the intended choice is unclear, ask the project owner rather than inventing a decision.

## 4. Git and pull request workflow

- Work on dedicated feature or fix branches.
- Prefer pull requests over direct changes to `main`.
- Keep commits focused and descriptive.
- Avoid unrelated formatting changes and opportunistic refactoring.
- Explain the problem, implementation, and relevant design decisions in pull request descriptions.
- Link related issues where appropriate.
- Do not merge pull requests manually.

The project owner has local automation that monitors pull request checks, merges a PR when its checks pass, and reports failures. Do not wait for CI, repeatedly poll checks, or duplicate that automation. Once the implementation and PR are ready, report the PR and hand control back to the owner.

## 5. Build and test responsibilities

Use the repository's CI workflows as the source of truth for builds and automated validation.

- Do not attempt local builds, compilation, or test execution as part of the agent workflow.
- Add or update appropriate automated tests with implementation changes.
- Review test code and coverage for consistency with the implementation.
- Do not weaken or rewrite valid tests merely to make a failing implementation pass. When a test fails, determine whether the defect is in production code, the test, or an outdated contract; change the test only when its expectation is demonstrably incorrect.
- Preserve CI coverage for the platform-independent core, CLI, bundled specs/presets, and Android packaging/signing as applicable.
- Do not claim that validation passed unless CI or another authoritative result confirms it.

## 6. Architecture and crate boundaries

Respect the workspace's separation of responsibilities:

- `mkxelz-core` owns generation rules, specification handling, deterministic output, and platform-independent logic.
- `mkxelz-cli` is an interface to the core, not a second implementation of generation behaviour.
- `mkxelz-app` owns the GUI and platform integration; it should call into the core rather than duplicate its rules.
- Keep the core free of GUI- and Android-specific dependencies.
- Keep platform-specific code isolated behind the appropriate crate boundaries and target configuration.
- Give state a clear owner and avoid redundant copies of generated data.
- Prefer explicit data flow and maintainable abstractions over premature generalisation.
- Do not add dependencies, frameworks, or architectural layers without a concrete need.

A feature available through one front end should use the same underlying core behaviour as the other front ends wherever applicable.

## 7. Determinism and generation quality

Determinism is a core contract.

- Use the project's seeded random facilities for all procedural choices that affect output.
- Do not introduce ambient, time-based, or otherwise unseeded randomness into generation.
- Keep output independent of irrelevant iteration order, platform state, and UI interaction history.
- Preserve reproducibility for identical generator version, specification, and seed.
- Add regression tests for deterministic output and important visual/generation invariants.
- When an intentional algorithm change alters pixels for existing specifications, document it and apply the project's versioning policy.
- Keep tile edges seamless where the relevant layer or tile mode promises seamless repetition.
- Preserve autotile mask semantics, including normalization rules for `blob47` and corner handling for `edges16`.
- Keep atlas ordering, dimensions, padding, and metadata indices consistent.

Do not silently change generated pixels or metadata while making an unrelated fix.

## 8. Specification, output, and compatibility

The JSON specification and exported metadata are public contracts used by downstream tools and games.

- Treat `spec_version`, exported metadata fields, tile indices, names, kinds, tags, and autotile masks as compatibility-sensitive.
- Validate input and report actionable errors rather than silently accepting malformed or contradictory specifications.
- Keep unknown-switch behaviour and conditional layer semantics consistent with the documented contract.
- Update the README spec reference and example specs when the schema changes.
- Do not silently reinterpret existing fields or change defaults in ways that alter output.
- Follow the documented SemVer policy: breaking specification or metadata changes require a major version; new features and pre-1.0 pixel-output changes follow the project's minor-version policy; fixes that do not change pixel output may use patch versions.
- Update exported version metadata and package version coherently.
- Record completed changes in `CHANGELOG.md` according to its conventions.

## 9. GUI, Android, and assets

- Treat Android and desktop as supported environments; do not solve one by unnecessarily breaking the other.
- Prefer touch-friendly controls and workflows that do not depend on a reliable soft keyboard on Android.
- Keep the GUI usable on narrow, phone-shaped screens.
- Preserve clear export locations and communicate platform-specific file-access limitations.
- Do not claim planned platform features, such as share-sheet integration or reliable on-device text editing, are implemented until they are complete.
- Keep generated output and user-provided specifications separate from application internals.
- Do not commit generated bulk assets or third-party content without a clear need and licensing review.

## 10. Collaboration and final reporting

The project owner is the final authority on visual direction, specification design, compatibility decisions, and platform priorities.

- Follow explicit decisions already made.
- Explain meaningful trade-offs when proposing a different approach.
- Ask when requirements are genuinely ambiguous or require an unapproved design decision.
- Be transparent about limitations, incomplete work, and unresolved issues.
- Do not claim a feature is complete when only part of its intended behaviour exists.

When a task is complete, report what changed, important decisions, relevant commits and PR, documentation updates, and remaining work. Distinguish completed implementation from future work.
