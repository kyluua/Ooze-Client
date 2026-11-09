package io.github.kyluua.ooze.mc.v1_21_11.gametest;

import io.github.kyluua.ooze.mc.v1_21_11.OozeClient;
import io.github.kyluua.ooze.mc.v1_21_11.dev.BridgeDiagnosticScreen;
import io.github.kyluua.ooze.mc.v1_21_11.dev.DeveloperProbe;
import io.github.kyluua.ooze.mc.v1_21_11.diag.UnavailableNotice;
import io.github.kyluua.ooze.mc.v1_21_11.engine.EngineConnector;
import io.github.kyluua.ooze.mc.v1_21_11.engine.EngineStatus;
import io.github.kyluua.ooze.mc.v1_21_11.engine.PlatformInfo;
import io.github.kyluua.ooze.mc.v1_21_11.mixin.GuiGraphicsAccessor;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.List;
import java.util.Locale;
import net.fabricmc.fabric.api.client.gametest.v1.FabricClientGameTest;
import net.fabricmc.fabric.api.client.gametest.v1.context.ClientGameTestContext;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.client.gui.screens.options.OptionsScreen;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * C0 launch test.
 *
 * <ul>
 *   <li>Always: the client reaches the vanilla title screen, reports the expected engine state and
 *       vanilla menus still work.
 *   <li>Engine not loaded: no Ooze key mappings and the "Ooze Client is off" notice is queued.
 *   <li>Engine loaded (Windows x64 with Sol's DLL): the developer probe round trip passes and the
 *       diagnostic screen renders the fixture, switching colour while the left button is held.
 * </ul>
 *
 * <p>Expected state: {@code -Doozeclient.test.expectEngineState}, otherwise UNSUPPORTED_PLATFORM on
 * non-Windows-x64 hosts and LOADED on Windows x64.
 */
public final class LaunchToTitleScreenTest implements FabricClientGameTest {

    private static final Logger LOG = LoggerFactory.getLogger("OozeClient/C0-LaunchTest");
    private static final int RELEASED = 0xFFF8F9FF;
    private static final int PRESSED = 0xFF171923;

    @Override
    public void runTest(ClientGameTestContext context) {
        context.waitForScreen(TitleScreen.class);

        EngineStatus status = OozeClient.engineStatus();
        String expected = System.getProperty("oozeclient.test.expectEngineState",
                PlatformInfo.current().isSupportedNativePlatform() ? "LOADED" : "UNSUPPORTED_PLATFORM");
        check(status.state().name().equals(expected),
                "engine state " + status.state() + " != expected " + expected + " (" + status.detail() + ")");

        // The render-state accessor mixin must apply in dev (named) and production (intermediary) runs.
        check(GuiGraphicsAccessor.class.isAssignableFrom(GuiGraphics.class),
                "GuiGraphicsAccessor mixin was not applied to GuiGraphics");

        List<String> oozeKeys = context.computeOnClient(mc -> Arrays.stream(mc.options.keyMappings)
                .map(KeyMapping::getName)
                .filter(n -> n.startsWith("key.oozeclient"))
                .toList());
        if (!status.isLoaded()) {
            check(oozeKeys.isEmpty(), "Ooze key mappings registered without an engine: " + oozeKeys);
            check(!OozeClient.gate().oozeFeaturesEnabled(), "Ooze features enabled without an engine");
            check(UnavailableNotice.wasShown(), "the 'Ooze Client is off' notice was not queued on the title screen");
        } else {
            check(!UnavailableNotice.wasShown(), "the unavailable notice was shown although the engine is loaded");
        }

        context.waitTicks(30);
        String tag = status.state().name().toLowerCase(Locale.ROOT);
        Path shot = context.takeScreenshot("c0-title-" + tag);
        LOG.info("C0 screenshot: {}", shot.toAbsolutePath());

        if (status.isLoaded() && OozeClient.gate().developerProbeEnabled()) {
            exerciseBridge(context, oozeKeys);
        }

        // Vanilla stays usable: open Options and come back.
        context.clickScreenButton("menu.options");
        context.waitForScreen(OptionsScreen.class);
        context.clickScreenButton("gui.done");
        context.waitForScreen(TitleScreen.class);

        LOG.info("C0 launch test PASSED: state={}, title screen reached, vanilla Options round-trip OK", status.state());
    }

    private static void exerciseBridge(ClientGameTestContext context, List<String> oozeKeys) {
        check(oozeKeys.contains(DeveloperProbe.KEY_NAME), "developer probe key missing: " + oozeKeys);
        EngineConnector.DevProbeResult probe = context.computeOnClient(mc -> DeveloperProbe.runProbe(OozeClient.connector()));
        LOG.info("C0 bridge probe ok={} in {} µs: {}", probe.ok(), probe.elapsedNanos() / 1_000, probe.summary());
        check(probe.ok(), "bridge probe failed: " + probe.summary());

        context.setScreen(BridgeDiagnosticScreen::new);
        context.waitForScreen(BridgeDiagnosticScreen.class);
        context.getInput().setCursorPos(120, 90);
        context.waitTicks(10);
        BridgeDiagnosticScreen screen = context.computeOnClient(mc -> (BridgeDiagnosticScreen) mc.screen);
        check(screen.failure() == null, "diagnostic failed: " + screen.failure());
        check(screen.framesRendered() > 0, "no fixture frames rendered");
        check(screen.lastArgb() == RELEASED, String.format("released colour %08X", screen.lastArgb()));
        context.takeScreenshot("c0-bridge-fixture-released");

        context.getInput().holdMouse(0);
        context.waitTicks(5);
        check(screen.lastArgb() == PRESSED, String.format("pressed colour %08X", screen.lastArgb()));
        context.takeScreenshot("c0-bridge-fixture-pressed");
        context.getInput().releaseMouse(0);
        context.waitTicks(5);
        check(screen.lastArgb() == RELEASED, String.format("colour after release %08X", screen.lastArgb()));
        LOG.info("C0 bridge fixture PASSED: {} frames, press/release colour change observed", screen.framesRendered());

        context.setScreen(TitleScreen::new);
        context.waitForScreen(TitleScreen.class);
    }

    private static void check(boolean condition, String message) {
        if (!condition) {
            throw new AssertionError(message);
        }
    }
}
