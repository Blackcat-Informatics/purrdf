// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The calling thread's stack bounds, read from the operating system once per thread.
//!
//! [`stack_floor`] returns the lowest address the calling thread's stack may reach — the
//! value [`super::refresh`] caches as the thread-local floor — or `None` where this target
//! exposes no way to ask, in which case the measurement stays inactive exactly as it does
//! before the first successful read on a supported target: see the [crate
//! documentation](super).
//!
//! * **Linux, Android, NetBSD**: `pthread_getattr_np` fills a `pthread_attr_t` for the
//!   calling thread, and `pthread_attr_getstack` reads the low end of the stack it
//!   describes out of that.
//! * **FreeBSD, DragonFly BSD**: the same shape, through `pthread_attr_get_np` — the name
//!   these two give the same call.
//! * **OpenBSD**: `pthread_stackseg_np` reports the stack as a `{top, size}` pair; the low
//!   end is their difference.
//! * **macOS, iOS**: `pthread_get_stackaddr_np` reports the stack's *high* end and
//!   `pthread_get_stacksize_np` its size; the low end is again their difference.
//! * **Windows**: `GetCurrentThreadStackLimits` reports the low and high bounds directly.
//! * Every other target — every other Unix `libc` does not bind one of the calls above for
//!   (illumos, Solaris, Haiku, the GNU/Hurd, ...), and any target this crate has not been
//!   ported to — reads nothing: [`stack_floor`] returns `None`.
//!
//! Every call here reads the *calling* thread's own bounds through its own `pthread_t` (or,
//! on Windows, no handle at all), so none of it reaches across threads; none of it is safe
//! to call from a signal handler, but nothing in this crate ever is.

use core::mem::MaybeUninit;

/// The lowest address the calling thread's stack may reach, or `None` where this target's
/// `libc` (or, on Windows, the platform SDK) exposes no way to ask.
#[cfg(any(target_os = "linux", target_os = "android", target_os = "netbsd"))]
pub(crate) fn stack_floor() -> Option<usize> {
    // SAFETY: `attr` is a plain-old-data struct with no destructor of its own, so holding
    // it uninitialized until `pthread_attr_init` fills it in is sound; every path below
    // runs `pthread_attr_destroy` on it exactly once, after `pthread_attr_init` succeeded,
    // before returning. `pthread_self` cannot fail, and `pthread_getattr_np` /
    // `pthread_attr_getstack` only ever write through the pointers this function passes,
    // which all name locals it owns.
    unsafe {
        let mut attr = MaybeUninit::<libc::pthread_attr_t>::uninit();
        if libc::pthread_attr_init(attr.as_mut_ptr()) != 0 {
            return None;
        }
        let mut attr = attr.assume_init();
        let mut stack_addr: *mut core::ffi::c_void = core::ptr::null_mut();
        let mut stack_size: usize = 0;
        let floor = (libc::pthread_getattr_np(libc::pthread_self(), &raw mut attr) == 0
            && libc::pthread_attr_getstack(
                &raw const attr,
                &raw mut stack_addr,
                &raw mut stack_size,
            ) == 0)
            .then_some(stack_addr as usize);
        libc::pthread_attr_destroy(&raw mut attr);
        floor
    }
}

/// The lowest address the calling thread's stack may reach, or `None` where this target's
/// `libc` exposes no way to ask.
#[cfg(any(target_os = "freebsd", target_os = "dragonfly"))]
pub(crate) fn stack_floor() -> Option<usize> {
    // SAFETY: as the Linux/Android/NetBSD case above; `pthread_attr_get_np` is this
    // family's name for the same "attributes of an already-running thread" call
    // `pthread_getattr_np` is elsewhere, with the same output contract.
    unsafe {
        let mut attr = MaybeUninit::<libc::pthread_attr_t>::uninit();
        if libc::pthread_attr_init(attr.as_mut_ptr()) != 0 {
            return None;
        }
        let mut attr = attr.assume_init();
        let mut stack_addr: *mut core::ffi::c_void = core::ptr::null_mut();
        let mut stack_size: usize = 0;
        let floor = (libc::pthread_attr_get_np(libc::pthread_self(), &raw mut attr) == 0
            && libc::pthread_attr_getstack(
                &raw const attr,
                &raw mut stack_addr,
                &raw mut stack_size,
            ) == 0)
            .then_some(stack_addr as usize);
        libc::pthread_attr_destroy(&raw mut attr);
        floor
    }
}

/// The lowest address the calling thread's stack may reach, or `None` where this target's
/// `libc` exposes no way to ask.
#[cfg(target_os = "openbsd")]
pub(crate) fn stack_floor() -> Option<usize> {
    // SAFETY: `info` is a plain-old-data struct `pthread_stackseg_np` writes wholesale
    // before this function reads any field of it, guarded by that call's own return value;
    // nothing else touches `info`.
    unsafe {
        let mut info = MaybeUninit::<libc::stack_t>::uninit();
        if libc::pthread_stackseg_np(libc::pthread_self(), info.as_mut_ptr()) != 0 {
            return None;
        }
        let info = info.assume_init();
        Some((info.ss_sp as usize).saturating_sub(info.ss_size))
    }
}

