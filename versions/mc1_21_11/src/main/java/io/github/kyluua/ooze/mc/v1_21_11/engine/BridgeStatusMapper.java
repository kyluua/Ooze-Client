package io.github.kyluua.ooze.mc.v1_21_11.engine;

import java.util.Locale;

/**
 * Maps Sol's {@code NativeLoader.LoadResult} (available flag + free-text diagnostic, bridge G0) to
 * the adapter's {@link EngineStatus} states (TARGET_PLATFORM §5).
 *
 * <p>G0 exposes no structured reason code, so classification uses the loader's documented
 * diagnostic phrases. This is deliberately isolated here and covered by tests against the exact
 * G0 strings; a structured reason has been requested from Sol (docs/handoffs/CLAUDE-C0.md §7).
 * Unknown text falls back to {@link EngineStatus.State#LOAD_FAILED}, never to LOADED.
 */
public final class BridgeStatusMapper {

    private BridgeStatusMapper() {}

    public static EngineStatus map(
            boolean available, String diagnostic, long capabilities, String bridgeVersion, PlatformInfo platform) {
        String text = diagnostic == null ? "" : diagnostic;
        if (available) {
            return new EngineStatus(EngineStatus.State.LOADED, bridgeVersion, text, "capabilities=" + capabilities);
        }
        String t = text.toLowerCase(Locale.ROOT);
        EngineStatus.State state;
        String reason;
        if (t.contains("limited to windows x64") || !platform.isSupportedNativePlatform()) {
            state = EngineStatus.State.UNSUPPORTED_PLATFORM;
            reason = platform.id() + " is not a supported Ooze native platform yet.";
        } else if (t.contains("missing bundled native resource") && t.contains("native.properties")) {
            state = EngineStatus.State.NOT_BUNDLED;
            reason = "This build of Ooze Client does not include the native engine.";
        } else if (t.contains("missing bundled native resource")) {
            state = EngineStatus.State.MISSING_LIBRARY;
            reason = "No Ooze native library was found for " + platform.id() + ".";
        } else if (t.contains("mismatch") || t.contains("incompatible") || t.contains("handshake")
                || t.contains("unsupported capabilities") || t.contains("test-only")
                || t.contains("already loaded")) {
            state = t.contains("sha-256 mismatch") ? EngineStatus.State.LOAD_FAILED : EngineStatus.State.INCOMPATIBLE;
            reason = state == EngineStatus.State.INCOMPATIBLE
                    ? "The Ooze native library version does not match this mod."
                    : "The Ooze native library is corrupt or does not match its manifest.";
        } else {
            state = EngineStatus.State.LOAD_FAILED;
            reason = "The Ooze native library failed to load.";
        }
        return new EngineStatus(state, null, reason, "bridge " + bridgeVersion + ": " + text);
    }
}
