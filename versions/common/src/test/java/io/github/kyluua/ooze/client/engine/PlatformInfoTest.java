package io.github.kyluua.ooze.client.engine;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class PlatformInfoTest {

    @Test
    void normalisesCommonJvmValues() {
        assertEquals("windows-x86_64", PlatformInfo.of("Windows 11", "amd64").id());
        assertEquals("windows-aarch64", PlatformInfo.of("Windows 11", "aarch64").id());
        assertEquals("linux-x86_64", PlatformInfo.of("Linux", "amd64").id());
        assertEquals("macos-aarch64", PlatformInfo.of("Mac OS X", "aarch64").id());
        assertEquals("macos-x86_64", PlatformInfo.of("Mac OS X", "x86_64").id());
        assertEquals("unknown-unknown", PlatformInfo.of("", "").id());
    }

    @Test
    void onlyWindowsX64IsSupportedFirst() {
        assertTrue(PlatformInfo.of("Windows 10", "amd64").isSupportedNativePlatform());
        assertFalse(PlatformInfo.of("Linux", "amd64").isSupportedNativePlatform());
        assertFalse(PlatformInfo.of("Mac OS X", "aarch64").isSupportedNativePlatform());
        assertFalse(PlatformInfo.of("Windows 11", "aarch64").isSupportedNativePlatform());
    }
}
