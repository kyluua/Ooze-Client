package io.github.kyluua.ooze.mc.v26.compat;

import com.mojang.blaze3d.platform.InputConstants;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.toasts.SystemToast;
import net.minecraft.client.gui.components.toasts.ToastManager;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import org.jspecify.annotations.Nullable;

/**
 * The Minecraft 26.2 family's spelling of the few calls that differ between 26.x families: 26.2
 * moved the toast manager and the current screen to {@code Gui} and removed
 * {@code SystemToast.multiline}. Shared 26.x code calls only these; each family project has its own copy.
 */
public final class McCompat {

    /** GLFW key code (26.1 and 26.2 use GLFW); compiled in from this family's constant. */
    public static final InputConstants.Type DEV_PROBE_KEY_TYPE = InputConstants.Type.KEYSYM;
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
