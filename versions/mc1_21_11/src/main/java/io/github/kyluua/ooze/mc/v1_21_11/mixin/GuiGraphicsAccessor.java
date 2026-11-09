package io.github.kyluua.ooze.mc.v1_21_11.mixin;

import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.render.state.GuiRenderState;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

/**
 * Read-only access to the frame's deferred GUI render state, needed to submit custom
 * {@code GuiElementRenderState}s (ARCHITECTURE §5.8). 1.21.11 keeps this field package-private and
 * Fabric API does not widen it. Accessor only: no injection, no behaviour change.
 */
@Mixin(GuiGraphics.class)
public interface GuiGraphicsAccessor {

    @Accessor("guiRenderState")
    GuiRenderState oozeclient$guiRenderState();
}
