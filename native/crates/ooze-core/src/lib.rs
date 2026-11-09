//! Minecraft-independent G0 protocol and deterministic developer fixture.
#![forbid(unsafe_code)]

pub const ABI_VERSION: i32 = 1;
pub const DEV_FIXTURE: u32 = 1;
pub const INPUT_HEADER_BYTES: usize = 16;
pub const INPUT_RECORD_BYTES: usize = 24;
pub const MAX_INPUT_RECORDS: usize = 1024;
pub const MAX_INPUT_BYTES: usize = INPUT_HEADER_BYTES + MAX_INPUT_RECORDS * INPUT_RECORD_BYTES;
pub const FRAME_HEADER_BYTES: usize = 32;
pub const VERTEX_BYTES: usize = 12;
pub const FIXTURE_FRAME_BYTES: usize = FRAME_HEADER_BYTES + 6 * VERTEX_BYTES;

/// Stable negative bridge status codes. Successful handles/counts are positive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum Error {
    AbiMismatch = -1,
    InvalidArgument = -2,
    InvalidHandle = -3,
    BufferTooSmall = -4,
    WrongThread = -5,
    Panic = -6,
    Unsupported = -7,
    Limit = -8,
    Internal = -9,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Session {
    flags: u32,
    width: u32,
    height: u32,
    scale: f32,
    pointer_x: f32,
    pointer_y: f32,
    pressed: bool,
    reduced_motion: bool,
}

impl Session {
    pub fn new(expected_abi: i32, flags: u32) -> Result<Self, Error> {
        if expected_abi != ABI_VERSION {
            return Err(Error::AbiMismatch);
        }
        if flags & !DEV_FIXTURE != 0 {
            return Err(Error::InvalidArgument);
        }
        Ok(Self {
            flags,
            width: 1,
            height: 1,
            scale: 1.0,
            pointer_x: 0.0,
            pointer_y: 0.0,
            pressed: false,
            reduced_motion: false,
        })
    }

    /// A rejected batch never applies a prefix of its events.
    pub fn submit(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() < INPUT_HEADER_BYTES || bytes.len() > MAX_INPUT_BYTES {
            return Err(Error::InvalidArgument);
        }
        if &bytes[..4] != b"OZIN" || read_u32(bytes, 4) != ABI_VERSION as u32 {
            return Err(Error::AbiMismatch);
        }
        let count = read_u32(bytes, 8) as usize;
        if count > MAX_INPUT_RECORDS || read_u32(bytes, 12) != INPUT_RECORD_BYTES as u32 {
            return Err(Error::InvalidArgument);
        }
        if bytes.len() != INPUT_HEADER_BYTES + count * INPUT_RECORD_BYTES {
            return Err(Error::InvalidArgument);
        }
        let mut next = *self;
        for record in bytes[INPUT_HEADER_BYTES..]
            .as_chunks::<INPUT_RECORD_BYTES>()
            .0
        {
            let kind = read_u32(record, 0);
            let a = read_u32(record, 4);
            let b = read_u32(record, 8);
            let x = f32::from_bits(read_u32(record, 12));
            let y = f32::from_bits(read_u32(record, 16));
            if read_u32(record, 20) != 0 || !x.is_finite() || !y.is_finite() {
                return Err(Error::InvalidArgument);
            }
            match kind {
                1 if (1..=16384).contains(&a)
                    && (1..=16384).contains(&b)
                    && (0.125..=8.0).contains(&x)
                    && y == 0.0 =>
                {
                    next.width = a;
                    next.height = b;
                    next.scale = x;
                }
                2 if a == 0 && b == 0 => {
                    next.pointer_x = x;
                    next.pointer_y = y;
                }
                3 if a <= 7 && b <= 1 && x == 0.0 && y == 0.0 => {
                    if a == 0 {
                        next.pressed = b != 0;
                    }
                }
                4 if a <= 1 && b == 0 && x == 0.0 && y == 0.0 => {
                    next.reduced_motion = a != 0;
                }
                _ => return Err(Error::InvalidArgument),
            }
        }
        *self = next;
        Ok(())
    }

