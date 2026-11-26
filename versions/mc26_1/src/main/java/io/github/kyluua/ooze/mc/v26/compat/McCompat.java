package io.github.kyluua.ooze.mc.v26.compat;

import com.mojang.blaze3d.platform.InputConstants;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.toasts.SystemToast;
import net.minecraft.client.gui.components.toasts.ToastManager;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import org.jspecify.annotations.Nullable;

/**
 * The Minecraft 26.1 family's spelling of the few calls that differ between 26.x families. Shared
 * 26.x code calls only these; each family project has its own copy (versions/mc26_*).
 */
public final class McCompat {

    /** GLFW key code (26.1 and 26.2 use GLFW); compiled in from this family's constant. */
    public static final InputConstants.Type DEV_PROBE_KEY_TYPE = InputConstants.Type.KEYSYM;
    public static final int DEV_PROBE_KEY = InputConstants.KEY_F9;

    private McCompat() {}

    public static ToastManager toasts(Minecraft mc) {
        return mc.getToastManager();
    }

    /** A system toast whose message wraps over several lines. */
    public static SystemToast noticeToast(Minecraft mc, SystemToast.SystemToastId id, Component title, Component body) {
        return SystemToast.multiline(mc, id, title, body);
    }

    public static @Nullable Screen screen(Minecraft mc) {
        return mc.screen;
    }

    public static void setScreen(Minecraft mc, Screen screen) {
        mc.setScreen(screen);
    }
}
