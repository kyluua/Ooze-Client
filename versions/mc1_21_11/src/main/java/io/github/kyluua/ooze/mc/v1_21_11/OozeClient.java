package io.github.kyluua.ooze.mc.v1_21_11;

import io.github.kyluua.ooze.client.OozeRuntime;
import io.github.kyluua.ooze.mc.v1_21_11.dev.DeveloperProbe;
import io.github.kyluua.ooze.mc.v1_21_11.diag.LaunchReadiness;
import io.github.kyluua.ooze.mc.v1_21_11.diag.UnavailableNotice;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientLifecycleEvents;
import net.fabricmc.loader.api.FabricLoader;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Ooze Client Fabric entrypoint for Minecraft 1.21.11.
 *
 * <p>The engine state lives in the shared {@link OozeRuntime}; this adapter only registers the
 * 1.21.11 hooks. Without a usable engine, Minecraft stays vanilla and shows one notice toast.
 * Product UI arrives in later milestones (docs/IMPLEMENTATION_PLAN.md).
 */
public final class OozeClient implements ClientModInitializer {

    public static final String MOD_ID = OozeRuntime.MOD_ID;
    private static final Logger LOG = LoggerFactory.getLogger("OozeClient");

    @Override
    public void onInitializeClient() {
        long start = System.nanoTime();
        FabricLoader loader = FabricLoader.getInstance();
        boolean dev = loader.isDevelopmentEnvironment() || Boolean.getBoolean("oozeclient.dev");
        String version = loader.getModContainer(MOD_ID)
                .map(c -> c.getMetadata().getVersion().getFriendlyString())
                .orElse("unknown");
        long loadMicros = OozeRuntime.start(loader.getGameDir(), dev, version, LOG);

        if (OozeRuntime.gate().showUnavailableNotice()) {
            UnavailableNotice.register(OozeRuntime.engineStatus());
        }
        // Readiness marker for the Ooze Launcher (title screen reached), in every engine state.
        LaunchReadiness.register(LOG);
        if (OozeRuntime.gate().developerProbeEnabled()) {
            DeveloperProbe.register(LOG);
        }
        // Product features (screens, key mappings, HUD) register here from C1 onwards, behind
        // OozeRuntime.gate().oozeFeaturesEnabled().

        ClientLifecycleEvents.CLIENT_STOPPING.register(client -> OozeRuntime.stop());
        LOG.info("Ooze Client bootstrap: {} µs total, engine connect {} µs, dev={}, state={}",
                (System.nanoTime() - start) / 1_000, loadMicros, dev, OozeRuntime.engineStatus().state());
    }
}
