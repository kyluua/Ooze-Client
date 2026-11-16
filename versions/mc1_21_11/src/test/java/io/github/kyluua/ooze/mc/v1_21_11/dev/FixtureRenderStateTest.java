package io.github.kyluua.ooze.mc.v1_21_11.dev;

import static org.junit.jupiter.api.Assertions.assertTrue;

import org.joml.Matrix3x2f;
import org.junit.jupiter.api.Test;

/** Both windings are normalised to vanilla's front-facing order (GUI pipeline culls back faces). */
class FixtureRenderStateTest {

    @Test
    void clockwiseAndCounterClockwiseTrianglesAreDetected() {
        float[] xy = {20, 40, 20, 110, 110, 110, 120, 40, 210, 110, 120, 110};
        int[] argb = {-1, -1, -1, -1, -1, -1};
        FixtureRenderState s = new FixtureRenderState(new Matrix3x2f(), xy, argb, null);
        assertTrue(s.signedArea(0, 1, 2) < 0, "vanilla winding (front-facing) has negative area");
        assertTrue(s.signedArea(3, 4, 5) > 0, "the opposite winding is detected for re-winding");
    }
}
