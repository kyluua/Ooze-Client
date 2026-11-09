package io.github.kyluua.ooze.mc.v1_21_11.engine;

/** Connector used when this JAR ships without Sol's bridge-java and native library. */
public final class UnbundledEngineConnector implements EngineConnector {

    @Override
    public EngineStatus connect() {
        return EngineStatus.of(
                EngineStatus.State.NOT_BUNDLED,
                "This build of Ooze Client does not include the native engine.",
                "bridge-java and the native library are not packaged in this JAR "
                        + "(expected until the G0 bridge handoff is integrated).");
    }

    @Override
    public DevProbeResult developerProbe() {
        throw new IllegalStateException("No native engine is bundled; the developer probe is unavailable.");
    }

    @Override
    public void close() {}
}
