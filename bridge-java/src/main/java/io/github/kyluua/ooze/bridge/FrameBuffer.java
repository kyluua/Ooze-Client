package io.github.kyluua.ooze.bridge;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.concurrent.atomic.AtomicBoolean;

/** Java-owned output storage. Reuse is forbidden until the renderer closes its lease. */
public final class FrameBuffer {
    private final ByteBuffer bytes = ByteBuffer.allocateDirect(BridgeProtocol.FIXTURE_FRAME_BYTES)
            .order(ByteOrder.LITTLE_ENDIAN);
    private boolean leased;

    synchronized ByteBuffer beginWrite() {
        if (leased) {
            throw new IllegalStateException("Frame buffer is still leased by a deferred renderer");
        }
        leased = true;
        return bytes;
    }

    synchronized void release() {
        leased = false;
    }

    FrameLease finishWrite(int byteLength) {
        ByteBuffer view = bytes.asReadOnlyBuffer().order(ByteOrder.LITTLE_ENDIAN);
        view.position(0).limit(byteLength);
        if (byteLength != 104 || view.getInt(0) != 0x52465a4f
                || view.getInt(4) != BridgeProtocol.ABI_VERSION || view.getInt(8) != byteLength
                || view.getInt(12) != BridgeProtocol.DEV_FIXTURE
                || view.getInt(24) != 6 || view.getInt(28) != 12) {
            throw new IllegalStateException("Native frame violated ABI v1");
        }
        return new FrameLease(this, view);
    }

    /**
     * Keep this lease alive through the actual deferred vertex consumption. Closing permits
     * overwrite: all ByteBuffer views obtained from the lease must then be discarded.
     */
    public static final class FrameLease implements AutoCloseable {
        private final FrameBuffer storage;
        private final ByteBuffer bytes;
        private final AtomicBoolean closed = new AtomicBoolean();

        private FrameLease(FrameBuffer storage, ByteBuffer bytes) {
            this.storage = storage;
            this.bytes = bytes;
        }

        public ByteBuffer bytes() {
            if (closed.get()) {
                throw new IllegalStateException("Frame lease is closed");
            }
            return bytes.asReadOnlyBuffer().order(ByteOrder.LITTLE_ENDIAN);
        }

        @Override
        public void close() {
            if (closed.compareAndSet(false, true)) {
                storage.release();
            }
        }
    }
}
