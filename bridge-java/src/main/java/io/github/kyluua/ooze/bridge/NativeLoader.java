package io.github.kyluua.ooze.bridge;

import java.io.IOException;
import java.io.InputStream;
import java.io.FileNotFoundException;
import java.nio.file.AtomicMoveNotSupportedException;
import java.nio.file.FileAlreadyExistsException;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.security.DigestInputStream;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.HexFormat;
import java.util.Locale;
import java.util.Properties;

/** Verifies bundled native resources and loads a version/hash-specific cached copy. */
public final class NativeLoader {
    private static final String MANIFEST = "META-INF/ooze/native.properties";
    private static final String PLATFORM = "windows-x86_64";
    private static final String RESOURCE = "natives/windows-x86_64/ooze_jni.dll";
    private static final long MAX_LIBRARY_BYTES = 64L * 1024 * 1024;
    private static String loadedHash;
    private static boolean ready;

    private NativeLoader() {}

    public enum LoadState {
        LOADED, MISSING_LIBRARY, UNSUPPORTED_PLATFORM, INCOMPATIBLE, LOAD_FAILED
    }

    public record LoadResult(LoadState state, String diagnostic, long capabilities) {
        public boolean available() {
            return state == LoadState.LOADED;
        }

        public String bridgeVersion() {
            return BridgeProtocol.LIBRARY_VERSION;
        }

        public int protocolVersion() {
            return BridgeProtocol.ABI_VERSION;
        }
    }

    public static LoadResult tryLoad(Path cacheDirectory) {
        return tryLoad(cacheDirectory, BridgeProtocol.ABI_VERSION, NativeLoader.class.getClassLoader());
    }

    static synchronized boolean isReady() {
        return ready;
    }

    static synchronized LoadResult tryLoad(Path cacheDirectory, int expectedAbi, ClassLoader resources) {
        try {
            if (cacheDirectory == null || resources == null) {
                return disabled(LoadState.LOAD_FAILED, "Native cache and resource loader are required");
            }
            if (!supportedPlatform(System.getProperty("os.name", ""), System.getProperty("os.arch", ""))) {
                return disabled(LoadState.UNSUPPORTED_PLATFORM, "G0 native support is limited to Windows x64");
            }
            Properties manifest = new Properties();
            try (InputStream stream = resource(resources, MANIFEST)) {
                manifest.load(stream);
            }
            if (!"1".equals(manifest.getProperty("schema"))
                    || !BridgeProtocol.LIBRARY_VERSION.equals(manifest.getProperty("library.version"))
                    || !PLATFORM.equals(manifest.getProperty("platform"))
                    || !RESOURCE.equals(manifest.getProperty("resource"))) {
                return disabled(LoadState.INCOMPATIBLE, "Native artifact manifest is incompatible with this Java bridge");
            }
            if (expectedAbi != BridgeProtocol.ABI_VERSION
                    || !Integer.toString(expectedAbi).equals(manifest.getProperty("bridge.abi"))) {
                return disabled(LoadState.INCOMPATIBLE, "Native ABI version mismatch; use matching Java and native artifacts");
            }
            String hash = manifest.getProperty("sha256", "");
            if (!hash.matches("[0-9a-f]{64}")) {
                return disabled(LoadState.INCOMPATIBLE, "Native manifest has no valid SHA-256");
            }
            long expectedCapabilities = Long.parseLong(manifest.getProperty("capabilities", "-1"));
            if (expectedCapabilities != 3 && expectedCapabilities != 7) {
                return disabled(LoadState.INCOMPATIBLE, "Native manifest has unsupported capabilities");
            }
            if (expectedCapabilities == 7 && !Boolean.getBoolean("ooze.bridge.allowFaultInjection")) {
                return disabled(LoadState.INCOMPATIBLE, "Test-only native artifact requires explicit fault-injection opt-in");
            }
            if (loadedHash != null && !loadedHash.equals(hash)) {
                return disabled(LoadState.INCOMPATIBLE, "A different Ooze native artifact is already loaded; restart Minecraft");
            }
            if (loadedHash == null) {
                Path root = cacheDirectory.toAbsolutePath().normalize();
                directory(root);
                Path version = directory(root.resolve(BridgeProtocol.LIBRARY_VERSION));
                Path destination = directory(directory(version.resolve(hash)).resolve(PLATFORM));
                Path library = destination.resolve("ooze_jni.dll");
                extract(resources, library, hash);
                System.load(library.toAbsolutePath().toString());
                loadedHash = hash;
            }
            if (NativeBindings.abiVersion() != expectedAbi
                    || NativeBindings.capabilities() != expectedCapabilities) {
                return disabled(LoadState.INCOMPATIBLE, "Loaded native library failed the ABI/capability handshake");
            }
            ready = true;
            return new LoadResult(LoadState.LOADED, "Ooze native " + BridgeProtocol.LIBRARY_VERSION
                    + " / ABI " + expectedAbi + " / " + PLATFORM, expectedCapabilities);
        } catch (FileNotFoundException failure) {
            return failure(LoadState.MISSING_LIBRARY, failure);
        } catch (ArtifactMismatchException | IllegalArgumentException failure) {
            return failure(LoadState.INCOMPATIBLE, failure);
        } catch (IOException | SecurityException | LinkageError failure) {
            return failure(LoadState.LOAD_FAILED, failure);
        }
    }

