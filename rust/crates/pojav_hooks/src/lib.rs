use jni::objects::JClass;
use jni::JNIEnv;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::atomic::{AtomicPtr, Ordering};

static READY_HANDLE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

#[no_mangle]
pub extern "C" fn app__pojav_linkerhook_pass_handles(
    data: *mut c_void,
    _android_dlopen_ext: *mut c_void,
    _android_get_exported_namespace: *mut c_void,
) {
    READY_HANDLE.store(data, Ordering::SeqCst);
}

#[no_mangle]
pub unsafe extern "C" fn android_dlopen_ext(
    filename: *const c_char,
    flags: c_int,
    _extinfo: *const c_void,
) -> *mut c_void {
    if !filename.is_null() {
        if let Ok(name) = CStr::from_ptr(filename).to_str() {
            if name.contains("vulkan.") {
                return READY_HANDLE.load(Ordering::SeqCst);
            }
        }
    }
    libc::dlopen(filename, flags)
}

#[no_mangle]
pub extern "C" fn atrace_get_enabled_tags() -> u64 {
    0
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_utils_JREUtils_initializeHooks(
    _env: JNIEnv,
    _class: JClass,
) {}
