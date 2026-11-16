package io.github.kyluua.ooze.mc.v1_21_11.gametest;

import io.github.kyluua.ooze.mc.v1_21_11.OozeClient;
import io.github.kyluua.ooze.mc.v1_21_11.dev.BridgeDiagnosticScreen;
import io.github.kyluua.ooze.mc.v1_21_11.dev.DeveloperProbe;
import io.github.kyluua.ooze.mc.v1_21_11.diag.LaunchReadiness;
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

        // The launcher's readiness marker is logged once the title screen opens (any engine state).
        check(LaunchReadiness.wasLogged(), "the OOZE_READY readiness marker was not logged on the title screen");

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

        checkFixturePixels(context);

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
        checkVisible(context, context.takeScreenshot("c0-bridge-fixture-released"), screen.lastCentroid(), RELEASED);

        context.getInput().holdMouse(0);
        context.waitTicks(5);
        check(screen.lastArgb() == PRESSED, String.format("pressed colour %08X", screen.lastArgb()));
        checkVisible(context, context.takeScreenshot("c0-bridge-fixture-pressed"), screen.lastCentroid(), PRESSED);
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

    /**
     * Visible-geometry check (Sol's L0 review): triangles submitted through the fixture path must
     * reach the framebuffer, whatever their winding and whatever the screen background draws.
     */
    private static void checkFixturePixels(ClientGameTestContext context) {
        context.setScreen(FixtureProbeScreen::new);
        context.waitTicks(5);
        double scale = context.computeOnClient(mc -> mc.getWindow().getGuiScale());
        Path shot = context.takeScreenshot("c0-fixture-probe");
        try (var in = java.nio.file.Files.newInputStream(shot);
                var image = com.mojang.blaze3d.platform.NativeImage.read(in)) {
            int left = image.getPixel((int) (FixtureProbeScreen.PROBE_LEFT[0] * scale), (int) (FixtureProbeScreen.PROBE_LEFT[1] * scale));
            int right = image.getPixel((int) (FixtureProbeScreen.PROBE_RIGHT[0] * scale), (int) (FixtureProbeScreen.PROBE_RIGHT[1] * scale));
            LOG.info("C0 fixture pixels: left={} right={} (expected {} / {}), gui scale {}",
                    String.format("%08X", left), String.format("%08X", right),
                    String.format("%08X", FixtureProbeScreen.LEFT), String.format("%08X", FixtureProbeScreen.RIGHT), scale);
            check(close(left, FixtureProbeScreen.LEFT), "counter-clockwise fixture triangle not visible: " + String.format("%08X", left));
            check(close(right, FixtureProbeScreen.RIGHT), "clockwise fixture triangle not visible: " + String.format("%08X", right));
        } catch (java.io.IOException e) {
            throw new AssertionError("could not read the fixture screenshot " + shot, e);
        }
        context.setScreen(() -> new TitleScreen());
        context.waitForScreen(TitleScreen.class);
    }

    /** The native fixture itself must be visible in the framebuffer, not only in its data. */
    private static void checkVisible(ClientGameTestContext context, Path shot, float[] centroidGui, int expected) {
        double scale = context.computeOnClient(mc -> mc.getWindow().getGuiScale());
        try (var in = java.nio.file.Files.newInputStream(shot);
                var image = com.mojang.blaze3d.platform.NativeImage.read(in)) {
            int x = (int) Math.round(centroidGui[0] * scale);
            int y = (int) Math.round(centroidGui[1] * scale);
            check(x >= 0 && y >= 0 && x < image.getWidth() && y < image.getHeight(), "fixture centroid off screen: " + x + "," + y);
            int pixel = image.getPixel(x, y);
            LOG.info("C0 bridge fixture pixel at ({},{}) = {} (expected {})", x, y, String.format("%08X", pixel), String.format("%08X", expected));
            check(close(pixel, expected), String.format("native fixture not visible: pixel %08X, expected %08X", pixel, expected));
        } catch (java.io.IOException e) {
            throw new AssertionError("could not read " + shot, e);
        }
    }

    private static boolean close(int actual, int expected) {
        for (int shift = 0; shift <= 16; shift += 8) {
            if (Math.abs(((actual >> shift) & 0xFF) - ((expected >> shift) & 0xFF)) > 6) {
                return false;
            }
        }
        return true;
    }
}
