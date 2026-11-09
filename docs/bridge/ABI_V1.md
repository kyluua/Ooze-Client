# Ooze native bridge ABI v1 — G0

Status: implemented G0 subset. Owner: Sol. Target: Minecraft Fabric **1.21.11**, Java **21**, Windows x64 native library. This contract records the owner's Rust architecture decision; older Java-core/multi-target instructions remain for Claude to reconcile in C0.

## Compatibility and discovery

The Java package is `io.github.kyluua.ooze.bridge`. Do not relocate or obfuscate its JNI declarations. `NativeBindings` is package-private; consumers use `NativeLoader`, `EngineSession`, `InputBatch` and `FrameBuffer`.

`JNI_OnLoad` advertises JNI 1.8 so the VM knows this library uses post-1.1 functions such as direct buffers. The Java classes/runtime still require Java 21. G0 does not create another JVM.

`bridge.abi=1` identifies the exact layouts below. All multibyte packet fields are **little endian**. Java must use `ByteOrder.LITTLE_ENDIAN`. Packed colors are numeric `0xAARRGGBB` in sRGB with straight alpha; their serialized LE bytes are B,G,R,A. The rendering adapter must convert to the vertex format it actually submits, rather than assuming these bytes match its GPU layout.

| JNI method | Result |
|---|---|
| `int abiVersion()` | `1`, or a negative status |
| `long capabilities()` | bitmask below, or a negative status |
| `long create(int expectedAbi, int flags)` | positive opaque handle, or negative status |
| `int destroy(long handle)` | 0 on success |
| `int submit(long handle, ByteBuffer input, int byteLength)` | 0 on success |
| `int renderFixture(long handle, long frameId, ByteBuffer output, int byteCapacity)` | 104 bytes on success |
| `int probePanic()` | -7 normally; -6 after catching the test-only injected panic |

Capabilities: bit 0 (`1`) is the diagnostic triangle fixture; bit 1 (`2`) is the input batch; bit 2 (`4`) is test-only fault injection. Normal artifacts advertise `3`, fault artifacts `7`. Only the fault artifact implements the injected panic; its loader also requires `-Dooze.bridge.allowFaultInjection=true`. Never package that artifact into the client.

Creation flag bit 0 (`DEV_FIXTURE=1`) enables fixture rendering for that session. Flag 0 creates a session that accepts input but cannot render the diagnostic fixture. Unknown bits are rejected. No production Ooze screen/module functionality is advertised by G0.

## Errors and session lifecycle

| Status | Meaning |
|---|---|
| -1 | ABI mismatch |
| -2 | Invalid argument, malformed packet, null/heap/read-only output or invalid capacity |
| -3 | Invalid/stale session handle |
| -4 | Output region too small |
| -5 | Operation called from another native thread |
| -6 | An unwinding Rust panic was caught |
| -7 | Unsupported operation/capability |
| -8 | Session/handle limit |
| -9 | Internal error, including poisoned registry |

Do not depend on which validation error is reported first when multiple arguments are invalid. Native validation failures do not deliberately throw Java exceptions; genuine JVM/JNI failures can still propagate a pending JVM exception.

Handles are monotonically increasing positive signed 64-bit integers, **not pointers**. They are never reused within the process. At most 64 sessions may be live. Handle exhaustion fails instead of wrapping. Destroy is explicit; a second raw destroy returns -3, while `EngineSession.close()` is idempotent.

Sessions and Java input encoders belong to their creating **platform thread**. Use the Minecraft client/render thread. `EngineSession` rejects Java virtual threads because migration can change their native carrier thread. All Java and Rust calls are synchronous; no callback or worker accesses Minecraft objects. Multiple sessions are serialized by a small Rust registry mutex. No `JNIEnv`, local/global Java reference, input pointer or output pointer is retained.

Rust entrypoints catch unwinding panics. A panic while holding registry state may poison that registry; further operations then fail with -9. The adapter must disable the affected Ooze integration. This does not recover aborts, access violations, OOM aborts or other native memory faults. The deliberate panic probe does not hold registry state.

