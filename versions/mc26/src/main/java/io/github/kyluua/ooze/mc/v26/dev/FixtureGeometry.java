package io.github.kyluua.ooze.mc.v26.dev;

import com.mojang.blaze3d.vertex.VertexConsumer;
import net.minecraft.client.gui.navigation.ScreenRectangle;
import net.minecraft.client.gui.render.TextureSetup;
import org.jspecify.annotations.Nullable;
import org.joml.Matrix3x2f;
import org.joml.Matrix3x2fc;

/**
 * A triangle list submitted into vanilla's deferred GUI render state (ARCHITECTURE §5.8).
 *
 * <p>Vertices are already converted from physical framebuffer pixels to GUI coordinates and are
 * an <b>owned copy</b>: no bridge {@code FrameLease} is referenced, so the lease is closed before
 * this element is submitted (eager-copy rule in docs/bridge/ABI_V1.md). Vanilla's {@code GUI}
 * pipeline draws QUADS with back-face culling, so each triangle is re-wound to vanilla's order and
 * emitted as a degenerate quad (a, b, c, c).
 *
 * <p>Shared by the 26.x families. Each family's {@code FixtureRenderState} implements vanilla's
 * {@code GuiElementRenderState} on top of it with that family's {@code RenderPipeline} type (26.3
 * moved it to {@code com.mojang.renderpearl}).
 */
public abstract class FixtureGeometry {

    private final Matrix3x2f pose;
    private final float[] xy; // GUI coordinates, 2 floats per vertex
    private final int[] argb; // numeric 0xAARRGGBB per vertex (Minecraft's setColor format)
    private final @Nullable ScreenRectangle scissor;
    private final ScreenRectangle bounds;

    protected FixtureGeometry(Matrix3x2fc pose, float[] xy, int[] argb, @Nullable ScreenRectangle scissor) {
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

    public void buildVertices(VertexConsumer consumer) {
        // vanilla's GUI pipeline culls back faces (RenderPipelines.GUI never disables culling),
        // and the bridge does not promise a winding. Emit every triangle in vanilla's quad winding
        // (top-left, bottom-left, bottom-right: negative signed area with y down), accounting for a
        // mirroring pose. Found by Sol's Windows C0b run: the native fixture was wound the other way.
        boolean mirrored = pose.determinant() < 0;
        for (int v = 0; v < argb.length; v += 3) {
            int a = v;
            int b = v + 1;
            int c = v + 2;
            if ((signedArea(a, b, c) > 0) != mirrored) {
                int t = b;
                b = c;
                c = t;
            }
            vertex(consumer, a);
            vertex(consumer, b);
            vertex(consumer, c);
            // Degenerate fourth vertex: QUADS → (a,b,c) + (a,c,c) with zero area.
            vertex(consumer, c);
        }
    }

    private void vertex(VertexConsumer consumer, int i) {
        consumer.addVertexWith2DPose(pose, xy[i * 2], xy[i * 2 + 1]).setColor(argb[i]);
    }

    /** Twice the signed area of triangle (a, b, c) in GUI coordinates (y down). */
    float signedArea(int a, int b, int c) {
        float ax = xy[a * 2], ay = xy[a * 2 + 1];
        return (xy[b * 2] - ax) * (xy[c * 2 + 1] - ay) - (xy[c * 2] - ax) * (xy[b * 2 + 1] - ay);
    }


    public TextureSetup textureSetup() {
        return TextureSetup.noTexture();
    }

    public @Nullable ScreenRectangle scissorArea() {
        return scissor;
    }

    public @Nullable ScreenRectangle bounds() {
        return bounds;
    }
}