/// The lowest address the calling thread's stack may reach.
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) fn stack_floor() -> Option<usize> {
    // SAFETY: both calls take the calling thread's own `pthread_t`, which `pthread_self`
    // always returns successfully, and return their result by value rather than writing
    // through a pointer, so there is nothing here for either call to leave uninitialized.
    unsafe {
        let thread = libc::pthread_self();
        let top = libc::pthread_get_stackaddr_np(thread) as usize;
        let size = libc::pthread_get_stacksize_np(thread);
        Some(top.saturating_sub(size))
    }
}

/// The lowest address the calling thread's stack may reach.
#[cfg(windows)]
pub(crate) fn stack_floor() -> Option<usize> {
    let mut low: usize = 0;
    let mut high: usize = 0;
    // SAFETY: both out-parameters are plain `usize` locals this function owns and holds
    // for the whole call; `GetCurrentThreadStackLimits` writes through them and nothing
    // else, and reports the calling thread's own limits, so it cannot fail.
    unsafe {
        windows_sys::Win32::System::Threading::GetCurrentThreadStackLimits(
            &raw mut low,
            &raw mut high,
        );
    }
    let _ = high;
    Some(low)
}

/// `None`: this target's `libc` (Illumos, Solaris, Haiku, the GNU/Hurd, ...), or any
/// target this crate has not been ported to, exposes no call this module knows to read the
/// thread's stack bounds with, so the measurement this crate makes stays inactive.
#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "netbsd",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "macos",
    target_os = "ios",
    windows,
)))]
pub(crate) fn stack_floor() -> Option<usize> {
    None
}

/// Tests exercised only where [`stack_floor`] is expected to read a real bound — the same
/// target set the implementations above are gated on — so they fail on a target this
/// module has a call for, and are silently absent (nothing to assert) everywhere else.
#[cfg(all(
    test,
    any(
        target_os = "linux",
        target_os = "android",
        target_os = "netbsd",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "macos",
        target_os = "ios",
        windows,
    )
))]
mod tests {
    use super::stack_floor;
    use crate::stack_pointer;

    /// Run `body` on a fresh thread with `bytes` of stack, so the read is that thread's
    /// own bound, not the harness's.
    fn on_thread<T: Send + 'static>(bytes: usize, body: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(bytes)
            .spawn(body)
            .expect("spawn")
            .join()
            .expect("join")
    }

    /// `levels` frames of 4 KiB below the caller, then `at_bottom`.
    fn deeper<T>(levels: usize, at_bottom: &dyn Fn() -> T) -> T {
        if levels == 0 {
            return at_bottom();
        }
        let frame = core::hint::black_box([0u8; 4096]);
        let value = deeper(levels - 1, at_bottom);
        core::hint::black_box(&frame);
        value
    }

    /// A supported platform reads a floor: `Some`, strictly below the stack pointer near
    /// the thread's entry point (so the headroom above it is positive), and nowhere near
    /// the enormous figure a floor of `0` (or another wrong, tiny value) would produce —
    /// the libc may hand a thread more than the size it asked for, but never orders of
    /// magnitude more.
    #[test]
    fn stack_floor_is_some_and_bounds_positive_headroom() {
        const BYTES: usize = 2 * 1024 * 1024;
        let (sp, floor) = on_thread(BYTES, || (stack_pointer(), stack_floor()));
        let floor = floor.expect("a supported platform reads a floor");
        assert!(floor < sp, "floor {floor:#x} below sp {sp:#x}");
        let headroom = sp - floor;
        assert!(headroom > 0, "positive headroom");
        assert!(
            headroom < BYTES * 4,
            "{headroom} bytes of headroom in a {BYTES}-byte thread — a floor near 0 would \
             report far more"
        );
    }

    /// The floor names a fixed point of the thread's stack, not the calling frame: read
    /// again sixteen 4 KiB frames deeper, it is unchanged, so the headroom computed from it
    /// has fallen by what those frames used.
    #[test]
    fn stack_floor_is_fixed_so_headroom_falls_with_depth() {
        let (top_floor, top_sp, deep_floor, deep_sp) = on_thread(2 * 1024 * 1024, || {
            let top = (stack_floor(), stack_pointer());
            let (deep_floor, deep_sp) = deeper(16, &|| (stack_floor(), stack_pointer()));
            (top.0, top.1, deep_floor, deep_sp)
        });
        assert_eq!(
            top_floor, deep_floor,
            "the floor belongs to the thread, not the frame that read it"
        );
        let floor = top_floor.expect("a supported platform reads a floor");
        let top_headroom = top_sp - floor;
        let deep_headroom = deep_sp - floor;
        assert!(
            deep_headroom < top_headroom,
            "{deep_headroom} bytes deep, {top_headroom} at the top"
        );
        let used = top_headroom - deep_headroom;
        assert!(
            (16 * 4096..16 * 4096 + 32 * 1024).contains(&used),
            "sixteen 4 KiB frames used {used} bytes"
        );
    }

    /// Two threads spawned with different `stack_size`s each report a floor within a sane
    /// range of what they asked for — proof the figure tracks the thread's own request
    /// rather than, say, always answering the main thread's or a fixed guess.
    #[test]
    fn stack_floor_tracks_a_thread_s_requested_stack_size() {
        for bytes in [512 * 1024, 3 * 1024 * 1024] {
            let (sp, floor) = on_thread(bytes, || (stack_pointer(), stack_floor()));
            let floor = floor.expect("a supported platform reads a floor");
            let headroom = sp - floor;
            assert!(
                (bytes.saturating_sub(64 * 1024)..bytes * 4).contains(&headroom),
                "{headroom} bytes of headroom in a thread asked for {bytes}"
            );
        }
    }
}