## Input packet

The caller supplies a direct `ByteBuffer`, and native reads bytes `[0, byteLength)` relative to that buffer's own base. Position and limit are deliberately not consulted. A direct slice has its own adjusted base/capacity. Read-only **input** is supported. The caller must keep the buffer alive and grant exclusive access until return. Raw calls are a trusted internal boundary; concurrent Java mutation of borrowed bytes is forbidden.

| Header offset | Type | Value |
|---|---|---|
| 0 | 4 bytes | ASCII `OZIN` |
| 4 | u32 | ABI 1 |
| 8 | u32 | record count, 0..1024 |
| 12 | u32 | record stride, exactly 24 |

Each record is `(kind:u32, a:u32, b:u32, x:f32, y:f32, reserved:u32)` at offsets 0,4,8,12,16,20. Reserved must be zero. Floats must be finite. Packet length must equal `16 + count*24`, with no trailing bytes; maximum is 24,592 bytes. A zero-record packet is a valid no-op.

| Kind | Meaning and constraints |
|---|---|
| 1 | Resize: a=framebuffer width and b=height (each 1..16384), x=Ooze UI scale (0.125..8), y=0 |
| 2 | Pointer move: a=b=0; x,y are physical framebuffer coordinates, clamped when producing the fixture |
| 3 | Pointer button: a=button (0..7), b=released/pressed (0/1), x=y=0. Only button 0 changes the fixture; other buttons are accepted without fixture behavior |
| 4 | Comfort state: a=reduced motion (0/1), b=x=y=0. Stored; this static fixture has identical output in both modes |

Unknown kinds are rejected. Validation is transactional: an invalid record rejects the **whole** batch without applying earlier events. `InputBatch` is the reusable Java encoder; `submit()` does not clear it automatically.

## Frame packet and deferred consumption

Output must be a writable direct `ByteBuffer`. The explicit capacity must be nonnegative, no larger than actual JVM capacity, and at most 16 MiB. G0 requires 104 bytes and writes exactly that many. A short-output failure writes nothing. Frame IDs are caller-supplied signed longs in 0..Long.MAX_VALUE and carry no animation-time meaning.

| Header offset | Type | Value |
|---|---|---|
| 0 | 4 bytes | ASCII `OZFR` |
| 4 | u32 | ABI 1 |
| 8 | u32 | total bytes, 104 |
| 12 | u32 | fixture marker, 1 |
| 16 | u64 | frame ID |
| 24 | u32 | vertex count, 6 |
| 28 | u32 | vertex stride, 12 |

Six vertices follow at offset 32, each `(x:f32, y:f32, argb:u32)`. They form a triangle list in order `(top-left, top-right, bottom-right), (top-left, bottom-right, bottom-left)`, in physical framebuffer coordinates with Y down. The diagnostic rectangle is 32*scale by 24*scale, constrained to the viewport. Its top-left follows the pointer, clamped so the whole rectangle fits. Button 0 changes its opaque color from `#F8F9FF` to `#171923`. This is test geometry, not the Ooze material renderer or its tokenized product controls.

Java owns all output memory. `FrameBuffer` refuses a write while its previous `FrameLease` is live. A lease exposes read-only LE views and can be closed on the eventual consuming thread. The buffer is independent of the Rust session and remains valid after session destruction.

**Close the lease only after vanilla has actually consumed/copied its vertices**, not merely when GUI element extraction returns. If the adapter copies eagerly into Minecraft-owned storage, close immediately after that copy; if it queues a deferred reader, transfer the lease to that reader and close in its final consumption/cleanup path. Pool multiple `FrameBuffer`s if frames overlap. After closing, discard all views: already returned `ByteBuffer`s cannot be revoked, and keeping one does not prevent buffer reuse. The lease API enforces ownership discipline, not a substitute for following it.

## Reserved future work — NOT implemented in G0

