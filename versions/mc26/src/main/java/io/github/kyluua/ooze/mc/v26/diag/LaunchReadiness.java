package io.github.kyluua.ooze.mc.v26.diag;

import io.github.kyluua.ooze.client.OozeRuntime;
import io.github.kyluua.ooze.client.diag.ReadinessMarker;
import net.fabricmc.fabric.api.client.screen.v1.ScreenEvents;
import net.minecraft.client.gui.screens.TitleScreen;
import org.slf4j.Logger;

/**
 * Logs the shared launcher readiness marker ({@link ReadinessMarker}) the first time the vanilla
 * title screen opens. The engine state is read at that moment (it may be disabled by then).
 */
public final class LaunchReadiness {

    private LaunchReadiness() {}

    public static void register(Logger log) {
        ScreenEvents.AFTER_INIT.register((client, screen, width, height) -> {
            if (screen instanceof TitleScreen && ReadinessMarker.claim()) {
                log.info(ReadinessMarker.line(OozeRuntime.engineStatus().state()));
            }
        });
    }
}
