package io.github.kyluua.ooze.client.diag;

import io.github.kyluua.ooze.client.engine.EngineStatus;
import java.util.Locale;
import java.util.concurrent.atomic.AtomicBoolean;

/**
 * The launcher readiness marker, shared by every Minecraft version adapter. Each adapter logs it
 * once, the first time the vanilla title screen opens, so the Ooze Launcher can tell "the Java
 * process started" apart from "Minecraft reached its title screen" (docs/launcher/LAUNCHER_UI.md
 * §6, docs/handoffs/CLAUDE-L0-request.md).
 *
 * <p>Format (stable, versioned): {@code OOZE_READY v1 screen=title engine=<STATE>}. It carries no
 * account, session or path data, and it is logged in vanilla-only mode too: it is a log line, not
 * an Ooze control. The format does not name the Minecraft version; the launcher knows the target.
 */
public final class ReadinessMarker {

    public static final String MARKER = "OOZE_READY v1";

    private static final AtomicBoolean logged = new AtomicBoolean();

    private ReadinessMarker() {}

    public static String line(EngineStatus.State state) {
        return MARKER + " screen=title engine=" + state.name().toUpperCase(Locale.ROOT);
    }

    /** Claims the single log slot; true only for the first caller. */
    public static boolean claim() {
        return logged.compareAndSet(false, true);
    }

    /** True once the marker was logged (used by launch tests). */
    public static boolean wasLogged() {
        return logged.get();
    }
}