    /// Writes only into caller-owned memory. No renderer or native pointer escapes.
    pub fn write_fixture(&self, frame_id: u64, out: &mut [u8]) -> Result<usize, Error> {
        if self.flags & DEV_FIXTURE == 0 {
            return Err(Error::Unsupported);
        }
        if out.len() < FIXTURE_FRAME_BYTES {
            return Err(Error::BufferTooSmall);
        }
        let out = &mut out[..FIXTURE_FRAME_BYTES];
        out[..4].copy_from_slice(b"OZFR");
        put_u32(out, 4, ABI_VERSION as u32);
        put_u32(out, 8, FIXTURE_FRAME_BYTES as u32);
        put_u32(out, 12, DEV_FIXTURE);
        out[16..24].copy_from_slice(&frame_id.to_le_bytes());
        put_u32(out, 24, 6);
        put_u32(out, 28, VERTEX_BYTES as u32);
        // Diagnostic geometry, not a product component or the Ooze material painter.
        let w = (32.0 * self.scale).min(self.width as f32);
        let h = (24.0 * self.scale).min(self.height as f32);
        let x = self.pointer_x.clamp(0.0, self.width as f32 - w);
        let y = self.pointer_y.clamp(0.0, self.height as f32 - h);
        let argb = if self.pressed { 0xff171923 } else { 0xfff8f9ff };
        let vertices = [
            (x, y),
            (x + w, y),
            (x + w, y + h),
            (x, y),
            (x + w, y + h),
            (x, y + h),
        ];
        for (i, (vx, vy)) in vertices.into_iter().enumerate() {
            let offset = FRAME_HEADER_BYTES + i * VERTEX_BYTES;
            put_u32(out, offset, vx.to_bits());
            put_u32(out, offset + 4, vy.to_bits());
            put_u32(out, offset + 8, argb);
        }
        Ok(FIXTURE_FRAME_BYTES)
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("validated record"),
    )
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch(records: &[(u32, u32, u32, f32, f32)]) -> Vec<u8> {
        let mut out = vec![0; INPUT_HEADER_BYTES + records.len() * INPUT_RECORD_BYTES];
        out[..4].copy_from_slice(b"OZIN");
        put_u32(&mut out, 4, 1);
        put_u32(&mut out, 8, records.len() as u32);
        put_u32(&mut out, 12, 24);
        for (i, &(kind, a, b, x, y)) in records.iter().enumerate() {
            for (j, value) in [kind, a, b, x.to_bits(), y.to_bits(), 0]
                .into_iter()
                .enumerate()
            {
                put_u32(&mut out, 16 + i * 24 + j * 4, value);
            }
        }
        out
    }

    #[test]
    fn fixture_is_exact_and_old_frames_are_independent() {
        let mut session = Session::new(1, DEV_FIXTURE).unwrap();
        session
            .submit(&batch(&[(1, 1920, 1080, 1.0, 0.0), (2, 0, 0, 12.0, 34.0)]))
            .unwrap();
        let mut old = [0; FIXTURE_FRAME_BYTES];
        assert_eq!(session.write_fixture(7, &mut old), Ok(104));
        assert_eq!(&old[..4], b"OZFR");
        assert_eq!(f32::from_bits(read_u32(&old, 32)), 12.0);
        assert_eq!(f32::from_bits(read_u32(&old, 36)), 34.0);
        assert_eq!(read_u32(&old, 40), 0xfff8f9ff);
        session.submit(&batch(&[(3, 0, 1, 0.0, 0.0)])).unwrap();
        let mut new = [0; FIXTURE_FRAME_BYTES];
        session.write_fixture(8, &mut new).unwrap();
        assert_eq!(read_u32(&new, 40), 0xff171923);
        assert_eq!(read_u32(&old, 40), 0xfff8f9ff);
    }

    #[test]
    fn invalid_batches_are_atomic_and_never_panic() {
        let mut session = Session::new(1, DEV_FIXTURE).unwrap();
        let original = session;
        let valid = batch(&[(1, 1920, 1080, 2.0, 0.0), (2, 0, 0, f32::NAN, 0.0)]);
        assert_eq!(session.submit(&valid), Err(Error::InvalidArgument));
        assert_eq!(session, original);
        for length in 0..valid.len() {
            assert!(session.submit(&valid[..length]).is_err());
            assert_eq!(session, original);
        }
        // Deterministic malformed-input corpus; arbitrary bytes cannot bypass bounds checks.
        let mut seed = 1_u32;
        for length in 0..512 {
            let mut bytes = vec![0; length];
            for byte in &mut bytes {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                *byte = (seed >> 24) as u8;
            }
            assert!(session.submit(&bytes).is_err());
        }
    }

    #[test]
    fn short_output_and_unavailable_fixture_do_not_write() {
        let mut out = [0xa5; FIXTURE_FRAME_BYTES];
        let fixture = Session::new(1, DEV_FIXTURE).unwrap();
        assert_eq!(
            fixture.write_fixture(0, &mut out[..103]),
            Err(Error::BufferTooSmall)
        );
        assert_eq!(out, [0xa5; FIXTURE_FRAME_BYTES]);
        assert_eq!(
            Session::new(1, 0).unwrap().write_fixture(0, &mut out),
            Err(Error::Unsupported)
        );
        assert_eq!(Session::new(2, 0), Err(Error::AbiMismatch));
        assert_eq!(Session::new(1, 2), Err(Error::InvalidArgument));
    }

    #[test]
    fn viewport_clamping_and_strict_record_validation() {
        let mut session = Session::new(1, DEV_FIXTURE).unwrap();
        session
            .submit(&batch(&[(1, 8, 4, 8.0, 0.0), (2, 0, 0, f32::MAX, -1.0)]))
            .unwrap();
        let mut out = [0; FIXTURE_FRAME_BYTES];
        session.write_fixture(0, &mut out).unwrap();
        assert_eq!(read_u32(&out, 32), 0);
        assert_eq!(read_u32(&out, 36), 0);
        for record in [
            (1, 0, 1, 1.0, 0.0),
            (1, 1, 1, f32::INFINITY, 0.0),
            (3, 0, 2, 0.0, 0.0),
            (4, 2, 0, 0.0, 0.0),
            (99, 0, 0, 0.0, 0.0),
        ] {
            assert_eq!(
                session.submit(&batch(&[record])),
                Err(Error::InvalidArgument)
            );
        }
        let mut bytes = batch(&[]);
        put_u32(&mut bytes, 8, u32::MAX);
        assert_eq!(session.submit(&bytes), Err(Error::InvalidArgument));
    }
}
