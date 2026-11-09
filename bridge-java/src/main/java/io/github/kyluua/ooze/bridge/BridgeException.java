package io.github.kyluua.ooze.bridge;

/** A controlled native failure. The Fabric adapter should disable the affected Ooze session. */
public final class BridgeException extends RuntimeException {
    private static final long serialVersionUID = 1L;
    private final int status;

    BridgeException(String operation, long status) {
        super(operation + " failed with bridge status " + status);
        this.status = (int) status;
    }

    public int status() {
        return status;
    }

    static int check(String operation, int result) {
        if (result < 0) {
            throw new BridgeException(operation, result);
        }
        return result;
    }
}
