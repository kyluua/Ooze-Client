package io.github.kyluua.ooze.mc.v26.compat;

import com.mojang.blaze3d.platform.InputConstants;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.toasts.SystemToast;
import net.minecraft.client.gui.components.toasts.ToastManager;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import org.jspecify.annotations.Nullable;

/**
 * The Minecraft 26.3 family's spelling of the few calls that differ between 26.x families. 26.3
 * replaced GLFW with SDL3: key mappings use {@code InputConstants.Type.KEYBOARD} with SDL
 * scancodes ({@code KEY_F9} is 66 here, 298 under GLFW), so this value must come from 26.3's own
 * constants. Like 26.2, the toast manager and the current screen live in {@code Gui}. Shared 26.x code calls only these; each family project has its own copy.
 */
public final class McCompat {

    /** SDL3 scancode; compiled in from this family's constant. */
    public static final InputConstants.Type DEV_PROBE_KEY_TYPE = InputConstants.Type.KEYBOARD;
    public static final int DEV_PROBE_KEY = InputConstants.KEY_F9;

    private McCompat() {}

    public static ToastManager toasts(Minecraft mc) {
        return mc.gui.toastManager();
    }

    /** A system toast whose message wraps over several lines (26.2 wraps in the constructor). */
    public static SystemToast noticeToast(Minecraft mc, SystemToast.SystemToastId id, Component title, Component body) {
        return new SystemToast(id, title, body);
    }

    public static @Nullable Screen screen(Minecraft mc) {
        return mc.gui.screen();
    }

    public static void setScreen(Minecraft mc, Screen screen) {
        mc.gui.setScreen(screen);
    }
}
