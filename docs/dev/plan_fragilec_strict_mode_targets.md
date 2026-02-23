# Plan: Complete `fragilec` strict mode for real-world target projects

## Goal
Make `fragilec` usable as a drop-in compiler (`CC`/`CXX`) in strict mode for our current external targets, so builds and project test paths pass from source without passthrough mode.

Targets in scope:
- zlib
- tinyxml2
- pugixml
- rapidjson
- xxhash

## Current baseline (2026-02-23)
- `pass` mode has been removed from `fragilec`.
- `auto` mode has been removed from `fragilec`.
- `fragilec` is strict-only:
  - compile-only single-source invocations are handled by fragile compile.
  - unsupported invocation shapes fail (no native fallback).

## Constraints we must close for strict
1. Compile shape:
- strict currently only accepts compile-only, single-source (`-c`) invocations.
- strict rejects direct link commands and multi-input compile commands.

2. Link/ABI path:
- strict compile emits Rust object files from transpiled units.
- complete project builds need a compatible symbol/export/link strategy across translation units.

3. Build-system integration:
- CMake/autoconf/make perform compiler sanity checks and mixed compile/link probes.
- strict must support those probes without silently delegating to native compiler.

4. Artifact model:
- we need deterministic object and archive (`.a`) production from strict outputs.
- metadata sidecars must remain enforceable (`FRAGILEC_BUILD_ID`, link input checks).

## Milestones

### M0: Keep baseline green while strict is incomplete
Exit criteria:
- strict-only behavior is enforced in driver/harness tests (no hidden fallback).
- strict mode has explicit, deterministic failure messages for unsupported invocation shapes.

Work:
- keep baseline harnesses explicit about where strict currently fails.
- keep strict-mode error text stable and tested.

### M1: Strict compile-only parity for target compile commands
Exit criteria:
- for each target, we can run its captured compile command list in strict mode and produce all expected `.o` outputs.

Work:
- expand argument support in strict parser (defines/includes/std flags, output conventions).
- add strict replay test entrypoints per project that execute compile-command manifests only.
- compare output presence and command-level status with native baseline.

Per-target harness inputs:
- zlib: `cc_driver.log` compile units + `compile_units_manifest.txt`
- tinyxml2: `cxx_driver.log` compile units + `compile_units_manifest.txt`
- pugixml: make test compile commands from driver logs/manifests
- rapidjson: no-STL example compile commands
- xxhash: `xxhsum` compile commands in make trace

### M2: Strict static archive generation (`.a`) and validation
Exit criteria:
- strict flow can generate project static libraries used by targets (`libz.a`, `libtinyxml2.a`, etc.) from strict-produced objects.
- archive member set matches native baseline (name-level parity).

Work:
- add archive creation path to `fragilec` workflow (or strict helper tool) using deterministic `ar` invocation.
- add manifest checks for archive members and object provenance metadata.

### M3: Strict link support for executables
Exit criteria:
- strict mode can complete link commands for target test binaries used in current harnesses.
- native-vs-strict parity checks pass for exit status and essential runtime behavior.

Work:
- implement strict handling for link invocations (no native fallback).
- enforce link-input metadata/build-id checks in strict path.
- validate symbol export/import assumptions with multi-object builds.

### M4: Build-system drop-in path (`CC/CXX=fragilec`, strict)
Exit criteria:
- configure/cmake compiler checks pass with strict mode.
- end-to-end build + project test command passes per target under strict mode.

Work:
- handle compiler-identification probes and try-compile patterns.
- support canonical flag subsets emitted by CMake/autoconf for these targets.

## Project-by-project graduation matrix

1. zlib
- Gate A: strict builds required object set for `make all`.
- Gate B: strict produces `libz.a` with expected members.
- Gate C: strict links/runs `example`, `minigzip`, and `make test` subset parity checks.

2. tinyxml2
- Gate A: strict builds `tinyxml2.o` and `xmltest.o` from driver manifest.
- Gate B: strict links `xmltest`.
- Gate C: strict `make test` subset replay parity (status/stdout/artifacts).

3. pugixml
- Gate A: strict compile replay for no-STL test objects.
- Gate B: strict links test binary used by `make test` no-STL path.
- Gate C: strict `make test` no-STL success.

4. rapidjson
- Gate A: strict compiles `condense`/`pretty` examples.
- Gate B: strict links both binaries.
- Gate C: runtime output checks for condense/pretty match baseline expectations.

5. xxhash
- Gate A: strict compiles all `xxhsum` objects.
- Gate B: strict links `xxhsum`.
- Gate C: `xxhsum --version` and selected CLI checks pass.

## CI rollout strategy
1. Add new ignored strict replay tests per target (compile-only first).
2. Keep nightly jobs strict-only; do not add fallback lanes.
3. Promote strict jobs to required once M3/M4 gates are met for that target.

## Definition of done
All scoped targets satisfy:
- build from source with `CC=fragilec`/`CXX=fragilec` and `FRAGILEC_MODE=strict`
- complete their existing harness test path successfully
- no native-compiler fallback on strict path
- deterministic artifacts and metadata checks for link inputs
