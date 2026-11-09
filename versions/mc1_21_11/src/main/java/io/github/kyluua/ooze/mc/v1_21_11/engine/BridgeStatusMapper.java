package io.github.kyluua.ooze.mc.v1_21_11.engine;

import io.github.kyluua.ooze.bridge.NativeLoader;

/**
 * Maps Sol's typed {@link NativeLoader.LoadResult} (G0 follow-up 190869b) one-to-one onto the
 * adapter's {@link EngineStatus}. No diagnostic-text matching: the loader's {@code LoadState} is
 * authoritative (Sol review, finding 2). The loader's diagnostic text goes to the log detail.
 */
public final class BridgeStatusMapper {

    private BridgeStatusMapper() {}

    public static EngineStatus map(NativeLoader.LoadResult result, PlatformInfo platform) {
        String version = result.bridgeVersion() + " / ABI " + result.protocolVersion();
        String detail = "bridge " + version + ": " + result.diagnostic();
        return switch (result.state()) {
            case LOADED -> new EngineStatus(
                    EngineStatus.State.LOADED, version, result.diagnostic(), "capabilities=" + result.capabilities());
            case UNSUPPORTED_PLATFORM -> new EngineStatus(EngineStatus.State.UNSUPPORTED_PLATFORM, null,
                    platform.id() + " is not a supported Ooze native platform yet.", detail);
            case MISSING_LIBRARY -> new EngineStatus(EngineStatus.State.MISSING_LIBRARY, null,
                    "The Ooze native library for " + platform.id() + " is missing from this build.", detail);
            case INCOMPATIBLE -> new EngineStatus(EngineStatus.State.INCOMPATIBLE, null,
                    "The Ooze native library does not match this mod (version, ABI or checksum).", detail);
            case LOAD_FAILED -> new EngineStatus(EngineStatus.State.LOAD_FAILED, null,
                    "The Ooze native library failed to load.", detail);
        };
    }
}
