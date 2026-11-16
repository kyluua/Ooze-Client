package io.github.kyluua.ooze.mc.v1_21_11.diag;

import io.github.kyluua.ooze.mc.v1_21_11.engine.EngineStatus;
import java.util.Locale;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.function.Supplier;
import net.fabricmc.fabric.api.client.screen.v1.ScreenEvents;
import net.minecraft.client.gui.screens.TitleScreen;
import org.slf4j.Logger;

/**
 * Logs one machine-readable line the first time the vanilla title screen opens, so the Ooze
 * Launcher can tell "the Java process started" apart from "Minecraft actually reached its title
 * screen" (docs/launcher/LAUNCHER_UI.md §6, request to Codex in docs/handoffs/CLAUDE-L0-request.md).
 *
 * <p>Format (stable, versioned): {@code OOZE_READY v1 screen=title engine=<STATE>}. The line
 * carries no account, session or path data. It is logged in vanilla-only mode too: it is a log
 * line, not an Ooze control.
 */
public final class LaunchReadiness {

    public static final String MARKER = "OOZE_READY v1";

    private static final AtomicBoolean logged = new AtomicBoolean();

    private LaunchReadiness() {}

    public static String line(EngineStatus.State state) {
        return MARKER + " screen=title engine=" + state.name().toUpperCase(Locale.ROOT);
    }

    /** {@code state} is read when the title screen opens (the engine may be disabled by then). */
    public static void register(Logger log, Supplier<EngineStatus> state) {
        ScreenEvents.AFTER_INIT.register((client, screen, width, height) -> {
            if (screen instanceof TitleScreen && logged.compareAndSet(false, true)) {
                log.info(line(state.get().state()));
            }
        });
    }

    /** True once the marker was logged (used by launch tests). */
    public static boolean wasLogged() {
        return logged.get();
    }
}
