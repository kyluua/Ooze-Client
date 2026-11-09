package io.github.kyluua.ooze.mc.v1_21_11.engine;

import java.nio.file.Path;
import java.util.Locale;
import java.util.function.Consumer;
import org.jspecify.annotations.Nullable;

/** Chooses the {@link EngineConnector} for this run and connects it without ever throwing. */
public final class EngineConnectors {

    /** System property for developer-only fault injection; ignored outside the dev environment. */
    public static final String FAULT_PROPERTY = "oozeclient.dev.engineFault";

    private EngineConnectors() {}

    /**
     * @param devEnvironment true in Loom dev runs or with {@code -Doozeclient.dev=true}
     * @param faultValue value of {@link #FAULT_PROPERTY}, or null
     * @param warn sink for configuration warnings (e.g. an unknown fault value)
     */
    public static EngineConnector select(
            PlatformInfo platform,
            Path nativeCacheRoot,
            boolean devEnvironment,
            @Nullable String faultValue,
            Consumer<String> warn) {
        if (faultValue != null && !faultValue.isBlank()) {
            if (!devEnvironment) {
                warn.accept("Ignoring -D" + FAULT_PROPERTY + " outside the development environment.");
            } else {
                EngineStatus.State fault = parseFault(faultValue);
                if (fault == null) {
                    warn.accept("Unknown -D" + FAULT_PROPERTY + "=" + faultValue
                            + " (use missing_library, unsupported_platform, incompatible or load_failed).");
                } else {
                    return new FaultInjectedEngineConnector(fault, platform);
                }
            }
        }
        // Sol's bridge-java (G0) is always compiled in; whether its native library is present,
        // compatible and loadable on this platform is decided by NativeLoader at connect().
        return new BridgeEngineConnector(nativeCacheRoot, platform);
    }

    /**
     * Connects, converting anticipated failures (runtime exceptions, linkage errors) into
     * {@link EngineStatus.State#LOAD_FAILED}. VM-fatal errors ({@link VirtualMachineError},
     * {@link ThreadDeath}) are not swallowed (Sol review, finding 4).
     */
    public static EngineStatus connectSafely(EngineConnector connector) {
        try {
            EngineStatus status = connector.connect();
            return status != null
                    ? status
                    : EngineStatus.of(EngineStatus.State.LOAD_FAILED, "The Ooze engine connector returned no status.", connector.getClass().getName());
        } catch (RuntimeException | LinkageError t) {
            return EngineStatus.of(
                    EngineStatus.State.LOAD_FAILED,
                    "The Ooze native library failed to load.",
                    t.getClass().getName() + ": " + t.getMessage());
        }
    }

    static EngineStatus.@Nullable State parseFault(String value) {
        String v = value.trim().toUpperCase(Locale.ROOT).replace('-', '_');
        if (v.equals("MISSING")) {
            v = "MISSING_LIBRARY";
        }
        try {
            EngineStatus.State s = EngineStatus.State.valueOf(v);
            return s == EngineStatus.State.LOADED ? null : s;
        } catch (IllegalArgumentException e) {
            return null;
        }
    }
}
