package io.github.kyluua.ooze.mc.v1_21_11.diag;

import io.github.kyluua.ooze.mc.v1_21_11.engine.EngineStatus;
import io.github.kyluua.ooze.mc.v1_21_11.engine.PlatformInfo;
import java.util.concurrent.atomic.AtomicBoolean;
import net.fabricmc.fabric.api.client.screen.v1.ScreenEvents;
import net.minecraft.client.gui.components.toasts.SystemToast;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.network.chat.Component;

/**
 * Shows one vanilla system toast the first time the title screen opens while the engine is not
 * loaded. It is a notice, not a control: it offers no Ooze actions (TARGET_PLATFORM §5).
 */
public final class UnavailableNotice {

    /** 12 s on screen, scaled by the player's notification-time option like other system toasts. */
    private static final SystemToast.SystemToastId TOAST_ID = new SystemToast.SystemToastId(12_000L);

    private static final AtomicBoolean shown = new AtomicBoolean();

    private UnavailableNotice() {}

    public static void register(EngineStatus status, PlatformInfo platform) {
        Component title = Component.translatable(EngineDiagnostics.KEY_TITLE);
        Component body = Component.translatable(EngineDiagnostics.bodyKey(status), EngineDiagnostics.bodyArgs(status, platform));
        ScreenEvents.AFTER_INIT.register((client, screen, width, height) -> {
            if (screen instanceof TitleScreen && shown.compareAndSet(false, true)) {
                client.getToastManager().addToast(SystemToast.multiline(client, TOAST_ID, title, body));
            }
        });
    }

    /** True once the notice has been queued (used by launch tests). */
    public static boolean wasShown() {
        return shown.get();
    }
}
