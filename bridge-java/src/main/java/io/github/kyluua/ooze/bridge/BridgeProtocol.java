package io.github.kyluua.ooze.bridge;

/** Constants for the deliberately small G0 subset of ABI v1. */
public final class BridgeProtocol {
    public static final int ABI_VERSION = 1;
    public static final String LIBRARY_VERSION = "0.1.0-g0";
    public static final int DEV_FIXTURE = 1;
    public static final long CAP_FIXTURE = 1;
    public static final long CAP_BATCHED_INPUT = 2;
    public static final long CAP_FAULT_INJECTION = 4;
    public static final int INPUT_HEADER_BYTES = 16;
    public static final int INPUT_RECORD_BYTES = 24;
    public static final int MAX_INPUT_RECORDS = 1024;
    public static final int FIXTURE_FRAME_BYTES = 104;
    public static final int ABI_MISMATCH = -1;
    public static final int INVALID_ARGUMENT = -2;
    public static final int INVALID_HANDLE = -3;
    public static final int BUFFER_TOO_SMALL = -4;
    public static final int WRONG_THREAD = -5;
    public static final int PANIC = -6;
    public static final int UNSUPPORTED = -7;
    public static final int LIMIT = -8;
    public static final int INTERNAL = -9;

    private BridgeProtocol() {}
}
