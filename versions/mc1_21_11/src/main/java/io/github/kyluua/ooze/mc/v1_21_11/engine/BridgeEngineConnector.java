package io.github.kyluua.ooze.mc.v1_21_11.engine;

import io.github.kyluua.ooze.bridge.BridgeProtocol;
import io.github.kyluua.ooze.bridge.EngineSession;
import io.github.kyluua.ooze.bridge.FrameBuffer;
import io.github.kyluua.ooze.bridge.InputBatch;
import io.github.kyluua.ooze.bridge.NativeLoader;
import java.nio.ByteBuffer;
import java.nio.file.Path;

/**
 * The single adapter over Sol's bridge-java G0 API (docs/bridge/ABI_V1.md). It adds no native
 * methods and no protocol of its own. All calls must happen on Minecraft's render (platform)
 * thread, which owns every {@link EngineSession} it creates.
 */
public final class BridgeEngineConnector implements EngineConnector {

    /** Bridge identity reported in logs and the About page. */
    public static final String BRIDGE_VERSION = BridgeProtocol.LIBRARY_VERSION + " / ABI " + BridgeProtocol.ABI_VERSION;

    private static final int FIXTURE_PRESSED_ARGB = 0xFF171923;

    private final Path cacheRoot;
    private final PlatformInfo platform;
    private boolean loaded;

    public BridgeEngineConnector(Path cacheRoot, PlatformInfo platform) {
        this.cacheRoot = cacheRoot;
        this.platform = platform;
    }

    @Override
    public EngineStatus connect() {
        NativeLoader.LoadResult result = NativeLoader.tryLoad(cacheRoot);
        EngineStatus status = BridgeStatusMapper.map(
                result.available(), result.diagnostic(), result.capabilities(), BRIDGE_VERSION, platform);
        loaded = status.isLoaded();
        return status;
    }

    /**
     * One synchronous round trip: create a DEV_FIXTURE session, submit a resize + pointer move +
     * button press batch, render the fixture, copy and check it, release the lease, destroy.
     */
    @Override
    public DevProbeResult developerProbe() {
        if (!loaded) {
            throw new IllegalStateException("The native engine is not loaded.");
        }
        long start = System.nanoTime();
        FrameBuffer frame = new FrameBuffer();
        try (EngineSession session = new EngineSession(BridgeProtocol.DEV_FIXTURE)) {
            InputBatch batch = new InputBatch(4)
                    .resize(1920, 1080, 1.0f)
                    .pointerMove(100f, 50f)
                    .pointerButton(0, true);
            session.submit(batch);
            float x0;
            float y0;
            int argb;
            try (FrameBuffer.FrameLease lease = session.renderFixture(1L, frame)) {
                ByteBuffer b = lease.bytes();
                x0 = b.getFloat(32);
                y0 = b.getFloat(36);
                argb = b.getInt(40);
            }
            long elapsed = System.nanoTime() - start;
            boolean ok = x0 == 100f && y0 == 50f && argb == FIXTURE_PRESSED_ARGB;
            return new DevProbeResult(ok, String.format(java.util.Locale.ROOT,
                    "bridge %s: fixture top-left=(%.0f,%.0f) argb=%08X (expected 100,50 / %08X)",
                    BRIDGE_VERSION, x0, y0, argb, FIXTURE_PRESSED_ARGB), elapsed);
        }
    }

    public boolean isLoaded() {
        return loaded;
    }

    @Override
    public void close() {
        // Sessions are owned and closed by their users (probe, diagnostic screen). A loaded JNI
        // library cannot be unloaded from this class loader; nothing else to release.
    }
}
