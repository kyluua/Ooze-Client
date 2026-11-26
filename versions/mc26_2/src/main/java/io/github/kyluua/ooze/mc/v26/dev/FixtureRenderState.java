package io.github.kyluua.ooze.mc.v26.dev;

import com.mojang.blaze3d.pipeline.RenderPipeline;
import net.minecraft.client.gui.navigation.ScreenRectangle;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.client.renderer.state.gui.GuiElementRenderState;
import org.jspecify.annotations.Nullable;
import org.joml.Matrix3x2fc;

/** {@link FixtureGeometry} as a vanilla GUI element, with Minecraft 26.2's pipeline type. */
public final class FixtureRenderState extends FixtureGeometry implements GuiElementRenderState {

    public FixtureRenderState(Matrix3x2fc pose, float[] xy, int[] argb, @Nullable ScreenRectangle scissor) {
        super(pose, xy, argb, scissor);
    }

    @Override
    public RenderPipeline pipeline() {
        return RenderPipelines.GUI;
    }
}
