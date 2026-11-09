package io.github.kyluua.ooze.mc.v1_21_11.engine;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.github.kyluua.ooze.bridge.NativeLoader.LoadResult;
import io.github.kyluua.ooze.bridge.NativeLoader.LoadState;
import java.util.Arrays;
import java.util.EnumSet;
import java.util.Set;
import java.util.stream.Collectors;
import org.junit.jupiter.api.Test;

/** One-to-one mapping of Sol's typed {@code NativeLoader.LoadState} (no text matching). */
class BridgeStatusMapperTest {

    private static final PlatformInfo WIN = PlatformInfo.of("Windows 11", "amd64");

    @Test
    void everyLoaderStateMapsToTheSameNamedAdapterState() {
        for (LoadState s : LoadState.values()) {
            EngineStatus status = BridgeStatusMapper.map(new LoadResult(s, "diag " + s, s == LoadState.LOADED ? 3 : 0), WIN);
            assertEquals(s.name(), status.state().name(), "mapping for " + s);
            assertTrue(status.detail().contains("diag " + s) || s == LoadState.LOADED, "loader text kept for the log");
        }
    }

    @Test
    void adapterStatesAreExactlyTheLoaderStates() {
        Set<String> loader = Arrays.stream(LoadState.values()).map(Enum::name).collect(Collectors.toSet());
        Set<String> adapter = EnumSet.allOf(EngineStatus.State.class).stream().map(Enum::name).collect(Collectors.toSet());
        assertEquals(loader, adapter, "adapter states must track Sol's LoadState exactly");
    }

    @Test
    void loadedCarriesBridgeAndProtocolVersion() {
        EngineStatus s = BridgeStatusMapper.map(new LoadResult(LoadState.LOADED, "Ooze native 0.1.0-g0 / ABI 1 / windows-x86_64", 3), WIN);
        assertTrue(s.isLoaded());
        assertEquals("0.1.0-g0 / ABI 1", s.bridgeVersion());
        assertEquals("capabilities=3", s.detail());
    }

    @Test
    void failuresCarryNoBridgeVersionButKeepItInTheDetail() {
        EngineStatus s = BridgeStatusMapper.map(new LoadResult(LoadState.INCOMPATIBLE, "Native SHA-256 mismatch", 0), WIN);
        assertFalse(s.isLoaded());
        assertNull(s.bridgeVersion());
        assertTrue(s.detail().startsWith("bridge 0.1.0-g0 / ABI 1: "));
    }

    @Test
    void runtimeFailureDisablesAndClosesTheGate() {
        EngineStatus loaded = BridgeStatusMapper.map(new LoadResult(LoadState.LOADED, "ok", 3), WIN);
        EngineStatus failed = EngineStatus.runtimeFailure(loaded, "the developer probe", new IllegalStateException("status -6"));
        assertEquals(EngineStatus.State.LOAD_FAILED, failed.state());
        assertEquals(loaded.bridgeVersion(), failed.bridgeVersion());
        EngineGate gate = new EngineGate(failed, true);
        assertFalse(gate.oozeFeaturesEnabled());
        assertFalse(gate.developerProbeEnabled());
        assertTrue(failed.detail().contains("status -6"));
    }
}