    static boolean supportedPlatform(String os, String arch) {
        String normalized = arch.toLowerCase(Locale.ROOT);
        return os.toLowerCase(Locale.ROOT).startsWith("windows")
                && (normalized.equals("amd64") || normalized.equals("x86_64"));
    }

    private static LoadResult disabled(LoadState state, String reason) {
        return new LoadResult(state, reason, 0);
    }

    private static LoadResult failure(LoadState state, Throwable failure) {
        return disabled(state, "Ooze native unavailable: " + failure.getClass().getSimpleName()
                + ": " + failure.getMessage());
    }

    private static final class ArtifactMismatchException extends IOException {
        private static final long serialVersionUID = 1L;

        ArtifactMismatchException(String message) {
            super(message);
        }
    }

    private static Path directory(Path directory) throws IOException {
        Files.createDirectories(directory);
        if (Files.isSymbolicLink(directory) || !Files.isDirectory(directory, LinkOption.NOFOLLOW_LINKS)) {
            throw new IOException("Native cache must contain real directories");
        }
        return directory;
    }

    private static InputStream resource(ClassLoader resources, String name) throws IOException {
        InputStream stream = resources.getResourceAsStream(name);
        if (stream == null) {
            throw new FileNotFoundException("Missing bundled native resource: " + name);
        }
        return stream;
    }

    private static void extract(ClassLoader resources, Path library, String expectedHash) throws IOException {
        if (Files.exists(library, LinkOption.NOFOLLOW_LINKS)) {
            verify(library, expectedHash);
            return;
        }
        Path temporary = Files.createTempFile(library.getParent(), "ooze-", ".tmp");
        try {
            try (InputStream stream = resource(resources, RESOURCE);
                    var output = Files.newOutputStream(temporary)) {
                byte[] buffer = new byte[16384];
                long copied = 0;
                int length;
                while ((length = stream.read(buffer)) != -1) {
                    copied += length;
                    if (copied > MAX_LIBRARY_BYTES) {
                        throw new IOException("Native library exceeds 64 MiB limit");
                    }
                    output.write(buffer, 0, length);
                }
            }
            verify(temporary, expectedHash);
            try {
                Files.move(temporary, library, StandardCopyOption.ATOMIC_MOVE);
            } catch (AtomicMoveNotSupportedException unsupported) {
                try {
                    Files.move(temporary, library);
                } catch (FileAlreadyExistsException concurrent) {
                    verify(library, expectedHash);
                }
            } catch (FileAlreadyExistsException concurrent) {
                verify(library, expectedHash);
            }
            verify(library, expectedHash);
        } finally {
            Files.deleteIfExists(temporary);
        }
    }

    private static void verify(Path library, String expectedHash) throws IOException {
        if (!Files.isRegularFile(library, LinkOption.NOFOLLOW_LINKS)
                || Files.size(library) > MAX_LIBRARY_BYTES) {
            throw new IOException("Native cache entry is not a bounded regular file");
        }
        try {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            try (var input = new DigestInputStream(Files.newInputStream(library), digest)) {
                input.transferTo(java.io.OutputStream.nullOutputStream());
            }
            if (!HexFormat.of().formatHex(digest.digest()).equals(expectedHash)) {
                throw new ArtifactMismatchException("Native SHA-256 mismatch; restore the matching Ooze artifact");
            }
        } catch (NoSuchAlgorithmException impossible) {
            throw new AssertionError("Java must provide SHA-256", impossible);
        }
    }
}
