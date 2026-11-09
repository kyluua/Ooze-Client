package io.github.kyluua.ooze.mc.v1_21_11.engine;

/**
 * Single decision point for what the Fabric adapter may register, given the engine status.
 * "Real or absent": nothing Ooze-branded and interactive exists unless the engine is loaded.
 */
public record EngineGate(EngineStatus status, boolean devEnvironment) {

    /** Ooze product features (screens, key mappings, HUD, title-screen integration) may register. */
    public boolean oozeFeaturesEnabled() {
        return status.isLoaded();
    }

    /** The developer-only bridge probe may register (dev environment + loaded engine). */
    public boolean developerProbeEnabled() {
        return devEnvironment && status.isLoaded();
    }

    /** Show the one-time "Ooze Client is off" vanilla toast on the title screen. */
    public boolean showUnavailableNotice() {
        return !status.isLoaded();
    }
}
