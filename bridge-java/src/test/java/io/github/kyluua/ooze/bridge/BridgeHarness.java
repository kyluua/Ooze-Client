package io.github.kyluua.ooze.bridge;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.StringWriter;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.Properties;
import java.util.concurrent.atomic.AtomicInteger;

/** Dependency-free contract tests against the actual packaged DLL, under Java 21. */
public final class BridgeHarness {
    private static final ClassLoader RESOURCES = BridgeHarness.class.getClassLoader();
    private static final String MANIFEST = "META-INF/ooze/native.properties";
    private static int checks;

    private BridgeHarness() {}

    public static void main(String[] args) throws Exception {
        check(Runtime.version().feature() == 21, "Harness must execute under Java 21");
        boolean faults = args.length > 0 && args[0].equals("--fault-injection");
        Path cache = Files.createTempDirectory("ooze-g0-harness-");
        System.out.println("Runtime: " + Runtime.version() + " / " + System.getProperty("os.arch"));
        loaderFailures(cache);
        NativeLoader.LoadResult result = NativeLoader.tryLoad(cache.resolve("valid"));
        check(result.available(), result.diagnostic());
        check(result.capabilities() == (faults ? 7 : 3), "Expected release/test capabilities");
        check(NativeBindings.abiVersion() == 1, "Live native ABI probe");
        check(NativeLoader.tryLoad(cache.resolve("valid")).available(), "Repeated bootstrap is supported");
        System.out.println("PASS packaged load/hash/ABI/capability handshake");
        rawValidation();
        deterministicFramesAndLeases();
        threadConfinement();
        lifecycleAndLimits();
        int panicResult = NativeBindings.probePanic();
        check(panicResult == (faults ? BridgeProtocol.PANIC : BridgeProtocol.UNSUPPORTED), "Panic probe status");
        try (EngineSession session = new EngineSession(BridgeProtocol.DEV_FIXTURE);
                var frame = session.renderFixture(100, new FrameBuffer())) {
            check(frame.bytes().getLong(16) == 100, "Engine/JVM still live after panic probe");
        }
        System.out.println("PASS " + (faults ? "caught native panic / continued operation" : "production panic probe disabled"));
        System.out.println("PASS G0: " + checks + " assertions; Minecraft launch/rendering NOT tested");
        System.out.println("Cache retained for inspection: " + cache);
    }

    private static void loaderFailures(Path cache) throws Exception {
        ClassLoader missing = new ClassLoader(RESOURCES) {
            @Override
            public InputStream getResourceAsStream(String name) {
                return null;
            }
        };
        check(!NativeLoader.tryLoad(cache.resolve("missing"), 1, missing).available(), "Missing artifact fails soft");
        Properties original = new Properties();
        try (InputStream input = RESOURCES.getResourceAsStream(MANIFEST)) {
            if (input == null) {
                throw new IOException("Harness needs the packaged manifest");
            }
            original.load(input);
        }
        Properties wrongAbi = copy(original);
        wrongAbi.setProperty("bridge.abi", "2");
        check(!NativeLoader.tryLoad(cache.resolve("abi"), 1, overrideManifest(wrongAbi, false)).available(), "Manifest ABI mismatch fails soft");
        check(!NativeLoader.tryLoad(cache.resolve("expected-abi"), 2, RESOURCES).available(), "Java expected ABI mismatch fails soft");
        Properties wrongPlatform = copy(original);
        wrongPlatform.setProperty("platform", "linux-x86_64");
        check(!NativeLoader.tryLoad(cache.resolve("platform"), 1, overrideManifest(wrongPlatform, false)).available(), "Wrong native platform rejected");
        Properties badHash = copy(original);
        badHash.setProperty("sha256", "0".repeat(64));
        NativeLoader.LoadResult corrupted = NativeLoader.tryLoad(cache.resolve("hash"), 1, overrideManifest(badHash, false));
        check(!corrupted.available() && corrupted.diagnostic().contains("SHA-256"), "Wrong packaged hash fails soft");
        check(!NativeLoader.tryLoad(cache.resolve("dll-missing"), 1, overrideManifest(original, true)).available(), "Missing DLL fails soft");
        Properties faultManifest = copy(original);
        faultManifest.setProperty("capabilities", "7");
        String faultOptIn = System.getProperty("ooze.bridge.allowFaultInjection");
        System.clearProperty("ooze.bridge.allowFaultInjection");
        try {
            var refused = NativeLoader.tryLoad(cache.resolve("fault-opt-in"), 1, overrideManifest(faultManifest, false));
            check(!refused.available() && refused.diagnostic().contains("opt-in"), "Fault artifact is refused by default");
        } finally {
            if (faultOptIn != null) {
                System.setProperty("ooze.bridge.allowFaultInjection", faultOptIn);
            }
        }
        Path staleRoot = cache.resolve("tampered");
        Path stale = staleRoot.resolve(BridgeProtocol.LIBRARY_VERSION).resolve(original.getProperty("sha256"))
                .resolve("windows-x86_64/ooze_jni.dll");
        Files.createDirectories(stale.getParent());
        Files.write(stale, new byte[] {1, 2, 3});
        check(!NativeLoader.tryLoad(staleRoot).available(), "Tampered cache is rejected");
        check(NativeLoader.supportedPlatform("Windows 11", "amd64"), "Windows x64 platform selection");
        check(!NativeLoader.supportedPlatform("Windows 11", "aarch64"), "Windows ARM is not claimed");
        check(!NativeLoader.supportedPlatform("Linux", "amd64"), "Linux is not claimed");
        try (var files = Files.walk(cache)) {
            check(files.noneMatch(file -> file.getFileName().toString().endsWith(".tmp")), "Failed extraction cleans temporary files");
        }
        check(!NativeLoader.isReady(), "No failed bootstrap made the bridge ready");
        System.out.println("PASS missing/corrupt/incompatible artifact fallback");
    }

