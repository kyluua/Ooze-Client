package io.github.kyluua.ooze.client;

import io.github.kyluua.ooze.client.diag.EngineDiagnostics;
import io.github.kyluua.ooze.client.engine.BridgeEngineConnector;
import io.github.kyluua.ooze.client.engine.EngineConnector;
import io.github.kyluua.ooze.client.engine.EngineConnectors;
import io.github.kyluua.ooze.client.engine.EngineGate;
import io.github.kyluua.ooze.client.engine.EngineStatus;
import io.github.kyluua.ooze.client.engine.PlatformInfo;
import java.nio.file.Path;
import org.jspecify.annotations.Nullable;
import org.slf4j.Logger;

/**
 * Engine state of the running client, shared by every Minecraft version adapter. The adapter's
 * Fabric entrypoint calls {@link #start} once, then registers its Minecraft hooks according to
 * {@link #gate()}. Nothing here touches Minecraft or Fabric types.
 */
public final class OozeRuntime {

    public static final String MOD_ID = "oozeclient";

    private static volatile EngineStatus engineStatus =
            EngineStatus.of(EngineStatus.State.LOAD_FAILED, "Not initialised yet.", "");
    private static volatile boolean dev;
    private static volatile EngineGate gate = new EngineGate(engineStatus, false);
    private static volatile @Nullable EngineConnector connector;
    private static volatile @Nullable Logger log;

    private OozeRuntime() {}

    /**
     * Connects the native engine and logs the diagnostic block.
     *
     * @param gameDir the instance's game directory; the native extraction cache lives below it
     * @param devEnvironment Loom development run or {@code -Doozeclient.dev=true}
     * @param modVersion the mod's own version, for the log
     * @return the connect time in microseconds
     */
    public static long start(Path gameDir, boolean devEnvironment, String modVersion, Logger logger) {
        log = logger;
        dev = devEnvironment;
        PlatformInfo platform = PlatformInfo.current();
        // Version/hash-specific extraction cache for Sol's NativeLoader (never the temp-dir root).
        Path nativeCache = gameDir.resolve(".ooze").resolve("natives");
        EngineConnector selected = EngineConnectors.select(
                platform, nativeCache, devEnvironment, System.getProperty(EngineConnectors.FAULT_PROPERTY), logger::warn);
        connector = selected;
        long loadStart = System.nanoTime();
        EngineStatus status = EngineConnectors.connectSafely(selected);
        long loadMicros = (System.nanoTime() - loadStart) / 1_000;
        engineStatus = status;
        gate = new EngineGate(status, devEnvironment);
        for (String line : EngineDiagnostics.logLines(status, platform, modVersion)) {
            if (status.isLoaded()) {
                logger.info(line);
            } else {
                logger.warn(line);
            }
        }
        return loadMicros;
    }

    /** Current engine status (read by launch tests and later milestones). */
    public static EngineStatus engineStatus() {
        return engineStatus;
    }

    public static EngineGate gate() {
        return gate;
    }

    /** The connector chosen at start-up (read by developer tools and launch tests). */
    public static @Nullable EngineConnector connector() {
        return connector;
    }

    /** Closes the engine connector when the client stops. */
    public static void stop() {
        EngineConnector c = connector;
        if (c != null) {
            c.close();
        }
    }

    /**
     * Disables the native engine for the rest of the session after a controlled runtime bridge
     * failure (Sol review, finding 5): the gate closes, so nothing submits further native work.
     * Callers must already have closed their own sessions and leases. Render thread only.
     */
    public static void disableEngine(String where, Throwable cause) {
        if (!engineStatus.isLoaded()) {
            return;
        }
        engineStatus = EngineStatus.runtimeFailure(engineStatus, where, cause);
        gate = new EngineGate(engineStatus, dev);
        if (connector instanceof BridgeEngineConnector bridge) {
            bridge.disable();
        }
        Logger l = log;
        if (l != null) {
            l.error("Ooze native engine disabled after a failure in {}; Minecraft continues as vanilla.", where, cause);
        }
    }
}
