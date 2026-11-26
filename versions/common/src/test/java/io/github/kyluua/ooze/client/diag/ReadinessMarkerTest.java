package io.github.kyluua.ooze.client.diag;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.github.kyluua.ooze.client.engine.EngineStatus;
import java.util.regex.Pattern;
import org.junit.jupiter.api.Test;

/** The launcher parses this line from the game's output; its format is a contract. */
class ReadinessMarkerTest {

    private static final Pattern CONTRACT = Pattern.compile("^OOZE_READY v1 screen=title engine=[A-Z_]+$");

    @Test
    void lineMatchesTheLauncherContractForEveryState() {
        for (EngineStatus.State s : EngineStatus.State.values()) {
            String line = ReadinessMarker.line(s);
            assertTrue(CONTRACT.matcher(line).matches(), line);
        }
        assertEquals("OOZE_READY v1 screen=title engine=LOADED", ReadinessMarker.line(EngineStatus.State.LOADED));
    }
}
