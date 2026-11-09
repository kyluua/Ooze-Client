package io.github.kyluua.ooze.mc.v1_21_11.engine;

import java.util.Objects;
import org.jspecify.annotations.Nullable;

/**
 * Result of connecting the Fabric adapter to the Ooze native engine (TARGET_PLATFORM §5).
 *
 * @param state what happened
 * @param bridgeVersion bridge/protocol version reported by the loaded library, if any
 * @param reason one-line human explanation (English, for logs; UI uses translation keys)
 * @param detail technical detail for the log (exception, expected/actual versions, paths)
 */
public record EngineStatus(State state, @Nullable String bridgeVersion, String reason, String detail) {

    public enum State {
        /** Bridge and native library loaded with a compatible version. */
        LOADED,
        /** No native library (or its manifest) is bundled for this build. */
        MISSING_LIBRARY,
        /** OS/architecture is not a supported native platform yet. */
        UNSUPPORTED_PLATFORM,
        /** Manifest, version, ABI, capabilities or checksum do not match this mod. */
        INCOMPATIBLE,
        /** Loading threw, or the engine was disabled after a runtime bridge failure. */
        LOAD_FAILED;

        /** Translation-key suffix, e.g. {@code missing_library}. */
        public String keySuffix() {
            return name().toLowerCase(java.util.Locale.ROOT);
        }
    }

    public EngineStatus {
        Objects.requireNonNull(state, "state");
        reason = Objects.requireNonNullElse(reason, "");
        detail = Objects.requireNonNullElse(detail, "");
    }

    public boolean isLoaded() {
        return state == State.LOADED;
    }

    public static EngineStatus of(State state, String reason, String detail) {
        return new EngineStatus(state, null, reason, detail);
    }

    /**
     * Status after a controlled bridge failure at runtime: the engine is disabled for the rest of
     * the session (no further native calls), keeping the bridge version for the log.
     */
    public static EngineStatus runtimeFailure(EngineStatus previous, String where, Throwable cause) {
        return new EngineStatus(State.LOAD_FAILED, previous.bridgeVersion(),
                "The Ooze native engine was disabled after a failure in " + where + ".",
                cause.getClass().getName() + ": " + cause.getMessage());
    }
}
