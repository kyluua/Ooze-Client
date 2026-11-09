package io.github.kyluua.ooze.mc.v1_21_11.engine;

/**
 * Development-only connector that reports a chosen failure state, so the vanilla-only diagnostic
 * path can be exercised in a launched client ({@code -Doozeclient.dev.engineFault=<state>}).
 * It can never report {@link EngineStatus.State#LOADED}: success must come from the real bridge.
 */
final class FaultInjectedEngineConnector implements EngineConnector {

    private final EngineStatus.State state;
    private final PlatformInfo platform;

    FaultInjectedEngineConnector(EngineStatus.State state, PlatformInfo platform) {
        if (state == EngineStatus.State.LOADED) {
            throw new IllegalArgumentException("LOADED cannot be fault-injected");
        }
        this.state = state;
        this.platform = platform;
    }

    @Override
    public EngineStatus connect() {
        String injected = " [developer fault injection: oozeclient.dev.engineFault]";
        return switch (state) {
            case MISSING_LIBRARY -> EngineStatus.of(state, "No Ooze native library was found for " + platform.id() + ".", "simulated missing file" + injected);
            case UNSUPPORTED_PLATFORM -> EngineStatus.of(state, platform.id() + " is not a supported Ooze native platform yet.", "simulated" + injected);
            case INCOMPATIBLE -> new EngineStatus(state, "0.0.0-simulated", "The Ooze native library version does not match this mod.", "expected bridge <G0 version>, found 0.0.0-simulated" + injected);
            case LOAD_FAILED -> EngineStatus.of(state, "The Ooze native library failed to load.", "java.lang.UnsatisfiedLinkError: simulated" + injected);
            case LOADED -> throw new AssertionError("unreachable");
        };
    }

    @Override
    public DevProbeResult developerProbe() {
        throw new IllegalStateException("Fault-injected connector has no native engine.");
    }

    @Override
    public void close() {}
}