    private static Properties copy(Properties original) {
        Properties result = new Properties();
        result.putAll(original);
        return result;
    }

    private static ClassLoader overrideManifest(Properties manifest, boolean hideDll) throws IOException {
        StringWriter writer = new StringWriter();
        manifest.store(writer, "test");
        byte[] bytes = writer.toString().getBytes(java.nio.charset.StandardCharsets.ISO_8859_1);
        return new ClassLoader(RESOURCES) {
            @Override
            public InputStream getResourceAsStream(String name) {
                if (name.equals(MANIFEST)) {
                    return new ByteArrayInputStream(bytes);
                }
                if (hideDll && name.endsWith(".dll")) {
                    return null;
                }
                return super.getResourceAsStream(name);
            }
        };
    }

    private static ByteBuffer input() {
        return ByteBuffer.allocateDirect(40).order(ByteOrder.LITTLE_ENDIAN)
                .putInt(0x4e495a4f).putInt(1).putInt(1).putInt(24)
                .putInt(2).putInt(0).putInt(0).putFloat(12).putFloat(34).putInt(0);
    }

    private static ByteBuffer output() {
        return ByteBuffer.allocateDirect(104).order(ByteOrder.LITTLE_ENDIAN);
    }

    private static void rawValidation() {
        check(NativeBindings.create(2, 1) == -1, "Native creation enforces ABI");
        check(NativeBindings.create(1, -1) == -2, "Unknown session flags rejected");
        long handle = NativeBindings.create(1, 1);
        check(handle > 0, "Raw session created");
        try {
            ByteBuffer input = input();
            for (int length : new int[] {-1, 0, 15, 39, 41, Integer.MAX_VALUE}) {
                check(NativeBindings.submit(handle, input, length) == -2, "Invalid input length " + length);
            }
            check(NativeBindings.submit(handle, null, 40) == -2, "Null input rejected");
            check(NativeBindings.submit(handle, ByteBuffer.allocate(40), 40) == -2, "Heap input rejected");
            check(NativeBindings.submit(handle, input.asReadOnlyBuffer(), 40) == 0, "Read-only direct input is accepted");
            for (int offset : new int[] {0, 4, 8, 12, 16, 20, 24, 36}) {
                int previous = input.getInt(offset);
                input.putInt(offset, -1);
                check(NativeBindings.submit(handle, input, 40) < 0, "Malformed record field " + offset);
                input.putInt(offset, previous);
            }
            for (float value : new float[] {Float.NaN, Float.POSITIVE_INFINITY, Float.NEGATIVE_INFINITY}) {
                input.putFloat(28, value);
                check(NativeBindings.submit(handle, input, 40) == -2, "Nonfinite pointer rejected");
            }
            input.putFloat(28, 12);
            ByteBuffer out = output();
            for (int i = 0; i < 104; i++) {
                out.put(i, (byte) 0x5a);
            }
            check(NativeBindings.renderFixture(handle, 0, out, 103) == -4, "Short output rejected");
            for (int i = 0; i < 104; i++) {
                check(out.get(i) == (byte) 0x5a, "Short output did not write");
            }
            check(NativeBindings.renderFixture(handle, 0, out, 105) == -2, "Claimed capacity larger than buffer rejected");
            check(NativeBindings.renderFixture(handle, 0, out, -1) == -2, "Negative output capacity rejected");
            check(NativeBindings.renderFixture(handle, -1, out, 104) == -2, "Negative frame id rejected");
            check(NativeBindings.renderFixture(handle, 0, null, 104) == -2, "Null output rejected");
            check(NativeBindings.renderFixture(handle, 0, ByteBuffer.allocate(104), 104) == -2, "Heap output rejected");
            check(NativeBindings.renderFixture(handle, 0, out.asReadOnlyBuffer(), 104) == -2, "Read-only output rejected");
            ByteBuffer storage = ByteBuffer.allocateDirect(120).order(ByteOrder.LITTLE_ENDIAN);
            storage.putLong(0, 0x1122334455667788L).putLong(112, 0x1122334455667788L);
            storage.position(8).limit(112);
            check(NativeBindings.renderFixture(handle, 2, storage.slice(), 104) == 104, "Direct slice output supported");
            check(storage.getLong(0) == 0x1122334455667788L, "Slice prefix guard intact");
            storage.limit(120);
            check(storage.getLong(112) == 0x1122334455667788L, "Slice suffix guard intact");
            for (long invalid : new long[] {0, -1, Long.MAX_VALUE}) {
                check(NativeBindings.destroy(invalid) == -3, "Invalid handle destroy");
                check(NativeBindings.submit(invalid, input, 40) == -3, "Invalid handle submit");
                check(NativeBindings.renderFixture(invalid, 0, out, 104) == -3, "Invalid handle render");
            }
            // Valid resize followed by an invalid event must not apply the resize prefix.
            ByteBuffer atomic = ByteBuffer.allocateDirect(64).order(ByteOrder.LITTLE_ENDIAN);
            atomic.putInt(0x4e495a4f).putInt(1).putInt(2).putInt(24)
                    .putInt(1).putInt(1920).putInt(1080).putFloat(2).putFloat(0).putInt(0)
                    .putInt(99).putInt(0).putInt(0).putFloat(0).putFloat(0).putInt(0);
            NativeBindings.renderFixture(handle, 3, out, 104);
            byte[] before = snapshot(out);
            check(NativeBindings.submit(handle, atomic, 64) == -2, "Bad batch rejected atomically");
            NativeBindings.renderFixture(handle, 3, out, 104);
            check(Arrays.equals(before, snapshot(out)), "Rejected input did not change frame state");
        } finally {
            check(NativeBindings.destroy(handle) == 0, "Raw session cleanup");
        }
        check(NativeBindings.destroy(handle) == -3, "Destroyed handle stays invalid");
        long noFixture = NativeBindings.create(1, 0);
        check(NativeBindings.renderFixture(noFixture, 0, output(), 104) == -7, "Fixture requires explicit dev flag");
        check(NativeBindings.destroy(noFixture) == 0, "Non-fixture session cleanup");
        System.out.println("PASS raw JNI lengths/capacities/records/handles/transaction validation");
    }

