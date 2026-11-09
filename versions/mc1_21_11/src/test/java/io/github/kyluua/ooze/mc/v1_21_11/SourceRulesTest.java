package io.github.kyluua.ooze.mc.v1_21_11;

import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;
import java.util.regex.Pattern;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;

/**
 * Architecture rules from AGENTS.md enforced on the Fabric adapter sources:
 * no JNI declarations outside Sol's bridge-java, no raw graphics APIs, no session secrets.
 */
class SourceRulesTest {

    private static final Path SOURCES = Paths.get("src/main/java");

    private static final Pattern NATIVE_METHOD = Pattern.compile("\\bnative\\s+[\\w<>\\[\\],\\s]+\\s+\\w+\\s*\\(");
    private static final List<String> FORBIDDEN_IMPORTS = List.of(
            "org.lwjgl.opengl", "org.lwjgl.vulkan", "org.lwjgl.nanovg", "com.mojang.blaze3d.opengl");
    private static final List<String> FORBIDDEN_IDENTITY_CALLS = List.of(
            "getAccessToken", "getSessionId", "accessToken", "getXuid", "getClientId");

    @Test
    void adapterDeclaresNoNativeMethods() throws IOException {
        assertNoViolations(src -> NATIVE_METHOD.matcher(stripComments(src)).find() ? "declares a native method" : null);
    }

    @Test
    void adapterUsesNoRawGraphicsApis() throws IOException {
        assertNoViolations(src -> FORBIDDEN_IMPORTS.stream()
                .filter(p -> src.contains("import " + p) || src.contains("import static " + p))
                .findFirst()
                .map(p -> "imports " + p)
                .orElse(null));
    }

    @Test
    void adapterNeverTouchesSessionSecrets() throws IOException {
        assertNoViolations(src -> FORBIDDEN_IDENTITY_CALLS.stream()
                .filter(stripComments(src)::contains)
                .findFirst()
                .map(p -> "references " + p)
                .orElse(null));
    }

    private interface Rule {
        String check(String source);
    }

    private static void assertNoViolations(Rule rule) throws IOException {
        assertTrue(Files.isDirectory(SOURCES), "run from the project directory: " + SOURCES.toAbsolutePath());
        List<String> violations = new ArrayList<>();
        try (Stream<Path> files = Files.walk(SOURCES)) {
            for (Path p : (Iterable<Path>) files.filter(f -> f.toString().endsWith(".java"))::iterator) {
                String v = rule.check(Files.readString(p, StandardCharsets.UTF_8));
                if (v != null) {
                    violations.add(SOURCES.relativize(p) + " " + v);
                }
            }
        }
        assertTrue(violations.isEmpty(), String.join("\n", violations));
    }

    /** Removes comments and string/char literals so prose never triggers a rule. */
    private static String stripComments(String src) {
        return src.replaceAll("(?s)\"\"\".*?\"\"\"", "\"\"")
                .replaceAll("\"(?:\\\\.|[^\"\\\\\\n])*\"", "\"\"")
                .replaceAll("'(?:\\\\.|[^'\\\\\\n])'", "''")
                .replaceAll("(?s)/\\*.*?\\*/", "")
                .replaceAll("//[^\\n]*", "");
    }
}
