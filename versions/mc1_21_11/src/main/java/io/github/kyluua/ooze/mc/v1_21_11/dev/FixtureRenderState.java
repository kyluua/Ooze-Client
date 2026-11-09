package io.github.kyluua.ooze.mc.v1_21_11.dev;

import com.mojang.blaze3d.pipeline.RenderPipeline;
import com.mojang.blaze3d.vertex.VertexConsumer;
import net.minecraft.client.gui.navigation.ScreenRectangle;
import net.minecraft.client.gui.render.TextureSetup;
import net.minecraft.client.gui.render.state.GuiElementRenderState;
import net.minecraft.client.renderer.RenderPipelines;
import org.jspecify.annotations.Nullable;
import org.joml.Matrix3x2f;
import org.joml.Matrix3x2fc;

/**
 * A triangle list submitted into vanilla's deferred GUI render state (ARCHITECTURE §5.8).
 *
 * <p>Vertices are already converted from physical framebuffer pixels to GUI coordinates and are
 * an <b>owned copy</b>: no bridge {@code FrameLease} is referenced, so the lease is closed before
 * this element is submitted (eager-copy rule in docs/bridge/ABI_V1.md). Vanilla's {@code GUI}
 * pipeline draws QUADS, so each triangle is emitted as a degenerate quad (v0, v1, v2, v2).
 */
final class FixtureRenderState implements GuiElementRenderState {

    private final Matrix3x2f pose;
    private final float[] xy; // GUI coordinates, 2 floats per vertex
    private final int[] argb; // numeric 0xAARRGGBB per vertex (Minecraft's setColor format)
    private final @Nullable ScreenRectangle scissor;
    private final ScreenRectangle bounds;

    FixtureRenderState(Matrix3x2fc pose, float[] xy, int[] argb, @Nullable ScreenRectangle scissor) {
        if (xy.length != argb.length * 2 || argb.length % 3 != 0) {
            throw new IllegalArgumentException("triangle list expected");
        }
        this.pose = new Matrix3x2f(pose);
        this.xy = xy.clone();
        this.argb = argb.clone();
        this.scissor = scissor;
        float minX = Float.POSITIVE_INFINITY, minY = Float.POSITIVE_INFINITY;
        float maxX = Float.NEGATIVE_INFINITY, maxY = Float.NEGATIVE_INFINITY;
        for (int i = 0; i < xy.length; i += 2) {
            minX = Math.min(minX, xy[i]);
            maxX = Math.max(maxX, xy[i]);
            minY = Math.min(minY, xy[i + 1]);
            maxY = Math.max(maxY, xy[i + 1]);
        }
        int x = (int) Math.floor(minX);
        int y = (int) Math.floor(minY);
        ScreenRectangle local = new ScreenRectangle(x, y,
                Math.max(1, (int) Math.ceil(maxX) - x), Math.max(1, (int) Math.ceil(maxY) - y));
        ScreenRectangle transformed = local.transformMaxBounds(this.pose);
        this.bounds = scissor != null ? scissor.intersection(transformed) : transformed;
    }

    @Override
    public void buildVertices(VertexConsumer consumer) {
        for (int v = 0; v < argb.length; v += 3) {
            for (int k = 0; k < 3; k++) {
                consumer.addVertexWith2DPose(pose, xy[(v + k) * 2], xy[(v + k) * 2 + 1]).setColor(argb[v + k]);
            }
            // Degenerate fourth vertex: QUADS → (v0,v1,v2) + (v2,v2,v0) with zero area.
            consumer.addVertexWith2DPose(pose, xy[(v + 2) * 2], xy[(v + 2) * 2 + 1]).setColor(argb[v + 2]);
        }
    }

    @Override
    public RenderPipeline pipeline() {
        return RenderPipelines.GUI;
    }

    @Override
    public TextureSetup textureSetup() {
        return TextureSetup.noTexture();
    }

    @Override
    public @Nullable ScreenRectangle scissorArea() {
        return scissor;
    }

    @Override
    public @Nullable ScreenRectangle bounds() {
        return bounds;
    }
}