    private static byte[] snapshot(ByteBuffer source) {
        byte[] result = new byte[source.capacity()];
        source.duplicate().clear().get(result);
        return result;
    }

    private static void deterministicFramesAndLeases() throws InterruptedException {
        EngineSession session = new EngineSession(1);
        InputBatch batch = new InputBatch(4).resize(1920, 1080, 1).pointerMove(12, 34).reducedMotion(true);
        session.submit(batch);
        FrameBuffer first = new FrameBuffer();
        var old = session.renderFixture(7, first);
        ByteBuffer oldBytes = old.bytes();
        check(oldBytes.isReadOnly(), "Frame view is read-only");
        check(oldBytes.getLong(16) == 7 && oldBytes.getInt(24) == 6, "Frame id and vertex count");
        check(oldBytes.getFloat(32) == 12 && oldBytes.getFloat(36) == 34, "Input reached Rust frame");
        check(oldBytes.getFloat(44) == 44 && oldBytes.getFloat(60) == 58, "Rectangle extent is exact");
        check(oldBytes.getInt(40) == 0xfff8f9ff, "Default fixture color");
        expectIllegalState(() -> session.renderFixture(8, first), "Leased buffer cannot be overwritten");
        session.submit(batch.clear().pointerButton(0, true));
        try (var current = session.renderFixture(8, new FrameBuffer())) {
            check(current.bytes().getInt(40) == 0xff171923, "Button event changes fixture");
        }
        session.close();
        session.close();
        check(old.bytes().getInt(40) == 0xfff8f9ff, "Deferred frame survives next frame and engine shutdown");
        AtomicInteger consumed = new AtomicInteger();
        Thread consumer = Thread.ofPlatform().start(() -> {
            consumed.set(old.bytes().getInt(40));
            old.close();
        });
        consumer.join();
        check(consumed.get() == 0xfff8f9ff, "Deferred consumer can read and release on another thread");
        old.close();
        expectIllegalState(old::bytes, "Closed lease cannot be reacquired");
        expectIllegalState(() -> session.submit(batch), "Closed engine rejects input");
        try (EngineSession next = new EngineSession(1); var recycled = next.renderFixture(9, first)) {
            check(recycled.bytes().getLong(16) == 9, "Released buffer can be reused");
        }
        try (EngineSession disabled = new EngineSession(0)) {
            boolean rejected = false;
            try {
                disabled.renderFixture(10, first);
            } catch (BridgeException expected) {
                rejected = expected.status() == BridgeProtocol.UNSUPPORTED;
            }
            check(rejected, "Controlled native failure reaches the Java adapter");
        }
        try (EngineSession recovered = new EngineSession(1); var frame = recovered.renderFixture(11, first)) {
            check(frame.bytes().getLong(16) == 11, "Failed render releases the output buffer for recovery");
        }
        System.out.println("PASS deterministic frame fixture and deferred buffer leases");
    }

