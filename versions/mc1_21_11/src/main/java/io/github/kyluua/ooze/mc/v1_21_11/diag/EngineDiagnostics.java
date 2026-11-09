package io.github.kyluua.ooze.mc.v1_21_11.diag;

import io.github.kyluua.ooze.mc.v1_21_11.engine.EngineStatus;
import io.github.kyluua.ooze.mc.v1_21_11.engine.PlatformInfo;
import java.util.ArrayList;
import java.util.List;

/**
 * Minecraft-independent formatting of the engine diagnostic: the log block and the translation
 * keys/arguments for the in-game notice. Keeping it free of Minecraft types makes it unit-testable.
 */
public final class EngineDiagnostics {

    public static final String KEY_TITLE = "oozeclient.engine.unavailable.title";
    public static final String KEY_BODY_PREFIX = "oozeclient.engine.unavailable.";

    private EngineDiagnostics() {}

    /** Multi-line log block. Never contains account or session data. */
    public static List<String> logLines(EngineStatus status, PlatformInfo platform, String modVersion) {
        List<String> lines = new ArrayList<>();
        if (status.isLoaded()) {
            lines.add("Ooze Client " + modVersion + ": native engine loaded (bridge " + status.bridgeVersion()
                    + ", platform " + platform.id() + ").");
            return lines;
        }
        lines.add("Ooze Client " + modVersion + " is OFF: " + status.reason());
        lines.add("  state    : " + status.state());
        lines.add("  platform : " + platform.id()
                + (platform.isSupportedNativePlatform() ? " (supported native platform)" : " (no supported native build yet)"));
        if (status.bridgeVersion() != null) {
            lines.add("  bridge   : " + status.bridgeVersion());
        }
        if (!status.detail().isBlank()) {
            lines.add("  detail   : " + status.detail());
        }
        lines.add("  effect   : Minecraft runs as vanilla. No Ooze screens, key mappings or HUD were registered.");
        lines.add("  next step: " + remedy(status.state(), platform));
        return lines;
    }

    /** Translation key for the notice body. */
    public static String bodyKey(EngineStatus status) {
        return KEY_BODY_PREFIX + status.state().keySuffix();
    }

    /** Arguments for the notice body (platform id where the message names it). */
    public static Object[] bodyArgs(EngineStatus status, PlatformInfo platform) {
        return switch (status.state()) {
            case MISSING_LIBRARY, UNSUPPORTED_PLATFORM -> new Object[] {platform.id()};
            default -> new Object[0];
        };
    }

    static String remedy(EngineStatus.State state, PlatformInfo platform) {
        return switch (state) {
            case LOADED -> "none";
            case NOT_BUNDLED -> "Install a release build of Ooze Client that bundles the native engine.";
            case MISSING_LIBRARY -> "Reinstall Ooze Client; the native library for " + platform.id() + " is missing.";
            case UNSUPPORTED_PLATFORM -> "Use Windows x64, the first supported platform, or keep playing vanilla.";
            case INCOMPATIBLE -> "Reinstall Ooze Client so the mod and native library come from the same release.";
            case LOAD_FAILED -> "Check the detail above; reinstall Ooze Client and make sure security software is not blocking it.";
        };
    }
}
