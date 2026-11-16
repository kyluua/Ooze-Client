package io.github.kyluua.ooze.mc.v1_21_11.diag;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.github.kyluua.ooze.mc.v1_21_11.engine.EngineStatus;
import java.util.regex.Pattern;
import org.junit.jupiter.api.Test;

/** The launcher parses this line from the game's output; its format is a contract. */
class LaunchReadinessTest {

    private static final Pattern CONTRACT = Pattern.compile("^OOZE_READY v1 screen=title engine=[A-Z_]+$");

    @Test
    void lineMatchesTheLauncherContractForEveryState() {
        for (EngineStatus.State s : EngineStatus.State.values()) {
            String line = LaunchReadiness.line(s);
            assertTrue(CONTRACT.matcher(line).matches(), line);
        }
        assertEquals("OOZE_READY v1 screen=title engine=LOADED", LaunchReadiness.line(EngineStatus.State.LOADED));
    }
}