No command IDs, offsets or JNI methods for these are currently assigned. Do not send made-up records or advertise these capabilities:

- Immutable game snapshots: Java extracts only needed primitive/non-secret data; Rust never receives Minecraft objects or authentication tokens.
- Frame time, ticks, keyboard/character input, focus loss and animation policy: a monotonic timestamp/delta contract must precede animation implementation. Frame ID is not a clock.
- Product geometry, clips, texture batches and text commands: preserve painter order; define upload/reload/retirement epochs and separate opaque Java GPU resource IDs from Rust handles.
- Platform actions/narration: Rust proposes validated action IDs; Java executes on the appropriate game thread. No arbitrary callbacks or function pointers across the ABI.
- Text measurement/localization: Java owns resource-pack translation, vanilla complex-script fallback and Minecraft font interop. Transfer text/metric requests in batches with caches. Future native rasterization/layout belongs in Rust with a documented font-data/metric contract; G0 has no glyph pipeline.
- Images/fonts/texture uploads: Java owns GPU creation, upload and destruction through vanilla pipelines. Rust may decode/process immutable bytes later. `resources.json` presently declares no product assets.

Add optional operations through explicit capabilities and new methods. Never silently change these existing packet layouts; incompatible changes require a new ABI and matching packaged artifacts.

## Artifact and authentication contract

The artifact contains `META-INF/ooze/native.properties`, `META-INF/ooze/resources.json`, notices/licenses and `natives/windows-x86_64/ooze_jni.dll`. Required properties are schema=1, library.version=0.1.0-g0, bridge.abi=1, platform=windows-x86_64, resource as above, a lowercase 64-digit sha256 and capabilities=3 (or gated test value 7). `variant`, `rust.target`, `source.commit` and `source.dirty` identify builds. The manifest is generated by `native/tools/build-g0.ps1` and is copied separately as `native-artifact.properties` beside each JAR.

`NativeLoader.tryLoad(cacheRoot)` accepts only this platform/version/path, verifies SHA-256, and extracts to `<cacheRoot>/0.1.0-g0/<sha256>/windows-x86_64/ooze_jni.dll`. Hashes are trusted metadata from the supplied JAR; they detect corruption/mismatching files, not a signature from an independent trust authority. Existing corrupt cache entries are refused. No native code is downloaded at runtime. A changed DLL requires a fresh JVM; Java cannot safely hot-reload the same JNI class with another library.

Missing/incompatible libraries return `LoadResult.available=false` with a log diagnostic. C0 must retain vanilla screens/HUD and suppress unavailable Ooze controls. The loader itself has no Fabric dependency, so only C0 can demonstrate the actual in-game fallback.

The Java facade also exposes `LoadResult.state()` as `LOADED`, `MISSING_LIBRARY`, `UNSUPPORTED_PLATFORM`, `INCOMPATIBLE` or `LOAD_FAILED`; `bridgeVersion()` and `protocolVersion()` identify this Java bridge's expected semantic/protocol versions, including on failure. Missing bundled metadata/DLL is MISSING_LIBRARY; version/hash mismatches are INCOMPATIBLE; platform selection happens before loading; I/O, permission and linkage failures are LOAD_FAILED. Anticipated loader failures return a result. This does not promise recovery from VM-fatal errors or make every invalid Java caller safe. The Fabric connector must map these typed states rather than parsing English diagnostic strings.

Minecraft is launched and signed in through the official launcher. Ooze neither implements OAuth nor exports session credentials; no separate Azure registration is part of this architecture.

References: [Java 21 JNI functions](https://docs.oracle.com/en/java/javase/21/docs/specs/jni/functions.html), [Rust FFI unwind rules](https://doc.rust-lang.org/stable/nomicon/ffi.html#ffi-and-unwinding). Implementation dependency `jni` is pinned to 0.21.1 in Cargo.lock; its checked-in downloaded source was consulted for direct-buffer capacity/address behavior.
