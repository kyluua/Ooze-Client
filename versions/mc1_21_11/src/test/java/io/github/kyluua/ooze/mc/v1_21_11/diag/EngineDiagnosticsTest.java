package io.github.kyluua.ooze.mc.v1_21_11.diag;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.github.kyluua.ooze.mc.v1_21_11.engine.EngineStatus;
import io.github.kyluua.ooze.mc.v1_21_11.engine.PlatformInfo;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import org.junit.jupiter.api.Test;

class EngineDiagnosticsTest {

    private static final PlatformInfo LINUX = PlatformInfo.of("Linux", "amd64");

    @Test
    void everyFailureStateHasAClearLogBlock() {
        for (EngineStatus.State st : EngineStatus.State.values()) {
            if (st == EngineStatus.State.LOADED) {
                continue;
            }
            List<String> lines = EngineDiagnostics.logLines(EngineStatus.of(st, "Reason " + st, "Detail " + st), LINUX, "1.2.3");
            String all = String.join("\n", lines);
            assertTrue(all.contains("is OFF"), all);
            assertTrue(all.contains("Reason " + st), all);
            assertTrue(all.contains("Detail " + st), all);
            assertTrue(all.contains("linux-x86_64"), all);
            assertTrue(all.contains("Minecraft runs as vanilla"), all);
            assertTrue(all.contains("next step:"), all);
        }
    }

    @Test
    void loadedStateLogsOneLine() {
        List<String> lines = EngineDiagnostics.logLines(
                new EngineStatus(EngineStatus.State.LOADED, "0.1.0", "ok", ""), PlatformInfo.of("Windows 11", "amd64"), "1.2.3");
        assertEquals(1, lines.size());
        assertTrue(lines.get(0).contains("bridge 0.1.0"));
    }

    @Test
    void everyStateHasEnglishTranslationsWithMatchingPlaceholders() throws IOException {
        String json = readLang();
        assertTrue(json.contains("\"" + EngineDiagnostics.KEY_TITLE + "\""));
        for (EngineStatus.State st : EngineStatus.State.values()) {
            if (st == EngineStatus.State.LOADED) {
                continue;
            }
            EngineStatus s = EngineStatus.of(st, "r", "d");
            String key = EngineDiagnostics.bodyKey(s);
            Matcher m = Pattern.compile("\"" + Pattern.quote(key) + "\"\\s*:\\s*\"([^\"]*)\"").matcher(json);
            assertTrue(m.find(), "missing translation for " + key);
            int placeholders = m.group(1).split("%s", -1).length - 1;
            assertEquals(EngineDiagnostics.bodyArgs(s, LINUX).length, placeholders, "placeholder count for " + key);
        }
    }

    @Test
    void diagnosticsNeverMentionAccountSecrets() {
        String all = String.join("\n", EngineDiagnostics.logLines(EngineStatus.of(EngineStatus.State.LOAD_FAILED, "r", "d"), LINUX, "1"));
        for (String forbidden : List.of("token", "session", "password", "xuid")) {
            assertFalse(all.toLowerCase().contains(forbidden), forbidden);
        }
    }

    private static String readLang() throws IOException {
        try (InputStream in = EngineDiagnosticsTest.class.getResourceAsStream("/assets/oozeclient/lang/en_us.json")) {
            assertNotNull(in, "en_us.json on the test classpath");
            return new String(in.readAllBytes(), StandardCharsets.UTF_8);
        }
    }
}
