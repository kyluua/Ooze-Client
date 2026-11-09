# Sol → Claude: G0 native foundation

Owner: GPT-6.1 Sol. Milestone: G0. Status: implemented and locally verified; ready for C0 integration/review.

- Base: `d56186573ffcd002fccbe902b877a8f7315e994d` on `claude/blissful-euler-g55yql`.
- Result code commit: `190869bd46acca5500d943d0f187c6ce310fa6df` on `sol/g0-native-foundation`; initial foundation was `d773fdfab474447b720127996e644f64c3218746`. The final documentation-only commit updates this handoff.
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

Sol did not edit Claude-owned root Gradle, product UI, adapter or product-spec files. Claude concurrently delivered C0 at `0dca252d06e0b6486a35c0dc5322cf4e0e07a7e8`; it was reviewed and integrated unchanged as the second parent of the result code commit. See [the concrete C0 review](SOL-G0-CLAUDE-C0-REVIEW.md). C0b integration remains Claude's next task.

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
- [x] **20,323 harness assertions per DLL variant** pass under Java 21 with `-Xcheck:jni`, including typed native-load states/version reporting. The injected panic's stderr message is expected; the process exits successfully after the continuation check.
- [x] Standalone `gradle -p bridge-java build --no-daemon --console=plain --warning-mode=fail` passes using Gradle 9.8.0/Java 21. Its real check is `bridgeContractTest`; the empty JUnit `test` task is explicitly disabled.
- [x] The combined `gradlew.bat build buildAll` passes on Windows x64/Java 21/Gradle 9.8.2, including **21 Claude adapter unit tests**, bridgeContractTest and the remapped client JAR. This is **Built** and **Headless-tested**, not **Game-launched** evidence. Existing Loom/Claude Gradle deprecation warnings remain.
- [x] Whitespace checks pass for Sol's diff relative to Claude C0. PE dependency inspection lists only Windows system libraries (KERNEL32, ntdll and the core synchronization API set), with no separate VCRUNTIME dependency.

The normal artifact was rebuilt from clean code commit 190869b; native-artifact.properties records that commit with `source.dirty=false`. No game/FPS benchmark was run: these are correctness checks on a headless fixture, not performance evidence or a finished renderer. The initial native CI runs passed; the compatibility commit's native + combined Fabric CI checks are linked from [draft PR #1](https://github.com/kyluua/Ooze-Client/pull/1). Consult the current check result rather than interpreting local success as a remote green check.

## Artifact for C0

Local normal package: `native/dist/g0/release/ooze-bridge-java-0.1.0-g0-windows-x86_64-release.jar`. It contains bridge classes, DLL and metadata/resources. The sibling `resources/` directory can be consumed by the bridge Gradle module using `-PoozeNativeResources=<absolute-directory>`.

Local transfer archive: `native/dist/g0/ooze-g0-windows-x86_64.zip` (normal artifact only). Outputs are intentionally not committed. Build the same package using the documented script or download the `ooze-g0-windows-x86_64-release` CI artifact once available. The `FAULT-INJECTION-DO-NOT-SHIP` artifact is test-only.

| Clean-code local artifact | SHA-256 |
|---|---|
| `ooze_jni.dll` | `79a168b0d54cdbb3111c6c2f2a9565d3208ac74640ba70de9349b3b733994fa8` |
| Packaged bridge JAR | `c6acc1d8de014bbc63bc41073c2eb703d92c8f801e22d4ccc04b6e30e29bada0` |

Generated `SHA256SUMS` records each build. JAR timestamps/build metadata may change hashes on rebuild; always use the manifest accompanying that actual build rather than treating the local JAR checksum as a universal reproducible-build value.

## Exact next task: Claude C0b

1. Fetch `sol/g0-native-foundation`, review G0 and the C0 review notes, then integrate into your own branch. The result code commit already contains your C0 unchanged. Preserve Sol's file ownership when requesting interface changes.
2. Add BridgeEngineConnector at your existing EngineConnectors seam, delegating to NativeLoader and EngineSession. Map LoadResult.state() directly to your typed EngineStatus states; bridgeVersion()/protocolVersion() expose 0.1.0-g0 / 1. Diagnostics are log details, not state identifiers to parse.
3. Consume the prebuilt normal resources without requiring Cargo for Java-only contributors. Your existing root includes bridge-java automatically. Resolve the packaging review item: the current combined JAR nests a Loom-generated helper mod; direct class/resource inclusion is the G0 recommendation if keeping one mod ID. Do not relocate NativeBindings or duplicate it across loaders.
4. On the client platform thread, call NativeLoader.tryLoad(cacheRoot), check available(), and create/close EngineSession on that thread only. Preserve vanilla screens/HUD on failure. Catch controlled failures without swallowing VM-fatal errors; disable the engine gate after an unrecoverable session error.
5. Wire the existing developer-only F9 probe to submit one resize/pointer/button batch, produce a fixture frame, verify its header/vertices and close its lease. Rendering it through a GUI adapter belongs to C1 unless deliberately added to the agreed C0b scope.
6. For any deferred rendering, hold each FrameLease until actual vertex consumption, or copy into Minecraft-owned state before closing it. Close discarded leases on shutdown. Positions are physical pixels; colors are numeric ARGB. G0 does not supply a GUI pipeline.
7. On Windows x64, verify the real native library reaches LOADED and the F9 round trip succeeds. In fresh runs, verify missing/corrupt/ABI-incompatible native packaging produces vanilla fallback. Record logs and runtime evidence; G0 has not performed these in-game checks. Other hosts can package prebuilt Windows resources, while the Windows-only contract test is explicitly SKIPPED there.
8. Keep product UI and the reserved protocols for the agreed later milestones. Keep sign-in/entitlement/session secrets in the launcher/game; no new login backend is needed. The completed C0 spec reconciliation does not need repeating.

## Limits and review status

Only Windows x64 native support is claimed. A combined client JAR was Built, but actual Minecraft launch with G0, renderer ordering/AA, authentication, runtime class loading of the packaged bridge, the OS matrix, FPS and GPU-memory budgets remain unverified. Core/UI algorithms beyond the small fixture are not implemented. Aborts/memory faults are not recoverable through catch_unwind.

Claude's C0 diff became available during G0 and was reviewed before integration; findings and combined build evidence are recorded in SOL-G0-CLAUDE-C0-REVIEW.md. This handoff completes G0 and does not start G1 or implement Claude's C0b connector.
