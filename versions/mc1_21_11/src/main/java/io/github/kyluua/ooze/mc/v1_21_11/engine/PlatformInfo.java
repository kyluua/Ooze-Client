package io.github.kyluua.ooze.mc.v1_21_11.engine;

import java.util.Locale;
import java.util.Set;

/**
 * Normalised OS/architecture of the running JVM, used for diagnostics and the supported-platform
 * policy in TARGET_PLATFORM §2. Native loading itself belongs to Sol's bridge-java.
 */
public record PlatformInfo(String os, String arch) {

    /** Platforms with a published native artifact and a passing Windows launch test (policy, not detection). */
    public static final Set<String> SUPPORTED_NATIVE_PLATFORMS = Set.of("windows-x86_64");

    public static PlatformInfo current() {
        return of(System.getProperty("os.name", ""), System.getProperty("os.arch", ""));
    }

    public static PlatformInfo of(String osName, String osArch) {
        String o = osName.toLowerCase(Locale.ROOT);
        String os;
        if (o.startsWith("windows")) {
            os = "windows";
        } else if (o.startsWith("mac") || o.startsWith("darwin")) {
            os = "macos";
        } else if (o.startsWith("linux")) {
            os = "linux";
        } else {
            os = o.isBlank() ? "unknown" : o.replaceAll("[^a-z0-9]+", "-");
        }
        String a = osArch.toLowerCase(Locale.ROOT);
        String arch = switch (a) {
            case "amd64", "x86_64", "x64" -> "x86_64";
            case "aarch64", "arm64" -> "aarch64";
            case "x86", "i386", "i486", "i586", "i686" -> "x86";
            default -> a.isBlank() ? "unknown" : a;
        };
        return new PlatformInfo(os, arch);
    }

    /** Identifier such as {@code windows-x86_64}. */
    public String id() {
        return os + "-" + arch;
    }

    public boolean isSupportedNativePlatform() {
        return SUPPORTED_NATIVE_PLATFORMS.contains(id());
    }
}
