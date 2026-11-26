package io.github.kyluua.ooze.mc.v26.mixin;

import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.renderer.state.gui.GuiRenderState;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

/**
 * Read-only access to the frame's deferred GUI render state, needed to submit custom
 * {@code GuiElementRenderState}s (ARCHITECTURE §5.8). 26.x renamed 1.21.11's {@code GuiGraphics}
 * to {@code GuiGraphicsExtractor} and keeps this field private; Fabric API does not widen it.
 * Accessor only: no injection, no behaviour change.
 */
@Mixin(GuiGraphicsExtractor.class)
public interface GuiGraphicsExtractorAccessor {

    @Accessor("guiRenderState")
    GuiRenderState oozeclient$guiRenderState();
}
