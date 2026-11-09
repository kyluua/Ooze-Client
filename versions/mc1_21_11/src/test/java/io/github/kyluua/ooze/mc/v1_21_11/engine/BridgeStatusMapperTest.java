package io.github.kyluua.ooze.mc.v1_21_11.engine;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.github.kyluua.ooze.mc.v1_21_11.engine.EngineStatus.State;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.Test;

/** Classification of Sol's G0 {@code NativeLoader} diagnostics (bridge-java @ 0.1.0-g0). */
class BridgeStatusMapperTest {

    private static final PlatformInfo WIN = PlatformInfo.of("Windows 11", "amd64");
    private static final PlatformInfo LINUX = PlatformInfo.of("Linux", "amd64");
    private static final String V = BridgeEngineConnector.BRIDGE_VERSION;

    private static State map(String diagnostic, PlatformInfo p) {
        return BridgeStatusMapper.map(false, diagnostic, 0, V, p).state();
    }

    @Test
    void loaded() {
        EngineStatus s = BridgeStatusMapper.map(true, "Ooze native 0.1.0-g0 / ABI 1 / windows-x86_64", 3, V, WIN);
        assertEquals(State.LOADED, s.state());
        assertEquals(V, s.bridgeVersion());
    }

    @Test
    void exactG0Diagnostics() {
        assertEquals(State.UNSUPPORTED_PLATFORM, map("G0 native support is limited to Windows x64", LINUX));
        assertEquals(State.NOT_BUNDLED, map(
                "Ooze native unavailable: IOException: Missing bundled native resource: META-INF/ooze/native.properties", WIN));
        assertEquals(State.MISSING_LIBRARY, map(
                "Ooze native unavailable: IOException: Missing bundled native resource: natives/windows-x86_64/ooze_jni.dll", WIN));
        assertEquals(State.INCOMPATIBLE, map("Native artifact manifest is incompatible with this Java bridge", WIN));
        assertEquals(State.INCOMPATIBLE, map("Native ABI version mismatch; use matching Java and native artifacts", WIN));
        assertEquals(State.INCOMPATIBLE, map("Native manifest has unsupported capabilities", WIN));
        assertEquals(State.INCOMPATIBLE, map("Test-only native artifact requires explicit fault-injection opt-in", WIN));
        assertEquals(State.INCOMPATIBLE, map("A different Ooze native artifact is already loaded; restart Minecraft", WIN));
        assertEquals(State.INCOMPATIBLE, map("Loaded native library failed the ABI/capability handshake", WIN));
        assertEquals(State.LOAD_FAILED, map(
                "Ooze native unavailable: IOException: Native SHA-256 mismatch; restore the matching Ooze artifact", WIN));
        assertEquals(State.LOAD_FAILED, map("Native manifest has no valid SHA-256", WIN));
        assertEquals(State.LOAD_FAILED, map(
                "Ooze native unavailable: UnsatisfiedLinkError: Can't load IA 32-bit .dll on a AMD 64-bit platform", WIN));
        assertEquals(State.LOAD_FAILED, map("something new and unknown", WIN));
    }

    @Test
    void unsupportedPlatformWinsEvenWithOtherText() {
        assertEquals(State.UNSUPPORTED_PLATFORM, map("Missing bundled native resource: x", PlatformInfo.of("Mac OS X", "aarch64")));
    }

    @Test
    void detailKeepsTheBridgeTextForTheLog() {
        EngineStatus s = BridgeStatusMapper.map(false, "Native ABI version mismatch; use matching Java and native artifacts", 0, V, WIN);
        assertTrue(s.detail().contains("ABI version mismatch"));
        assertTrue(s.detail().contains(V));
    }

    /** Drift guard: fails if Sol's loader stops using a phrase this mapper relies on. */
    @Test
    void phrasesStillExistInSolsLoader() throws IOException {
        Path loader = Path.of("../../bridge-java/src/main/java/io/github/kyluua/ooze/bridge/NativeLoader.java");
        assertTrue(Files.isRegularFile(loader), "expected Sol's loader at " + loader.toAbsolutePath());
        String src = Files.readString(loader, StandardCharsets.UTF_8);
        for (String phrase : List.of(
                "limited to Windows x64",
                "Missing bundled native resource: ",
                "META-INF/ooze/native.properties",
                "is incompatible with this Java bridge",
                "Native ABI version mismatch",
                "unsupported capabilities",
                "Test-only native artifact",
                "already loaded",
                "handshake",
                "Native SHA-256 mismatch")) {
            assertTrue(src.contains(phrase), "NativeLoader no longer contains: " + phrase);
        }
    }
}
