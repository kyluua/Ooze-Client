package io.github.kyluua.ooze.mc.v26.gametest;

import io.github.kyluua.ooze.mc.v26.dev.BridgeDiagnosticScreen;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;

/**
 * Test-only screen (gametest source set, never shipped). It draws two triangles through the same
 * submission path as {@code BridgeDiagnosticScreen}, one wound each way, so the launch test can
 * assert visible pixels on any platform without the native library.
 */
final class FixtureProbeScreen extends Screen {

    static final int LEFT = 0xFFF8F9FF;
    static final int RIGHT = 0xFF171923;
    /** GUI-coordinate centres of the two triangles' interiors. */
    static final float[] PROBE_LEFT = {40, 95};
    static final float[] PROBE_RIGHT = {140, 95};

    FixtureProbeScreen() {
        super(Component.literal("Ooze fixture probe (test only)"));
    }

    @Override
    public void extractRenderState(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float partialTick) {
        super.extractRenderState(graphics, mouseX, mouseY, partialTick);
        // Left: counter-clockwise on screen (y down). Right: clockwise. Both must be visible.
        float[] xy = {
            20, 40, 20, 110, 110, 110,
            120, 40, 210, 110, 120, 110,
        };
        int[] argb = {LEFT, LEFT, LEFT, RIGHT, RIGHT, RIGHT};
        BridgeDiagnosticScreen.submitTriangles(graphics, xy, argb);
    }
}
