package io.github.kyluua.ooze.mc.v1_21_11.dev;

import com.mojang.blaze3d.platform.InputConstants;
import io.github.kyluua.ooze.mc.v1_21_11.engine.EngineConnector;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.gui.components.toasts.SystemToast;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import org.lwjgl.glfw.GLFW;
import org.slf4j.Logger;

/**
 * Developer-only action that exercises the native bridge (C0 requirement 5). Registered only when
 * {@code EngineGate#developerProbeEnabled()} is true: development environment AND loaded engine.
 * F9 runs one synchronous probe round trip (logged + toast) and opens {@link BridgeDiagnosticScreen}.
 */
public final class DeveloperProbe {

    public static final String KEY_NAME = "key.oozeclient.dev.bridge_probe";
    private static final SystemToast.SystemToastId TOAST_ID = new SystemToast.SystemToastId(6_000L);

    private DeveloperProbe() {}

    public static void register(EngineConnector connector, Logger log) {
        KeyMapping.Category category = KeyMapping.Category.register(Identifier.fromNamespaceAndPath("oozeclient", "developer"));
        KeyMapping key = KeyBindingHelper.registerKeyBinding(
                new KeyMapping(KEY_NAME, InputConstants.Type.KEYSYM, GLFW.GLFW_KEY_F9, category));
        ClientTickEvents.END_CLIENT_TICK.register(client -> {
            while (key.consumeClick()) {
                EngineConnector.DevProbeResult result = runProbe(connector);
                log.info("[dev] bridge probe ok={} in {} µs: {}", result.ok(), result.elapsedNanos() / 1_000, result.summary());
                SystemToast.addOrUpdate(
                        client.getToastManager(),
                        TOAST_ID,
                        Component.translatable(result.ok() ? "oozeclient.dev.probe.ok" : "oozeclient.dev.probe.failed"),
                        Component.literal(result.summary()));
                if (result.ok() && !(client.screen instanceof BridgeDiagnosticScreen)) {
                    client.setScreen(new BridgeDiagnosticScreen());
                }
            }
        });
    }

    /** Runs the probe without letting a bridge failure escape into the game loop. */
    public static EngineConnector.DevProbeResult runProbe(EngineConnector connector) {
        try {
            return connector.developerProbe();
        } catch (RuntimeException e) {
            return new EngineConnector.DevProbeResult(false, e.toString(), 0L);
        }
    }
}
