//! Per-frame statistics for HUD modules, with fixed-size storage (no allocation per frame).

/// Clicks per second over a sliding 1-second window. Display only: Ooze never generates input.
#[derive(Clone, Debug)]
pub struct CpsCounter {
    /// Click timestamps in milliseconds, oldest first, as a ring.
    times: [u64; Self::CAP],
    head: usize,
    len: usize,
}

impl Default for CpsCounter {
    fn default() -> Self {
        Self {
            times: [0; Self::CAP],
            head: 0,
            len: 0,
        }
    }
}

impl CpsCounter {
    /// Above any human click rate; older clicks fall off when full.
    pub const CAP: usize = 64;
    pub const WINDOW_MS: u64 = 1000;

    pub fn click(&mut self, now_ms: u64) {
        let idx = (self.head + self.len) % Self::CAP;
        self.times[idx] = now_ms;
        if self.len == Self::CAP {
            self.head = (self.head + 1) % Self::CAP;
        } else {
            self.len += 1;
        }
    }

    /// Clicks within the last second at `now_ms`; expired clicks are dropped.
    pub fn cps(&mut self, now_ms: u64) -> u32 {
        while self.len > 0 && now_ms.saturating_sub(self.times[self.head]) >= Self::WINDOW_MS {
            self.head = (self.head + 1) % Self::CAP;
            self.len -= 1;
        }
        self.len as u32
    }
}

/// Rolling frame-time window: average FPS, average frame time and the 1% low.
#[derive(Clone, Debug)]
pub struct FrameStats {
    ms: [f32; Self::CAP],
    scratch: [f32; Self::CAP],
    head: usize,
    len: usize,
    window: usize,
}

impl FrameStats {
    pub const CAP: usize = 1200;

    /// `window` frames (clamped to 1..=CAP), for example 3 s at the current frame rate.
    pub fn new(window: usize) -> Self {
        Self {
            ms: [0.0; Self::CAP],
            scratch: [0.0; Self::CAP],
            head: 0,
            len: 0,
            window: window.clamp(1, Self::CAP),
        }
    }

    pub fn push(&mut self, frame_ms: f32) {
        if !frame_ms.is_finite() || frame_ms <= 0.0 {
            return;
        }
        let idx = (self.head + self.len) % Self::CAP;
        self.ms[idx] = frame_ms;
        if self.len == self.window {
            self.head = (self.head + 1) % Self::CAP;
        } else {
            self.len += 1;
        }
    }

    pub fn samples(&self) -> usize {
        self.len
    }

    pub fn avg_ms(&self) -> Option<f32> {
        (self.len > 0).then(|| self.iter().sum::<f32>() / self.len as f32)
    }

    pub fn avg_fps(&self) -> Option<f32> {
        self.avg_ms().map(|ms| 1000.0 / ms)
    }

    /// FPS equivalent of the slowest 1% of frames (at least one frame).
    pub fn low_1pct_fps(&mut self) -> Option<f32> {
        if self.len == 0 {
            return None;
        }
        let n = self.len;
        for (i, v) in (0..n)
            .map(|i| self.ms[(self.head + i) % Self::CAP])
            .enumerate()
        {
            self.scratch[i] = v;
        }
        let worst = (n / 100).max(1);
        let slice = &mut self.scratch[..n];
        slice.select_nth_unstable_by(n - worst, |a, b| a.total_cmp(b));
        let slow = &slice[n - worst..];
        Some(1000.0 / (slow.iter().sum::<f32>() / worst as f32))
    }

    fn iter(&self) -> impl Iterator<Item = f32> + '_ {
        (0..self.len).map(|i| self.ms[(self.head + i) % Self::CAP])
    }
}

/// Cardinal facing from Minecraft yaw (0 = south, 90 = west, 180 = north, 270 = east).
pub fn facing(yaw: f32) -> (&'static str, &'static str) {
    let y = yaw.rem_euclid(360.0);
    match ((y + 45.0) / 90.0) as u32 % 4 {
        0 => ("facing.south", "+Z"),
        1 => ("facing.west", "-X"),
        2 => ("facing.north", "-Z"),
        _ => ("facing.east", "+X"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cps_counts_a_sliding_second() {
        let mut c = CpsCounter::default();
        for t in [0, 100, 200, 900] {
            c.click(t);
        }
        assert_eq!(c.cps(950), 4);
        assert_eq!(c.cps(1050), 3, "the click at 0 ms expired");
        assert_eq!(c.cps(5000), 0);
        for t in 0..200 {
            c.click(6000 + t);
        }
        assert_eq!(c.cps(6200), CpsCounter::CAP as u32, "bounded storage");
    }

    #[test]
    fn frame_stats_average_and_one_percent_low() {
        let mut s = FrameStats::new(200);
        for _ in 0..198 {
            s.push(10.0);
        }
        s.push(50.0);
        s.push(50.0);
        let avg = s.avg_fps().unwrap();
        assert!((avg - 1000.0 / 10.4).abs() < 0.01, "{avg}");
        assert!((s.low_1pct_fps().unwrap() - 20.0).abs() < 0.01);
        for _ in 0..300 {
            s.push(5.0);
        }
        assert_eq!(s.samples(), 200, "window bounded");
        assert!((s.avg_fps().unwrap() - 200.0).abs() < 0.01);
        s.push(f32::NAN);
        s.push(-1.0);
        assert_eq!(s.samples(), 200, "invalid frame times are ignored");
    }

    #[test]
    fn facing_follows_minecraft_yaw() {
        assert_eq!(facing(0.0).0, "facing.south");
        assert_eq!(facing(90.0).0, "facing.west");
        assert_eq!(facing(-180.0).0, "facing.north");
        assert_eq!(facing(-90.0).0, "facing.east");
        assert_eq!(facing(44.0).0, "facing.south");
        assert_eq!(facing(46.0).0, "facing.west");
    }
}
