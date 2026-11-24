//! macOS notification for changes to the selected keyboard input source.
//!
//! The system shortcut is user-configurable, so key combinations are not a
//! reliable signal. The Carbon distributed notification is delivered for
//! changes made by shortcuts, the Input menu, or the Globe key.

use std::ffi::{CStr, c_char, c_void};
use std::sync::Once;
use std::sync::atomic::{AtomicU64, Ordering};

static REGISTER: Once = Once::new();
static GENERATION: AtomicU64 = AtomicU64::new(0);
const UTF8: u32 = 0x0800_0100;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    static kTISNotifySelectedKeyboardInputSourceChanged: *const c_void;
    static kTISPropertyInputSourceID: *const c_void;
    fn TISCopyCurrentKeyboardInputSource() -> *const c_void;
    fn TISGetInputSourceProperty(source: *const c_void, key: *const c_void) -> *const c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFNotificationCenterGetDistributedCenter() -> *mut c_void;
    fn CFNotificationCenterAddObserver(
        center: *mut c_void,
        observer: *const c_void,
        callback: Option<
            unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                *const c_void,
                *const c_void,
                *const c_void,
            ),
        >,
        name: *const c_void,
        object: *const c_void,
        suspension_behavior: isize,
    );
    fn CFStringGetCString(
        string: *const c_void,
        buffer: *mut c_char,
        buffer_size: isize,
        encoding: u32,
    ) -> u8;
    fn CFRelease(value: *const c_void);
}

unsafe extern "C" fn source_changed(
    _center: *mut c_void,
    _observer: *mut c_void,
    _name: *const c_void,
    _object: *const c_void,
    _user_info: *const c_void,
) {
    GENERATION.fetch_add(1, Ordering::Release);
}

/// Register once for the process lifetime and return the number of changes.
pub fn generation() -> u64 {
    REGISTER.call_once(|| unsafe {
        let center = CFNotificationCenterGetDistributedCenter();
        if !center.is_null() {
            CFNotificationCenterAddObserver(
                center,
                (&GENERATION as *const AtomicU64).cast(),
                Some(source_changed),
                kTISNotifySelectedKeyboardInputSourceChanged,
                std::ptr::null(),
                1, // Drop notifications while the app is suspended.
            );
        }
    });
    GENERATION.load(Ordering::Acquire)
}

/// ID of the currently selected source, used to discard duplicate notices.
pub fn current_source_id() -> Option<String> {
    unsafe {
        let source = TISCopyCurrentKeyboardInputSource();
        if source.is_null() {
            return None;
        }
        let id = TISGetInputSourceProperty(source, kTISPropertyInputSourceID);
        let mut buffer = [0_i8; 512];
        let result = if !id.is_null()
            && CFStringGetCString(id, buffer.as_mut_ptr(), buffer.len() as isize, UTF8) != 0
        {
            CStr::from_ptr(buffer.as_ptr())
                .to_str()
                .ok()
                .map(str::to_owned)
        } else {
            None
        };
        CFRelease(source);
        result
    }
}
