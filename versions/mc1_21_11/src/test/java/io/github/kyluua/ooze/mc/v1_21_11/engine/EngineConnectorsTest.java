package io.github.kyluua.ooze.mc.v1_21_11.engine;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import org.junit.jupiter.api.Test;

class EngineConnectorsTest {

    private static final PlatformInfo LINUX = PlatformInfo.of("Linux", "amd64");
    private static final Path CACHE = Path.of("build", "test-native-cache");

    @Test
    void defaultSelectionUsesSolsBridge() {
        List<String> warnings = new ArrayList<>();
        EngineConnector c = EngineConnectors.select(LINUX, CACHE, true, null, warnings::add);
        assertInstanceOf(BridgeEngineConnector.class, c);
        assertTrue(warnings.isEmpty());
    }

    @Test
    void faultInjectionOnlyInDevEnvironment() {
        List<String> warnings = new ArrayList<>();
        EngineConnector release = EngineConnectors.select(LINUX, CACHE, false, "incompatible", warnings::add);
        assertInstanceOf(BridgeEngineConnector.class, release);
        assertEquals(1, warnings.size());

        EngineConnector dev = EngineConnectors.select(LINUX, CACHE, true, "incompatible", warnings::add);
        EngineStatus s = EngineConnectors.connectSafely(dev);
        assertEquals(EngineStatus.State.INCOMPATIBLE, s.state());
        assertTrue(s.detail().contains("fault injection"), "injected states must say so in the log detail");
    }

    @Test
    void loadedCannotBeInjectedAndUnknownValuesWarn() {
        List<String> warnings = new ArrayList<>();
        assertInstanceOf(BridgeEngineConnector.class, EngineConnectors.select(LINUX, CACHE, true, "loaded", warnings::add));
        assertInstanceOf(BridgeEngineConnector.class, EngineConnectors.select(LINUX, CACHE, true, "banana", warnings::add));
        assertEquals(2, warnings.size());
        assertNull(EngineConnectors.parseFault("LOADED"));
        assertEquals(EngineStatus.State.MISSING_LIBRARY, EngineConnectors.parseFault("missing"));
        assertEquals(EngineStatus.State.LOAD_FAILED, EngineConnectors.parseFault("load-failed"));
    }

    @Test
    void everyFailureStateCanBeInjected() {
        for (EngineStatus.State st : EngineStatus.State.values()) {
            if (st == EngineStatus.State.LOADED) {
                assertThrows(IllegalArgumentException.class, () -> new FaultInjectedEngineConnector(st, LINUX));
                continue;
            }
            EngineStatus s = EngineConnectors.connectSafely(new FaultInjectedEngineConnector(st, LINUX));
            assertEquals(st, s.state());
            assertTrue(!s.reason().isBlank());
        }
    }

    @Test
    void throwingConnectorBecomesLoadFailed() {
        EngineConnector boom = new EngineConnector() {
            @Override
            public EngineStatus connect() {
                throw new UnsatisfiedLinkError("ooze_native.dll: The specified module could not be found");
            }

            @Override
            public DevProbeResult developerProbe() {
                throw new IllegalStateException();
            }

            @Override
            public void close() {}
        };
        EngineStatus s = EngineConnectors.connectSafely(boom);
        assertEquals(EngineStatus.State.LOAD_FAILED, s.state());
        assertTrue(s.detail().contains("UnsatisfiedLinkError"));
    }
}
