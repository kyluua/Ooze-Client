package io.github.kyluua.ooze.bridge;

/** A platform-thread-confined engine instance; contains no Minecraft objects or credentials. */
public final class EngineSession implements AutoCloseable {
    private final Thread owner = Thread.currentThread();
    private long handle;

    public EngineSession(int flags) {
        if (owner.isVirtual()) {
            throw new IllegalStateException("The bridge requires a stable platform thread");
        }
        if (!NativeLoader.isReady()) {
            throw new IllegalStateException("Load and verify the native library before creating a session");
        }
        long result = NativeBindings.create(BridgeProtocol.ABI_VERSION, flags);
        if (result <= 0) {
            throw new BridgeException("create", result);
        }
        handle = result;
    }

    public void submit(InputBatch batch) {
        requireOpen();
        BridgeException.check("submit", NativeBindings.submit(handle, batch.bytes(), batch.byteLength()));
    }

    public FrameBuffer.FrameLease renderFixture(long frameId, FrameBuffer destination) {
        requireOpen();
        var output = destination.beginWrite();
        try {
            int length = BridgeException.check("renderFixture",
                    NativeBindings.renderFixture(handle, frameId, output, output.capacity()));
            return destination.finishWrite(length);
        } catch (RuntimeException | Error failure) {
            destination.release();
            throw failure;
        }
    }

    private void requireOwner() {
        if (Thread.currentThread() != owner) {
            throw new IllegalStateException("EngineSession belongs to another thread");
        }
    }

    private void requireOpen() {
        requireOwner();
        if (handle == 0) {
            throw new IllegalStateException("EngineSession is closed");
        }
    }

    @Override
    public void close() {
        requireOwner();
        if (handle != 0) {
            BridgeException.check("destroy", NativeBindings.destroy(handle));
            handle = 0;
        }
    }
}
