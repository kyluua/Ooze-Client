package io.github.kyluua.ooze.bridge;

import java.nio.ByteBuffer;

/** Trusted synchronous boundary; callers grant exclusive buffer access during each call. */
final class NativeBindings {
    static native int abiVersion();
    static native long capabilities();
    static native long create(int expectedAbi, int flags);
    static native int destroy(long handle);
    static native int submit(long handle, ByteBuffer input, int byteLength);
    static native int renderFixture(long handle, long frameId, ByteBuffer output, int byteCapacity);
    static native int probePanic();

    private NativeBindings() {}
}
