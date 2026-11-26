use crate::environ::POJAV_ENVIRON;
use jni::objects::{JClass, JObject};
use jni::sys::jlong;
use jni::JNIEnv;
use std::ffi::{c_int, c_void, CString};

#[link(name = "android")]
extern "C" {
    fn ANativeWindow_fromSurface(env: *mut jni::sys::JNIEnv, surface: jni::sys::jobject) -> *mut c_void;
    fn ANativeWindow_release(window: *mut c_void);
    fn ANativeWindow_acquire(window: *mut c_void);
    fn ANativeWindow_getWidth(window: *mut c_void) -> i32;
    fn ANativeWindow_getHeight(window: *mut c_void) -> i32;
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_utils_JREUtils_setupBridgeWindow(
    env: JNIEnv,
    _class: JClass,
    surface: JObject,
) {
    unsafe {
        let win = ANativeWindow_fromSurface(env.get_native_interface(), surface.into_raw());
        if let Ok(mut st) = POJAV_ENVIRON.lock() {
            st.pojav_window = win;
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_utils_JREUtils_releaseBridgeWindow(
    _env: JNIEnv,
    _class: JClass,
) {
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        if !st.pojav_window.is_null() {
            unsafe { ANativeWindow_release(st.pojav_window); }
            st.pojav_window = std::ptr::null_mut();
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_vulkan_VK_getVulkanDriverHandle(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    let vk_name = CString::new("libvulkan.so").unwrap();
    let ptr = unsafe { libc::dlopen(vk_name.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL) };
    ptr as jlong
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_opengl_PojavRendererInit_nativeInitGl4esInternals(
    _env: JNIEnv,
    _class: JClass,
    _function_provider: JObject,
) {}

#[no_mangle]
pub extern "C" fn pojavInit() -> c_int {
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        if !st.pojav_window.is_null() {
            unsafe {
                ANativeWindow_acquire(st.pojav_window);
                st.saved_width = ANativeWindow_getWidth(st.pojav_window);
                st.saved_height = ANativeWindow_getHeight(st.pojav_window);
            }
            return 1;
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn pojavInitOpenGL() -> c_int { 0 }
#[no_mangle]
pub extern "C" fn pojavSetWindowHint(_hint: c_int, _value: c_int) {}
#[no_mangle]
pub extern "C" fn pojavSwapBuffers() {}
#[no_mangle]
pub extern "C" fn pojavMakeCurrent(_window: *mut c_void) {}
#[no_mangle]
pub extern "C" fn pojavCreateContext(_src: *mut c_void) -> *mut c_void {
    POJAV_ENVIRON.lock().map(|s| s.pojav_window).unwrap_or(std::ptr::null_mut())
}
#[no_mangle]
pub extern "C" fn pojavGetCurrentContext() -> *mut c_void {
    POJAV_ENVIRON.lock().map(|s| s.pojav_window).unwrap_or(std::ptr::null_mut())
}
#[no_mangle]
pub extern "C" fn pojavSwapInterval(_interval: c_int) {}
#[no_mangle]
pub extern "C" fn pojavTerminate() {}
