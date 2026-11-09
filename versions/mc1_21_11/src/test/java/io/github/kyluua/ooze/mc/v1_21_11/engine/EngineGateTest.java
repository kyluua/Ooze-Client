package io.github.kyluua.ooze.mc.v1_21_11.engine;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.EnumSource;

class EngineGateTest {

    @ParameterizedTest
    @EnumSource(value = EngineStatus.State.class, names = "LOADED", mode = EnumSource.Mode.EXCLUDE)
    void unavailableEngineRegistersNothingAndShowsNotice(EngineStatus.State state) {
        for (boolean dev : new boolean[] {false, true}) {
            EngineGate gate = new EngineGate(EngineStatus.of(state, "r", "d"), dev);
            assertFalse(gate.oozeFeaturesEnabled(), state + " must not enable Ooze features");
            assertFalse(gate.developerProbeEnabled(), state + " must not expose the developer probe");
            assertTrue(gate.showUnavailableNotice(), state + " must show the diagnostic notice");
        }
    }

    @Test
    void loadedEngineEnablesFeaturesAndProbeOnlyInDev() {
        EngineStatus loaded = new EngineStatus(EngineStatus.State.LOADED, "1.0.0", "ok", "");
        EngineGate release = new EngineGate(loaded, false);
        EngineGate dev = new EngineGate(loaded, true);
        assertTrue(release.oozeFeaturesEnabled());
        assertFalse(release.developerProbeEnabled());
        assertFalse(release.showUnavailableNotice());
        assertTrue(dev.developerProbeEnabled());
    }

    @Test
    void keySuffixIsLowerSnakeCase() {
        assertEquals("missing_library", EngineStatus.State.MISSING_LIBRARY.keySuffix());
    }
}
