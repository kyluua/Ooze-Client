//! Small JNI boundary. No Minecraft references, retained Java pointers, or graphics calls.
use jni::JNIEnv;
use jni::objects::{JByteBuffer, JClass};
use jni::sys::{jint, jlong};
use ooze_core::{ABI_VERSION, Error, FIXTURE_FRAME_BYTES, MAX_INPUT_BYTES, Session};
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Mutex, OnceLock};
use std::thread::{self, ThreadId};

#[unsafe(no_mangle)]
pub extern "system" fn JNI_OnLoad(
    _vm: *mut jni::sys::JavaVM,
    _reserved: *mut std::ffi::c_void,
) -> jint {
    // Direct-buffer operations require JNI 1.4+. Advertise the JNI 1.8 baseline;
    // the Java bridge itself requires Java 21 and does not construct another VM.
    guard(|| Ok(jni::sys::JNI_VERSION_1_8 as i64)) as jint
}

const CAPABILITIES: i64 = 3 | if cfg!(feature = "fault-injection") {
    4
} else {
    0
};
const MAX_SESSIONS: usize = 64;
const MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

struct Entry {
    owner: ThreadId,
    session: Session,
}

struct Registry {
    next: i64,
    entries: BTreeMap<i64, Entry>,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            next: 1,
            entries: BTreeMap::new(),
        }
    }
}

static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();

fn registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}

fn guard(f: impl FnOnce() -> Result<i64, Error>) -> i64 {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => error as i64,
        Err(_) => Error::Panic as i64,
    }
}

fn with_session<T>(
    handle: i64,
    f: impl FnOnce(&mut Session) -> Result<T, Error>,
) -> Result<T, Error> {
    let mut registry = registry().lock().map_err(|_| Error::Internal)?;
    let entry = registry
        .entries
        .get_mut(&handle)
        .ok_or(Error::InvalidHandle)?;
    if entry.owner != thread::current().id() {
        return Err(Error::WrongThread);
    }
    f(&mut entry.session)
}

fn direct_region(
    env: &mut JNIEnv,
    buffer: &JByteBuffer,
    length: jint,
    max: usize,
    writable: bool,
) -> Result<(*mut u8, usize), Error> {
    if buffer.is_null() || length < 0 || length as usize > max {
        return Err(Error::InvalidArgument);
    }
    let capacity = env
        .get_direct_buffer_capacity(buffer)
        .map_err(|_| Error::InvalidArgument)?;
    if length as usize > capacity {
        return Err(Error::InvalidArgument);
    }
    if writable
        && env
            .call_method(buffer, "isReadOnly", "()Z", &[])
            .and_then(|v| v.z())
            .map_err(|_| Error::Internal)?
    {
        return Err(Error::InvalidArgument);
    }
    let address = env
        .get_direct_buffer_address(buffer)
        .map_err(|_| Error::InvalidArgument)?;
    if address.is_null() {
        return Err(Error::InvalidArgument);
    }
    Ok((address, length as usize))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_kyluua_ooze_bridge_NativeBindings_abiVersion(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    guard(|| Ok(ABI_VERSION as i64)) as jint
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_kyluua_ooze_bridge_NativeBindings_capabilities(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    guard(|| Ok(CAPABILITIES))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_kyluua_ooze_bridge_NativeBindings_create(
    _env: JNIEnv,
    _class: JClass,
    expected_abi: jint,
    flags: jint,
) -> jlong {
    guard(|| {
        let session = Session::new(expected_abi, flags as u32)?;
        let mut registry = registry().lock().map_err(|_| Error::Internal)?;
        if registry.entries.len() >= MAX_SESSIONS || registry.next == i64::MAX {
            return Err(Error::Limit);
        }
        let handle = registry.next;
        registry.next += 1;
        registry.entries.insert(
            handle,
            Entry {
                owner: thread::current().id(),
                session,
            },
        );
        Ok(handle)
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_kyluua_ooze_bridge_NativeBindings_destroy(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    guard(|| {
        with_session(handle, |_| Ok(()))?;
        registry()
            .lock()
            .map_err(|_| Error::Internal)?
            .entries
            .remove(&handle)
            .ok_or(Error::InvalidHandle)?;
        Ok(0)
    }) as jint
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_kyluua_ooze_bridge_NativeBindings_submit(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    buffer: JByteBuffer,
    byte_length: jint,
) -> jint {
    guard(|| {
        let (address, length) =
            direct_region(&mut env, &buffer, byte_length, MAX_INPUT_BYTES, false)?;
        // SAFETY: the synchronous Java caller owns a live direct buffer and grants exclusive
        // access for this call. Bounds are checked against JVM capacity. Nothing is retained.
        let bytes = unsafe { std::slice::from_raw_parts(address.cast_const(), length) };
        with_session(handle, |session| session.submit(bytes))?;
        Ok(0)
    }) as jint
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_kyluua_ooze_bridge_NativeBindings_renderFixture(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    frame_id: jlong,
    buffer: JByteBuffer,
    byte_capacity: jint,
) -> jint {
    guard(|| {
        if frame_id < 0 {
            return Err(Error::InvalidArgument);
        }
        let (address, capacity) =
            direct_region(&mut env, &buffer, byte_capacity, MAX_OUTPUT_BYTES, true)?;
        if capacity < FIXTURE_FRAME_BYTES {
            return Err(Error::BufferTooSmall);
        }
        // SAFETY: the synchronous Java caller grants exclusive write access to a live,
        // writable direct buffer. Only 104 checked bytes are borrowed, never retained.
        let out = unsafe { std::slice::from_raw_parts_mut(address, FIXTURE_FRAME_BYTES) };
        with_session(handle, |session| {
            session.write_fixture(frame_id as u64, out)
        })
        .map(|n| n as i64)
    }) as jint
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_kyluua_ooze_bridge_NativeBindings_probePanic(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    guard(|| {
        #[cfg(feature = "fault-injection")]
        panic!("Ooze G0 controlled panic probe");
        #[cfg(not(feature = "fault-injection"))]
        Err(Error::Unsupported)
    }) as jint
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_is_translated_without_poisoning_registry() {
        assert_eq!(guard(|| panic!("test boundary")), Error::Panic as i64);
        assert_eq!(guard(|| Ok(42)), 42);
        assert!(registry().lock().is_ok());
    }
}
