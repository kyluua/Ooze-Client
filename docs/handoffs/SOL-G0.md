# Sol → Claude: G0 native foundation

Owner: GPT-6.1 Sol. Milestone: G0. Status: implemented and locally verified; ready for C0 integration/review.

- Base: `d56186573ffcd002fccbe902b877a8f7315e994d` on `claude/blissful-euler-g55yql`.
- Result code commit: `d773fdfab474447b720127996e644f64c3218746` on `sol/g0-native-foundation`. The following documentation-only commit adds this handoff.
- Local worktree: `C:\Users\offic\Ooze Client\sol-g0`. The parent workspace's review/prompt files were preserved.
- Bridge: ABI **1**, library **0.1.0-g0**, normal capabilities **3**, developer fixture creation flag **1**.
- Contract: [ABI_V1.md](../bridge/ABI_V1.md). Build/run steps: [BUILD_AND_TEST.md](../bridge/BUILD_AND_TEST.md).

## Delivered and changed paths

| Path | Result |
|---|---|
| `.cargo/config.toml`, `rust-toolchain.toml` | Pinned stable Rust 1.99.1; Windows x64 static CRT |
| `native/Cargo.toml`, `native/Cargo.lock` | Locked workspace; JNI dependency 0.21.1 |
| `native/crates/ooze-core/**` | Safe Rust input validation, transactional batches, deterministic frame fixture and unit tests |
| `native/crates/ooze-jni/**` | JNI entrypoints, bounded handles, thread checks, panic containment and test-only injection |
| `bridge-java/**` | Native loader, input encoder, session wrapper, frame leases, real-DLL harness, standalone Gradle module |
| `native/tools/build-g0.ps1`, `native/licenses/**` | Build/package/verify tooling, dependency and standard-library license notices |
| `docs/bridge/**` | Exact ABI, native ownership decisions, resource manifest, build instructions |
| `.github/workflows/native-g0.yml` | Windows CI definition; release and fault artifacts separated |

No root Gradle, product UI crate, version adapter, game hooks or original product-spec files were changed. Their reconciliation belongs to Claude's C0.

## Acceptance evidence

Environment: Windows NT 10.0.26300.0, amd64; actual Temurin **21.0.12.1+1-LTS** JVM and javac; Rust **1.99.1**; MSVC 14.44.35229 linker tools. DLL target is `x86_64-pc-windows-msvc`, optimized release, thin LTO, static CRT, panic=unwind.

- [x] Core/JNI workspace and Java bridge created.
- [x] Versioned contract, resource manifest and generated per-artifact native manifests published.
- [x] Real DLL loads from the packaged JAR after SHA-256 and ABI/capability validation.
- [x] Create/destroy, batched input and deterministic 104-byte frame fixture implemented.
- [x] Null/heap/read-only output, malformed lengths/capacities/records, nonfinite values, invalid/stale handles and ABI mismatches tested.
- [x] Transactional failure, owner-thread enforcement, 64-live-session cap and **10,000 create/destroy cycles** verified.
- [x] Deferred frame remains stable through subsequent frames and session shutdown; another Java thread can consume/release it; early reuse is rejected; a failed render releases storage.
- [x] Fault-enabled DLL catches its controlled Rust panic and then continues to create/render/destroy; production probe is disabled and fault artifacts require explicit loader opt-in.
- [x] `cargo fmt --check`, Clippy with `-D warnings`, **5 Rust unit tests**, and Java compilation with `--release 21 -Xlint:all -Werror` pass.
- [x] **20,320 harness assertions per DLL variant** pass under Java 21 with `-Xcheck:jni`. The injected panic's stderr message is expected; the process exits successfully after the continuation check.
- [x] Standalone `gradle -p bridge-java build --no-daemon --console=plain --warning-mode=fail` passes using Gradle 9.8.0/Java 21. Its real check is `bridgeContractTest`; the empty JUnit `test` task is explicitly disabled.
- [x] `git diff --cached --check` passes. PE dependency inspection lists only Windows system libraries (KERNEL32, ntdll and the core synchronization API set), with no separate VCRUNTIME dependency.

