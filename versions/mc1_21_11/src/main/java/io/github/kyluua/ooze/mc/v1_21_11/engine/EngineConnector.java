package io.github.kyluua.ooze.mc.v1_21_11.engine;

/**
 * Fabric-side port onto the Ooze native engine.
 *
 * <p>This is <b>not</b> a bridge protocol and declares no {@code native} methods. Sol's
 * {@code bridge-java} owns native loading, JNI and the versioned protocol; once its G0 handoff
 * lands, a single adapter class implements this interface by delegating to the agreed
 * bridge-java API. Until then {@link UnbundledEngineConnector} is the only production
 * implementation.
 */
public interface EngineConnector extends AutoCloseable {

    /** Loads/verifies the engine. Must not throw; failures are reported as an {@link EngineStatus}. */
    EngineStatus connect();

    /**
     * Developer-only round trip through the native bridge (C0 requirement 5). Only called when
     * {@link #connect()} returned {@link EngineStatus.State#LOADED} and the game runs in the
     * development environment.
     */
    DevProbeResult developerProbe();

    /** Releases native resources. Idempotent; never throws. */
    @Override
    void close();

    /** Outcome of {@link #developerProbe()}. */
    record DevProbeResult(boolean ok, String summary, long elapsedNanos) {}
}
