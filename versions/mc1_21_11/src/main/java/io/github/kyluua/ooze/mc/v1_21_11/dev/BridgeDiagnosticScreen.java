package io.github.kyluua.ooze.mc.v1_21_11.dev;

import com.mojang.blaze3d.platform.Window;
import io.github.kyluua.ooze.bridge.BridgeException;
import io.github.kyluua.ooze.bridge.BridgeProtocol;
import io.github.kyluua.ooze.bridge.EngineSession;
import io.github.kyluua.ooze.bridge.FrameBuffer;
import io.github.kyluua.ooze.bridge.InputBatch;
import io.github.kyluua.ooze.mc.v1_21_11.OozeClient;
import io.github.kyluua.ooze.mc.v1_21_11.mixin.GuiGraphicsAccessor;
import java.nio.ByteBuffer;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.input.MouseButtonEvent;
import net.minecraft.network.chat.Component;
import org.jspecify.annotations.Nullable;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Developer-only screen (G0 handoff step 5): drives Sol's diagnostic fixture through the real
 * bridge every frame — one batched input submission and one fixture render per frame — and draws
 * the result through vanilla's GUI render state. Not a product screen; reachable only through the
 * developer key mapping, which exists only in the development environment with a loaded engine.
 */
public final class BridgeDiagnosticScreen extends Screen {

    private static final Logger LOG = LoggerFactory.getLogger("OozeClient/BridgeDiagnostic");

    private @Nullable EngineSession session;
    private @Nullable InputBatch batch;
    private final FrameBuffer frameBuffer = new FrameBuffer();
    private long frameId;
    private @Nullable Boolean pendingPrimary;
    private @Nullable String failure;

    // Last rendered fixture, for launch tests and the on-screen readout.
    private volatile int lastArgb;
    private volatile long framesRendered;

    public BridgeDiagnosticScreen() {
        super(Component.translatable("oozeclient.dev.diagnostic.title"));
    }

    @Override
    protected void init() {
        if (session == null && failure == null && !OozeClient.gate().oozeFeaturesEnabled()) {
            failure = "engine not loaded";
        }
        if (session == null && failure == null) {
            try {
                // Created on the render thread, which owns it until removed().
                session = new EngineSession(BridgeProtocol.DEV_FIXTURE);
                batch = new InputBatch(8);
            } catch (RuntimeException e) {
                fail("create", e);
            }
        }
    }

    @Override
    public void render(GuiGraphics graphics, int mouseX, int mouseY, float partialTick) {
        super.render(graphics, mouseX, mouseY, partialTick);
        EngineSession s = session;
        InputBatch b = batch;
        if (s != null && b != null) {
            try {
                renderFixture(graphics, s, b);
            } catch (RuntimeException | LinkageError e) {
                fail("frame", e);
            }
        }
        Component line = failure != null
                ? Component.translatable("oozeclient.dev.diagnostic.failed", failure)
                : Component.translatable("oozeclient.dev.diagnostic.hint", framesRendered, String.format("%08X", lastArgb));
        graphics.drawString(font, title, 8, 8, 0xFFFFFFFF);
        graphics.drawString(font, line, 8, 20, failure != null ? 0xFFFF7A8C : 0xFFDCE8F7);
    }

    private void renderFixture(GuiGraphics graphics, EngineSession s, InputBatch b) {
        Window window = minecraft.getWindow();
        int fbW = window.getWidth();
        int fbH = window.getHeight();
        double guiScale = window.getGuiScale();
        // Ooze scale model (DESIGN_SYSTEM §9.1), clamped to the ABI's accepted range.
        float uiScale = (float) Math.max(0.75, Math.min(4.0, Math.min(fbW / 1920.0, fbH / 1080.0)));
        // Mouse position in physical framebuffer pixels (window coords may differ on HiDPI).
        double px = minecraft.mouseHandler.xpos() * fbW / Math.max(1, window.getScreenWidth());
        double py = minecraft.mouseHandler.ypos() * fbH / Math.max(1, window.getScreenHeight());

        b.clear().resize(Math.max(1, fbW), Math.max(1, fbH), uiScale).pointerMove((float) px, (float) py);
        Boolean press = pendingPrimary;
        if (press != null) {
            b.pointerButton(0, press);
            pendingPrimary = null;
        }
        s.submit(b);

        float[] xy = new float[12];
        int[] argb = new int[6];
        // Eager copy into Minecraft-owned data, then close the lease before vanilla consumes it.
        try (FrameBuffer.FrameLease lease = s.renderFixture(frameId++, frameBuffer)) {
            ByteBuffer bytes = lease.bytes();
            int count = bytes.getInt(24);
            int stride = bytes.getInt(28);
            for (int i = 0; i < count; i++) {
                int at = 32 + i * stride;
                xy[i * 2] = (float) (bytes.getFloat(at) / guiScale);
                xy[i * 2 + 1] = (float) (bytes.getFloat(at + 4) / guiScale);
                argb[i] = bytes.getInt(at + 8); // numeric 0xAARRGGBB == Minecraft setColor(int)
            }
        }
        ((GuiGraphicsAccessor) graphics).oozeclient$guiRenderState()
                .submitGuiElement(new FixtureRenderState(graphics.pose(), xy, argb, null));
        lastArgb = argb[0];
        framesRendered++;
    }

    @Override
    public boolean mouseClicked(MouseButtonEvent event, boolean doubleClick) {
        if (event.button() == 0) {
            pendingPrimary = Boolean.TRUE;
            return true;
        }
        return super.mouseClicked(event, doubleClick);
    }

    @Override
    public boolean mouseReleased(MouseButtonEvent event) {
        if (event.button() == 0) {
            pendingPrimary = Boolean.FALSE;
            return true;
        }
        return super.mouseReleased(event);
    }

    @Override
    public void removed() {
        closeSession();
        super.removed();
    }

    @Override
    public boolean isPauseScreen() {
        return false;
    }

    /** ARGB of the most recent fixture (0xFFF8F9FF released, 0xFF171923 pressed). */
    public int lastArgb() {
        return lastArgb;
    }

    public long framesRendered() {
        return framesRendered;
    }

    public @Nullable String failure() {
        return failure;
    }

    private void fail(String where, Throwable e) {
        String status = e instanceof BridgeException be ? "status " + be.status() : e.getClass().getSimpleName();
        failure = where + ": " + status;
        LOG.error("Native bridge diagnostic stopped after {} failure", where, e);
        closeSession();
        // Session and leases are already released; now stop all further native work.
        OozeClient.disableEngine("the bridge diagnostic (" + where + ")", e);
    }

    private void closeSession() {
        EngineSession s = session;
        session = null;
        batch = null;
        if (s != null) {
            try {
                s.close();
            } catch (RuntimeException e) {
                LOG.warn("Closing the diagnostic engine session failed", e);
            }
        }
    }
}