    private static void threadConfinement() throws InterruptedException {
        long raw = NativeBindings.create(1, 1);
        AtomicInteger nativeStatus = new AtomicInteger();
        Thread worker = Thread.ofPlatform().start(() -> nativeStatus.set(NativeBindings.destroy(raw)));
        worker.join();
        check(nativeStatus.get() == -5, "Native owner-thread rule enforced");
        check(NativeBindings.destroy(raw) == 0, "Owner can still destroy session");
        try (EngineSession session = new EngineSession(1)) {
            InputBatch batch = new InputBatch(1);
            AtomicInteger rejected = new AtomicInteger();
            Thread second = Thread.ofPlatform().start(() -> {
                try {
                    session.submit(batch);
                } catch (IllegalStateException expected) {
                    rejected.incrementAndGet();
                }
                try {
                    batch.clear();
                } catch (IllegalStateException expected) {
                    rejected.incrementAndGet();
                }
            });
            second.join();
            check(rejected.get() == 2, "Java session and encoder are thread confined");
        }
        AtomicInteger virtualRejected = new AtomicInteger();
        Thread virtual = Thread.ofVirtual().start(() -> {
            try (EngineSession ignored = new EngineSession(1)) {
                ignored.submit(new InputBatch(1));
                virtualRejected.set(-1);
            } catch (IllegalStateException expected) {
                virtualRejected.set(1);
            }
        });
        virtual.join();
        check(virtualRejected.get() == 1, "Virtual-thread sessions rejected");
        System.out.println("PASS Java/native thread confinement");
    }

    private static void lifecycleAndLimits() {
        long[] handles = new long[64];
        for (int i = 0; i < handles.length; i++) {
            handles[i] = NativeBindings.create(1, 0);
            check(handles[i] > 0, "Session limit admits slot " + i);
        }
        check(NativeBindings.create(1, 0) == -8, "Session limit prevents unbounded registry growth");
        for (long handle : handles) {
            check(NativeBindings.destroy(handle) == 0, "Limited session cleanup");
        }
        long previous = handles[63];
        for (int i = 0; i < 10000; i++) {
            long handle = NativeBindings.create(1, 0);
            check(handle > previous, "Handles never alias a destroyed session");
            check(NativeBindings.destroy(handle) == 0, "Repeated lifecycle cleanup");
            previous = handle;
        }
        check(NativeBindings.destroy(handles[0]) == -3, "Old handle is still stale after reuse cycles");
        System.out.println("PASS 10,000 create/destroy cycles and bounded session registry");
    }

    private static void expectIllegalState(Runnable action, String label) {
        boolean rejected = false;
        try {
            action.run();
        } catch (IllegalStateException expected) {
            rejected = true;
        }
        check(rejected, label);
    }

    private static void check(boolean condition, String message) {
        checks++;
        if (!condition) {
            throw new AssertionError(message);
        }
    }
}
