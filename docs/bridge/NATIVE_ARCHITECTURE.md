# Native architecture decision: G0

The latest owner instruction selects Fabric 1.21.11 with Java 21 and a Rust engine/UI. It supersedes the old requirement to implement three versions with a pure-Java core. Claude's C0 has reconciled AGENTS.md and the original product specification; this document records Sol's native decisions without rewriting Claude's files.

| Area | Owner | G0 state |
|---|---|---|
| `native/crates/ooze-core` | Sol | Safe Rust packet validation and deterministic diagnostic geometry |
| `native/crates/ooze-jni` | Sol | Small JNI boundary, bounded registry, status translation |
| `bridge-java` | Sol | JVM-independent loader, public session/encoder/frame leases, harness, standalone Gradle module |
| `native/crates/ooze-ui` | Claude | Not created; Cargo's `crates/*` member glob will discover it when added |
| `versions/mc1_21_11` and root Gradle | Claude | C0 integrated unchanged after review; connector integration remains C0b |
| Product docs/assets | Claude | Design preserved; C0 target/language reconciliation integrated |

Rust 1.99.1 is pinned in the root rust-toolchain.toml. The edition is 2024. Cargo.lock pins dependencies; the intentional JNI library dependency is 0.21.1. Rust release uses `panic=unwind`, thin LTO and one codegen unit; the Windows target statically links its CRT to avoid an additional Visual C++ redistributable dependency. Builds must start from the repository root so `.cargo/config.toml` applies.

No preview Java APIs, second JVM, Rust game runtime, direct OpenGL/Vulkan calls or runtime compiler/downloader is introduced. `bridge-java` has no Minecraft/Fabric dependencies. Every Rust unsafe block is confined to the two synchronous checked direct-buffer borrows in the JNI layer; core forbids unsafe code. JNI is loaded once by the same class loader as NativeBindings.

The boundary favors whole batches over object graphs or one call per vertex. State parsing is transactional, heap-free per submitted record, and bounded by 1024 records. Output writes into Java-owned buffers. The global registry caps live native sessions at 64 and reclaims entries on explicit close; it does not keep dead handles. Monotonic IDs prevent stale-handle aliasing.

Frame leases explicitly address the deferred-rendering risk identified in the review. No pointer into Rust-owned memory crosses the boundary. It is still the adapter's responsibility to retain each lease until vertex consumption and to close abandoned work on all screen/resource shutdown paths.

G0 deliberately implements only a diagnostic rectangle. UI material painting, typography, tessellation/AA, real settings/config, search, background decoding, module state and HUD behavior are later milestones. Their empty placeholders are not shipped as product controls. The original visual/accessibility/token specifications remain requirements for those milestones.

Resources and ownership for future work are defined in ABI_V1.md. Java owns resource-pack/i18n access, vanilla text fallback, Minecraft state and GPU operations. Rust owns reusable engine/UI logic. Future texture lifetimes must account for both CPU-native and GPU memory; a JVM allocation measurement alone is insufficient.

G0 makes **no FPS, latency or zero-allocation claim**. Its harness validates correctness, not representative client performance. Later performance work must report release-build end-to-end JNI/copy costs, Java allocations, native retained/peak bytes and GPU residency, and compare representative kernels to reasonable Java baselines. The frame lease object/view wrappers and Rust registry lock are known overheads to measure before optimization.

The account/session boundary remains in the signed-in game. No auth secret is logged, transported to Rust, stored in native configuration or requested from the user by this module.