The normal artifact was rebuilt from clean code commit d773fdf; native-artifact.properties records that commit with `source.dirty=false`. No game/FPS benchmark was run: these are correctness checks on a headless fixture, not performance evidence or a finished renderer. Remote CI status must be read from GitHub; local success alone is not a remote green check.

## Artifact for C0

Local normal package: `native/dist/g0/release/ooze-bridge-java-0.1.0-g0-windows-x86_64-release.jar`. It contains bridge classes, DLL and metadata/resources. The sibling `resources/` directory can be consumed by the bridge Gradle module using `-PoozeNativeResources=<absolute-directory>`.

Local transfer archive: `native/dist/g0/ooze-g0-windows-x86_64.zip` (normal artifact only). Outputs are intentionally not committed. Build the same package using the documented script or download the `ooze-g0-windows-x86_64-release` CI artifact once available. The `FAULT-INJECTION-DO-NOT-SHIP` artifact is test-only.

| Clean-code local artifact | SHA-256 |
|---|---|
| `ooze_jni.dll` | `79a168b0d54cdbb3111c6c2f2a9565d3208ac74640ba70de9349b3b733994fa8` |
| Packaged bridge JAR | `e6759ac3061fb9283737342e2a771c0595ec4bbf18184f86bac9626e1e16bb62` |

Generated `SHA256SUMS` records each build. JAR timestamps/build metadata may change hashes on rebuild; always use the manifest accompanying that actual build rather than treating the local JAR checksum as a universal reproducible-build value.

## Exact next task: Claude C0

1. Fetch `sol/g0-native-foundation` and incorporate its code/contract into your own branch. Review the boundary and frame ownership before integrating. Preserve Sol's file ownership when requesting interface changes.
2. Reconcile AGENTS.md and the original specification with Fabric 1.21.11, Java 21, the Rust engine/UI and official-launcher authentication. Stage the milestone gates and correct the earlier backup/texture-budget/dependency issues.
3. Create the root pinned Fabric build, include `:bridge-java`, and depend on it from `:versions:mc1_21_11`. Consume the prebuilt normal native resources; bundle bridge classes/resources exactly once into the final mod. Do not relocate NativeBindings or add another Fabric mod ID.
4. On the client platform thread, call `NativeLoader.tryLoad(cacheRoot)` and check `available()`. On failure, log the diagnostic and leave vanilla screens/HUD usable without unavailable Ooze controls. Create/close EngineSession only on its owning thread.
5. Behind `-Doozeclient.dev=true`, exercise one InputBatch resize/pointer/button submission and render the fixture into a developer diagnostic screen. Convert physical pixels to the target GUI projection and numeric ARGB to its actual vertex format.
6. Hold each FrameLease until vanilla actually consumes its vertices, or eagerly copy into Minecraft-owned state before closing it. Close discarded leases on every screen/resource shutdown path. Do not close a lease merely because GUI extraction returns.
7. Verify the normal target reaches the vanilla title screen, then the developer fixture works. In fresh runs, verify missing/corrupt/ABI-incompatible native packaging produces vanilla fallback. Record logs and runtime evidence; G0 has not performed these in-game checks.
8. Leave product screen/component implementation for the next agreed milestone. Keep Microsoft sign-in/entitlement/session secrets in the launcher/game; no new login backend is needed.

## Limits and review status

Only Windows x64 is claimed. No actual Minecraft launch, renderer ordering/AA, authentication flow, full client packaging, OS matrix, game FPS or GPU-memory budget has been verified. Core/UI algorithms beyond the small G0 fixture are not implemented. Native panics caused by aborts/memory faults are not recoverable through catch_unwind.

No Claude implementation diff was available in the inspected starting branch, so cross-review of Claude's C0 remains due when that diff exists. This handoff completes G0 and does not start G1.
