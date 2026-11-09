package io.github.kyluua.ooze.mc.v1_21_11;

import io.github.kyluua.ooze.mc.v1_21_11.dev.DeveloperProbe;
import io.github.kyluua.ooze.mc.v1_21_11.diag.EngineDiagnostics;
import io.github.kyluua.ooze.mc.v1_21_11.diag.UnavailableNotice;
import io.github.kyluua.ooze.mc.v1_21_11.engine.EngineConnector;
import io.github.kyluua.ooze.mc.v1_21_11.engine.EngineConnectors;
import io.github.kyluua.ooze.mc.v1_21_11.engine.EngineGate;
import io.github.kyluua.ooze.mc.v1_21_11.engine.EngineStatus;
import io.github.kyluua.ooze.mc.v1_21_11.engine.PlatformInfo;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientLifecycleEvents;
import net.fabricmc.loader.api.FabricLoader;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Ooze Client Fabric entrypoint for Minecraft 1.21.11.
 *
 * <p>C0 scope: load Sol's native bridge through {@link EngineConnector}, log a clear diagnostic,
 * and register Ooze features only when the engine is loaded. Without a usable engine, Minecraft
 * stays vanilla and shows one notice toast. Product UI arrives in later milestones
 * (docs/IMPLEMENTATION_PLAN.md).
 */
public final class OozeClient implements ClientModInitializer {

    public static final String MOD_ID = "oozeclient";
    private static final Logger LOG = LoggerFactory.getLogger("OozeClient");

    private static volatile EngineStatus engineStatus =
            EngineStatus.of(EngineStatus.State.NOT_BUNDLED, "Not initialised yet.", "");
    private static volatile EngineGate gate = new EngineGate(engineStatus, false);
    private static volatile EngineConnector connector;

    @Override
    public void onInitializeClient() {
        long start = System.nanoTime();
        FabricLoader loader = FabricLoader.getInstance();
        boolean dev = loader.isDevelopmentEnvironment() || Boolean.getBoolean("oozeclient.dev");
        String version = loader.getModContainer(MOD_ID)
                .map(c -> c.getMetadata().getVersion().getFriendlyString())
                .orElse("unknown");
        PlatformInfo platform = PlatformInfo.current();

        // Version/hash-specific extraction cache for Sol's NativeLoader (never the temp-dir root).
        java.nio.file.Path nativeCache = loader.getGameDir().resolve(".ooze").resolve("natives");
        EngineConnector connector = EngineConnectors.select(
                platform, nativeCache, dev, System.getProperty(EngineConnectors.FAULT_PROPERTY), LOG::warn);
        OozeClient.connector = connector;
        long loadStart = System.nanoTime();
        EngineStatus status = EngineConnectors.connectSafely(connector);
        long loadMicros = (System.nanoTime() - loadStart) / 1_000;
        engineStatus = status;
        gate = new EngineGate(status, dev);

        for (String line : EngineDiagnostics.logLines(status, platform, version)) {
            if (status.isLoaded()) {
                LOG.info(line);
            } else {
                LOG.warn(line);
            }
        }

        if (gate.showUnavailableNotice()) {
            UnavailableNotice.register(status, platform);
        }
        if (gate.developerProbeEnabled()) {
            DeveloperProbe.register(connector, LOG);
        }
        // Product features (screens, key mappings, HUD) register here from C1 onwards, behind
        // gate.oozeFeaturesEnabled().

        ClientLifecycleEvents.CLIENT_STOPPING.register(client -> connector.close());
        LOG.info("Ooze Client bootstrap: {} µs total, engine connect {} µs, dev={}, state={}",
                (System.nanoTime() - start) / 1_000, loadMicros, dev, status.state());
    }

    /** Current engine status (read by launch tests and later milestones). */
    public static EngineStatus engineStatus() {
        return engineStatus;
    }

    public static EngineGate gate() {
        return gate;
    }

    /** The connector chosen at start-up (read by developer tools and launch tests). */
    public static EngineConnector connector() {
        return connector;
    }
}
