package io.github.kyluua.ooze.bridge;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;

/** Reusable, thread-confined input encoder. Coordinates are physical framebuffer pixels. */
public final class InputBatch {
    private final Thread owner = Thread.currentThread();
    private final ByteBuffer bytes;
    private int count;

    public InputBatch(int maxRecords) {
        if (maxRecords < 1 || maxRecords > BridgeProtocol.MAX_INPUT_RECORDS) {
            throw new IllegalArgumentException("Input capacity must be 1..1024 records");
        }
        bytes = ByteBuffer.allocateDirect(16 + maxRecords * 24).order(ByteOrder.LITTLE_ENDIAN);
        clear();
    }

    public InputBatch clear() {
        checkThread();
        bytes.clear();
        bytes.putInt(0x4e495a4f); // OZIN in little endian.
        bytes.putInt(BridgeProtocol.ABI_VERSION).putInt(0).putInt(24);
        count = 0;
        return this;
    }

    public InputBatch resize(int width, int height, float uiScale) {
        return append(1, width, height, uiScale, 0);
    }

    public InputBatch pointerMove(float x, float y) {
        return append(2, 0, 0, x, y);
    }

    public InputBatch pointerButton(int button, boolean pressed) {
        return append(3, button, pressed ? 1 : 0, 0, 0);
    }

    public InputBatch reducedMotion(boolean enabled) {
        return append(4, enabled ? 1 : 0, 0, 0, 0);
    }

    private InputBatch append(int kind, int a, int b, float x, float y) {
        checkThread();
        if (bytes.remaining() < 24) {
            throw new IllegalStateException("Input batch is full; submit and clear it");
        }
        bytes.putInt(kind).putInt(a).putInt(b).putFloat(x).putFloat(y).putInt(0);
        bytes.putInt(8, ++count);
        return this;
    }

    ByteBuffer bytes() {
        checkThread();
        return bytes;
    }

    int byteLength() {
        checkThread();
        return bytes.position();
    }

    private void checkThread() {
        if (Thread.currentThread() != owner) {
            throw new IllegalStateException("InputBatch belongs to another thread");
        }
    }
}
