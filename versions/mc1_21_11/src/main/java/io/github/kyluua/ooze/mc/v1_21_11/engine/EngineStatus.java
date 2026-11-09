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
        /** This JAR contains no bridge or native library (C0 builds before Sol's G0 handoff). */
        NOT_BUNDLED,
        /** Bridge present but no native library for this OS/architecture, or the file is missing. */
        MISSING_LIBRARY,
        /** OS/architecture is not a supported native platform yet. */
        UNSUPPORTED_PLATFORM,
        /** Library loaded but its bridge or protocol version does not match. */
        INCOMPATIBLE,
        /** Loading threw (corrupt file, missing runtime, blocked by security software, ...). */
        LOAD_FAILED;

        /** Translation-key suffix, e.g. {@code not_bundled}. */
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
}
