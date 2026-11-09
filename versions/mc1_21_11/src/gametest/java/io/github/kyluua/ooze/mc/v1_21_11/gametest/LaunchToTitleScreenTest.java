package io.github.kyluua.ooze.mc.v1_21_11.gametest;

import io.github.kyluua.ooze.mc.v1_21_11.OozeClient;
import io.github.kyluua.ooze.mc.v1_21_11.diag.UnavailableNotice;
import io.github.kyluua.ooze.mc.v1_21_11.engine.EngineStatus;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.List;
import net.fabricmc.fabric.api.client.gametest.v1.FabricClientGameTest;
import net.fabricmc.fabric.api.client.gametest.v1.context.ClientGameTestContext;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.client.gui.screens.options.OptionsScreen;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * C0 launch test: the client reaches the vanilla title screen with Ooze loaded, reports the
 * expected engine state, registers no Ooze controls without an engine, and vanilla menus still
 * work. Expected state comes from -Doozeclient.test.expectEngineState (default NOT_BUNDLED).
 */
public final class LaunchToTitleScreenTest implements FabricClientGameTest {

    private static final Logger LOG = LoggerFactory.getLogger("OozeClient/C0-LaunchTest");

    @Override
    public void runTest(ClientGameTestContext context) {
        context.waitForScreen(TitleScreen.class);

        EngineStatus status = OozeClient.engineStatus();
        String expected = System.getProperty("oozeclient.test.expectEngineState", "NOT_BUNDLED");
        check(status.state().name().equals(expected), "engine state " + status.state() + " != expected " + expected);

        if (!status.isLoaded()) {
            List<String> oozeKeys = context.computeOnClient(mc -> Arrays.stream(mc.options.keyMappings)
                    .map(KeyMapping::getName)
                    .filter(n -> n.startsWith("key.oozeclient"))
                    .toList());
            check(oozeKeys.isEmpty(), "Ooze key mappings registered without an engine: " + oozeKeys);
            check(!OozeClient.gate().oozeFeaturesEnabled(), "Ooze features enabled without an engine");
            check(UnavailableNotice.wasShown(), "the 'Ooze Client is off' notice was not queued on the title screen");
        }

        // Let the toast slide in, then capture evidence.
        context.waitTicks(30);
        Path shot = context.takeScreenshot("c0-title-" + status.state().name().toLowerCase(java.util.Locale.ROOT));
        LOG.info("C0 screenshot: {}", shot.toAbsolutePath());

        // Vanilla stays usable: open Options and come back.
        context.clickScreenButton("menu.options");
        context.waitForScreen(OptionsScreen.class);
        context.clickScreenButton("gui.done");
        context.waitForScreen(TitleScreen.class);

        LOG.info("C0 launch test PASSED: state={}, title screen reached, vanilla Options round-trip OK", status.state());
    }

    private static void check(boolean condition, String message) {
        if (!condition) {
            throw new AssertionError(message);
        }
    }
}
