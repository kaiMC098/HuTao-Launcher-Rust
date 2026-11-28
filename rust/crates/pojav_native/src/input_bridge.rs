use crate::affinity_stdio::log_native;
use crate::environ::*;
use jni::objects::{JByteArray, JClass, JDoubleArray, JObject, JString, JValue};
use jni::sys::{
    jboolean, jbyte, jchar, jclass, jdouble, jfloat, jint, jlong, jmethodID, jobject, jstring,
    jvalue, JavaVM, JNI_FALSE, JNI_TRUE, JNI_VERSION_1_4,
};
use jni::JNIEnv;
use std::ffi::{c_int, c_void, CStr, CString};
use once_cell::sync::Lazy;
use std::sync::atomic::Ordering;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

static EVENT_WAIT: Lazy<(Mutex<bool>, Condvar)> = Lazy::new(|| (Mutex::new(false), Condvar::new()));

fn notify_event_wait() {
    let (lock, cvar) = &*EVENT_WAIT;
    if let Ok(mut ready) = lock.lock() {
        *ready = true;
        cvar.notify_all();
    }
}

unsafe fn clear_exception(raw_env: *mut jni::sys::JNIEnv) -> bool {
    if raw_env.is_null() {
        return false;
    }
    if let Some(ex_check) = (**raw_env).ExceptionCheck {
        if ex_check(raw_env) != JNI_FALSE {
            if let Some(ex_clear) = (**raw_env).ExceptionClear {
                ex_clear(raw_env);
            }
            return true;
        }
    }
    false
}

unsafe fn find_static_method(
    raw_env: *mut jni::sys::JNIEnv,
    clazz: jclass,
    name: &str,
    sig: &str,
) -> jmethodID {
    if raw_env.is_null() || clazz.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(c_name) = CString::new(name) else {
        return std::ptr::null_mut();
    };
    let Ok(c_sig) = CString::new(sig) else {
        return std::ptr::null_mut();
    };
    let Some(get_mid) = (**raw_env).GetStaticMethodID else {
        return std::ptr::null_mut();
    };
    let mid = get_mid(raw_env, clazz, c_name.as_ptr(), c_sig.as_ptr());
    if clear_exception(raw_env) {
        return std::ptr::null_mut();
    }
    mid
}

#[no_mangle]
pub unsafe extern "system" fn JNI_OnLoad(vm: *mut JavaVM, _reserved: *mut c_void) -> jint {
    let is_first_dvm_load = if let Ok(st) = POJAV_ENVIRON.lock() {
        st.dalvik_java_vm.is_null()
    } else {
        return JNI_VERSION_1_4;
    };

    if is_first_dvm_load {
        let mut bridge_global: jclass = std::ptr::null_mut();
        let mut mid_clipboard: jmethodID = std::ptr::null_mut();
        let mut mid_grab: jmethodID = std::ptr::null_mut();
        let mut mid_direct_input: jmethodID = std::ptr::null_mut();

        if let Ok(jvm) = jni::JavaVM::from_raw(vm) {
            if let Ok(env) = jvm.get_env() {
                let raw_env = env.get_raw();
                if let (Some(find_class), Some(new_global), Some(del_local)) = (
                    (**raw_env).FindClass,
                    (**raw_env).NewGlobalRef,
                    (**raw_env).DeleteLocalRef,
                ) {
                    let cls_name = CString::new("org/lwjgl/glfw/CallbackBridge").unwrap();
                    let local_cls = find_class(raw_env, cls_name.as_ptr());
                    if !local_cls.is_null() && !clear_exception(raw_env) {
                        bridge_global = new_global(raw_env, local_cls) as jclass;
                        del_local(raw_env, local_cls);
                        if !bridge_global.is_null() {
                            mid_clipboard = find_static_method(
                                raw_env,
                                bridge_global,
                                "accessAndroidClipboard",
                                "(ILjava/lang/String;)Ljava/lang/String;",
                            );
                            mid_grab = find_static_method(
                                raw_env,
                                bridge_global,
                                "onGrabStateChanged",
                                "(Z)V",
                            );
                            mid_direct_input = find_static_method(
                                raw_env,
                                bridge_global,
                                "onDirectInputEnable",
                                "()V",
                            );
                        }
                    } else {
                        clear_exception(raw_env);
                    }
                }
            }
        }

        if let Ok(mut st) = POJAV_ENVIRON.lock() {
            st.dalvik_java_vm = vm;
            st.bridge_clazz = bridge_global;
            st.method_access_android_clipboard = mid_clipboard;
            st.method_on_grab_state_changed = mid_grab;
            st.method_on_direct_input_enable = mid_direct_input;
            st.is_use_stack_queue_call = false;
            st.is_grabbing = JNI_FALSE;
        }
        log_native("[Rust Input] Saved Dalvik VM & CallbackBridge method IDs");
    } else if let Ok(mut st) = POJAV_ENVIRON.lock() {
        if st.dalvik_java_vm != vm {
            st.runtime_java_vm = vm;
            log_native("[Rust Input] Saved HotSpot Runtime JVM pointer");
        }
        st.is_grabbing = JNI_FALSE;
    }

    JNI_VERSION_1_4
}

/// Lazily resolves org/lwjgl/glfw/GLFW class, static methods, and key/mouse state ByteBuffers
/// on the HotSpot JVM thread (never inside JNI_OnLoad to avoid <clinit> re-entrancy).
unsafe fn ensure_glfw_bridge_initialized(raw_env: *mut jni::sys::JNIEnv) {
    if raw_env.is_null() {
        return;
    }
    let already_init = POJAV_ENVIRON
        .lock()
        .map(|st| !st.vm_glfw_class.is_null())
        .unwrap_or(true);
    if already_init {
        return;
    }

    let (
        Some(find_class),
        Some(new_global),
        Some(del_local),
        Some(get_fid),
        Some(get_static_obj),
        Some(get_buf_addr),
    ) = (
        (**raw_env).FindClass,
        (**raw_env).NewGlobalRef,
        (**raw_env).DeleteLocalRef,
        (**raw_env).GetStaticFieldID,
        (**raw_env).GetStaticObjectField,
        (**raw_env).GetDirectBufferAddress,
    )
    else {
        return;
    };

    let cls_name = CString::new("org/lwjgl/glfw/GLFW").unwrap();
    let local_cls = find_class(raw_env, cls_name.as_ptr());
    if local_cls.is_null() || clear_exception(raw_env) {
        return;
    }

    let mid_attrib = find_static_method(raw_env, local_cls, "glfwSetWindowAttrib", "(JII)V");
    let mid_win_size = find_static_method(raw_env, local_cls, "internalWindowSizeChanged", "(J)V");
    let mid_mon_size = find_static_method(raw_env, local_cls, "internalChangeMonitorSize", "(II)V");

    let c_key_name = CString::new("keyDownBuffer").unwrap();
    let c_mouse_name = CString::new("mouseDownBuffer").unwrap();
    let c_bb_sig = CString::new("Ljava/nio/ByteBuffer;").unwrap();

    let fid_key = get_fid(raw_env, local_cls, c_key_name.as_ptr(), c_bb_sig.as_ptr());
    let fid_mouse = get_fid(raw_env, local_cls, c_mouse_name.as_ptr(), c_bb_sig.as_ptr());
    if clear_exception(raw_env) {
        del_local(raw_env, local_cls);
        return;
    }

    let mut key_ptr: *mut jbyte = std::ptr::null_mut();
    let mut mouse_ptr: *mut jbyte = std::ptr::null_mut();

    if !fid_key.is_null() {
        let key_obj = get_static_obj(raw_env, local_cls, fid_key);
        if !key_obj.is_null() && !clear_exception(raw_env) {
            key_ptr = get_buf_addr(raw_env, key_obj) as *mut jbyte;
            del_local(raw_env, key_obj);
        }
    }
    if !fid_mouse.is_null() {
        let mouse_obj = get_static_obj(raw_env, local_cls, fid_mouse);
        if !mouse_obj.is_null() && !clear_exception(raw_env) {
            mouse_ptr = get_buf_addr(raw_env, mouse_obj) as *mut jbyte;
            del_local(raw_env, mouse_obj);
        }
    }

    let global_cls = new_global(raw_env, local_cls) as jclass;
    del_local(raw_env, local_cls);
    if global_cls.is_null() {
        return;
    }

    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        if st.vm_glfw_class.is_null() {
            st.vm_glfw_class = global_cls;
            st.method_glfw_set_window_attrib = mid_attrib;
            st.method_internal_window_size_changed = mid_win_size;
            st.method_internal_change_monitor_size = mid_mon_size;
            st.key_down_buffer = key_ptr;
            st.mouse_down_buffer = mouse_ptr;
            log_native("[Rust Input] Initialized GLFW bridge & key/mouse state buffers");
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeSetUseInputStackQueue(
    _env: JNIEnv,
    _class: JClass,
    use_queue: jboolean,
) {
    if let Ok(mut env_state) = POJAV_ENVIRON.lock() {
        env_state.is_use_stack_queue_call = use_queue != JNI_FALSE;
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeSetInputReady(
    env: JNIEnv,
    _class: JClass,
    input_ready: jboolean,
) -> jboolean {
    unsafe {
        ensure_glfw_bridge_initialized(env.get_raw());
    }
    if let Ok(mut env_state) = POJAV_ENVIRON.lock() {
        env_state.is_input_ready = input_ready != JNI_FALSE;
        if env_state.is_use_stack_queue_call {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeSetGrabbing(
    _env: JNIEnv,
    _class: JClass,
    grabbing: jboolean,
) {
    let (dalvik_vm, bridge_clazz, mid_grab) = {
        let Ok(mut st) = POJAV_ENVIRON.lock() else {
            return;
        };
        st.is_grabbing = grabbing;
        (st.dalvik_java_vm, st.bridge_clazz, st.method_on_grab_state_changed)
    };

    if dalvik_vm.is_null() || bridge_clazz.is_null() || mid_grab.is_null() {
        log_native("[Rust Input] Warning: nativeSetGrabbing skipped (Dalvik bridge not ready)");
        return;
    }

    unsafe {
        if let Ok(jvm) = jni::JavaVM::from_raw(dalvik_vm) {
            if let Ok(dvm_env) = jvm.attach_current_thread() {
                let raw_env = dvm_env.get_raw();
                if let Some(call_void) = (**raw_env).CallStaticVoidMethodA {
                    let args = [jvalue { z: grabbing }];
                    call_void(raw_env, bridge_clazz, mid_grab, args.as_ptr());
                    clear_exception(raw_env);
                }
            }
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeSendData(
    _env: JNIEnv,
    _class: JClass,
    _is_android: jboolean,
    _event_type: jint,
    _data: JString,
) {}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeSendChar(
    _env: JNIEnv,
    _class: JClass,
    codepoint: jchar,
) -> jboolean {
    let direct_call = {
        let Ok(mut st) = POJAV_ENVIRON.lock() else {
            return JNI_FALSE;
        };
        if !st.is_input_ready || st.cb_char == 0 {
            return JNI_FALSE;
        }
        if st.is_use_stack_queue_call {
            st.push_event(EVENT_TYPE_CHAR, codepoint as i32, 0, 0, 0);
            None
        } else {
            Some((st.showing_window as usize as *mut c_void, st.cb_char))
        }
    };
    if let Some((win, cb)) = direct_call {
        unsafe {
            let f: GlfwCharFun = std::mem::transmute(cb);
            f(win, codepoint as u32);
        }
    }
    JNI_TRUE
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeSendCharMods(
    _env: JNIEnv,
    _class: JClass,
    codepoint: jchar,
    mods: jint,
) -> jboolean {
    let direct_call = {
        let Ok(mut st) = POJAV_ENVIRON.lock() else {
            return JNI_FALSE;
        };
        if !st.is_input_ready || st.cb_char_mods == 0 {
            return JNI_FALSE;
        }
        if st.is_use_stack_queue_call {
            st.push_event(EVENT_TYPE_CHAR_MODS, codepoint as i32, mods, 0, 0);
            None
        } else {
            Some((st.showing_window as usize as *mut c_void, st.cb_char_mods))
        }
    };
    if let Some((win, cb)) = direct_call {
        unsafe {
            let f: GlfwCharModsFun = std::mem::transmute(cb);
            f(win, codepoint as u32, mods as c_int);
        }
    }
    JNI_TRUE
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeSendKey(
    _env: JNIEnv,
    _class: JClass,
    key: jint,
    scancode: jint,
    action: jint,
    mods: jint,
) {
    let direct_call = {
        let Ok(mut st) = POJAV_ENVIRON.lock() else {
            return;
        };
        if !st.is_input_ready {
            return;
        }
        if !st.key_down_buffer.is_null() && (31..=347).contains(&key) {
            let idx = (key - 31) as isize;
            unsafe {
                *st.key_down_buffer.offset(idx) = action as i8;
            }
        }
        if st.cb_key == 0 {
            return;
        }
        if st.is_use_stack_queue_call {
            st.push_event(EVENT_TYPE_KEY, key, scancode, action, mods);
            None
        } else {
            Some((st.showing_window as usize as *mut c_void, st.cb_key))
        }
    };
    if let Some((win, cb)) = direct_call {
        unsafe {
            let f: GlfwKeyFun = std::mem::transmute(cb);
            f(win, key as c_int, scancode as c_int, action as c_int, mods as c_int);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeSendCursorPos(
    _env: JNIEnv,
    _class: JClass,
    x: jfloat,
    y: jfloat,
) {
    let (enter_call, pos_call) = {
        let Ok(mut st) = POJAV_ENVIRON.lock() else {
            return;
        };
        if !st.is_input_ready || st.cb_cursor_pos == 0 {
            return;
        }

        let mut enter_call = None;
        if !st.is_cursor_entered && st.cb_cursor_enter != 0 {
            st.is_cursor_entered = true;
            if st.is_use_stack_queue_call {
                st.push_event(EVENT_TYPE_CURSOR_ENTER, 1, 0, 0, 0);
            } else {
                enter_call = Some((st.showing_window as usize as *mut c_void, st.cb_cursor_enter));
            }
        }

        st.cursor_x = x as f64;
        st.cursor_y = y as f64;

        let pos_call = if st.is_use_stack_queue_call {
            None
        } else {
            st.c_last_x = st.cursor_x;
            st.c_last_y = st.cursor_y;
            Some((st.showing_window as usize as *mut c_void, st.cb_cursor_pos))
        };
        (enter_call, pos_call)
    };

    unsafe {
        if let Some((win, cb_enter)) = enter_call {
            let f: GlfwCursorEnterFun = std::mem::transmute(cb_enter);
            f(win, 1);
        }
        if let Some((win, cb_pos)) = pos_call {
            let f: GlfwCursorPosFun = std::mem::transmute(cb_pos);
            f(win, x as f64, y as f64);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeSendMouseButton(
    _env: JNIEnv,
    _class: JClass,
    button: jint,
    action: jint,
    mods: jint,
) {
    let direct_call = {
        let Ok(mut st) = POJAV_ENVIRON.lock() else {
            return;
        };
        if !st.is_input_ready {
            return;
        }
        if !st.mouse_down_buffer.is_null() && (0..8).contains(&button) {
            unsafe {
                *st.mouse_down_buffer.offset(button as isize) = action as i8;
            }
        }
        if st.cb_mouse_button == 0 {
            return;
        }
        if st.is_use_stack_queue_call {
            st.push_event(EVENT_TYPE_MOUSE_BUTTON, button, action, mods, 0);
            None
        } else {
            Some((st.showing_window as usize as *mut c_void, st.cb_mouse_button))
        }
    };
    if let Some((win, cb)) = direct_call {
        unsafe {
            let f: GlfwMouseButtonFun = std::mem::transmute(cb);
            f(win, button as c_int, action as c_int, mods as c_int);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeSendScroll(
    _env: JNIEnv,
    _class: JClass,
    xoffset: jdouble,
    yoffset: jdouble,
) {
    let direct_call = {
        let Ok(mut st) = POJAV_ENVIRON.lock() else {
            return;
        };
        if !st.is_input_ready || st.cb_scroll == 0 {
            return;
        }
        if st.is_use_stack_queue_call {
            st.push_event(
                EVENT_TYPE_SCROLL,
                (xoffset * 100.0) as i32,
                (yoffset * 100.0) as i32,
                0,
                0,
            );
            None
        } else {
            Some((st.showing_window as usize as *mut c_void, st.cb_scroll))
        }
    };
    if let Some((win, cb)) = direct_call {
        unsafe {
            let f: GlfwScrollFun = std::mem::transmute(cb);
            f(win, xoffset, yoffset);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeSendScreenSize(
    _env: JNIEnv,
    _class: JClass,
    width: jint,
    height: jint,
) {
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        st.saved_width = width;
        st.saved_height = height;
        st.monitor_size_consumed = false;
        st.should_update_monitor_size = true;
    }
    notify_event_wait();
}

// --- GLFW Native Callback Registrations (required by LWJGL3 GLFW.java) ---

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_nglfwSetCharCallback(
    env: JNIEnv,
    _class: JClass,
    _window: jlong,
    ptr: jlong,
) -> jlong {
    unsafe { ensure_glfw_bridge_initialized(env.get_raw()); }
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        let prev = st.cb_char as jlong;
        st.cb_char = ptr as usize;
        prev
    } else {
        0
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_nglfwSetCharModsCallback(
    _env: JNIEnv,
    _class: JClass,
    _window: jlong,
    ptr: jlong,
) -> jlong {
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        let prev = st.cb_char_mods as jlong;
        st.cb_char_mods = ptr as usize;
        prev
    } else {
        0
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_nglfwSetCursorEnterCallback(
    _env: JNIEnv,
    _class: JClass,
    _window: jlong,
    ptr: jlong,
) -> jlong {
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        let prev = st.cb_cursor_enter as jlong;
        st.cb_cursor_enter = ptr as usize;
        prev
    } else {
        0
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_nglfwSetCursorPosCallback(
    env: JNIEnv,
    _class: JClass,
    _window: jlong,
    ptr: jlong,
) -> jlong {
    unsafe { ensure_glfw_bridge_initialized(env.get_raw()); }
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        let prev = st.cb_cursor_pos as jlong;
        st.cb_cursor_pos = ptr as usize;
        prev
    } else {
        0
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_nglfwSetKeyCallback(
    env: JNIEnv,
    _class: JClass,
    _window: jlong,
    ptr: jlong,
) -> jlong {
    unsafe { ensure_glfw_bridge_initialized(env.get_raw()); }
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        let prev = st.cb_key as jlong;
        st.cb_key = ptr as usize;
        prev
    } else {
        0
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_nglfwSetMouseButtonCallback(
    _env: JNIEnv,
    _class: JClass,
    _window: jlong,
    ptr: jlong,
) -> jlong {
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        let prev = st.cb_mouse_button as jlong;
        st.cb_mouse_button = ptr as usize;
        prev
    } else {
        0
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_nglfwSetScrollCallback(
    _env: JNIEnv,
    _class: JClass,
    _window: jlong,
    ptr: jlong,
) -> jlong {
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        let prev = st.cb_scroll as jlong;
        st.cb_scroll = ptr as usize;
        prev
    } else {
        0
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_nglfwSetShowingWindow(
    env: JNIEnv,
    _class: JClass,
    window: jlong,
) {
    unsafe { ensure_glfw_bridge_initialized(env.get_raw()); }
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        st.showing_window = window;
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_nglfwGetCursorPos(
    mut env: JNIEnv,
    _class: JClass,
    _window: jlong,
    xpos: JObject,
    ypos: JObject,
) {
    let (cx, cy) = if let Ok(st) = POJAV_ENVIRON.lock() {
        (st.cursor_x, st.cursor_y)
    } else {
        (0.0, 0.0)
    };
    let raw_env = env.get_raw();
    unsafe {
        if let Some(get_addr) = (**raw_env).GetDirectBufferAddress {
            if !xpos.is_null() {
                let ptr = get_addr(raw_env, xpos.as_raw()) as *mut f64;
                if !ptr.is_null() {
                    *ptr = cx;
                } else {
                    let _ = env.call_method(&xpos, "put", "(ID)Ljava/nio/DoubleBuffer;", &[JValue::Int(0), JValue::Double(cx)]);
                }
            }
            if !ypos.is_null() {
                let ptr = get_addr(raw_env, ypos.as_raw()) as *mut f64;
                if !ptr.is_null() {
                    *ptr = cy;
                } else {
                    let _ = env.call_method(&ypos, "put", "(ID)Ljava/nio/DoubleBuffer;", &[JValue::Int(0), JValue::Double(cy)]);
                }
            }
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_nglfwGetCursorPosA(
    env: JNIEnv,
    _class: JClass,
    _window: jlong,
    xpos: JDoubleArray,
    ypos: JDoubleArray,
) {
    if let Ok(st) = POJAV_ENVIRON.lock() {
        if !xpos.is_null() {
            let _ = env.set_double_array_region(&xpos, 0, &[st.cursor_x]);
        }
        if !ypos.is_null() {
            let _ = env.set_double_array_region(&ypos, 0, &[st.cursor_y]);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_glfwSetCursorPos(
    _env: JNIEnv,
    _class: JClass,
    _window: jlong,
    xpos: jdouble,
    ypos: jdouble,
) {
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        st.c_last_x = xpos;
        st.cursor_x = xpos;
        st.c_last_y = ypos;
        st.cursor_y = ypos;
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_internalGetGamepadDataPointer(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    if let Ok(st) = POJAV_ENVIRON.lock() {
        (&st.gamepad_state as *const GLFWGamepadState) as usize as jlong
    } else {
        0
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeCreateGamepadButtonBuffer(
    mut env: JNIEnv,
    _class: JClass,
) -> jobject {
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        let slice = &mut st.gamepad_state.buttons[..];
        if let Ok(buf) = unsafe { env.new_direct_byte_buffer(slice.as_mut_ptr(), slice.len()) } {
            return buf.into_raw();
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeCreateGamepadAxisBuffer(
    mut env: JNIEnv,
    _class: JClass,
) -> jobject {
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        let ptr = st.gamepad_state.axes.as_mut_ptr() as *mut u8;
        let len = st.gamepad_state.axes.len() * std::mem::size_of::<f32>();
        if let Ok(buf) = unsafe { env.new_direct_byte_buffer(ptr, len) } {
            return buf.into_raw();
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeSetWindowAttrib(
    _env: JNIEnv,
    _class: JClass,
    attrib: jint,
    value: jint,
) {
    let (runtime_vm, glfw_cls, mid_attrib, win) = {
        let Ok(st) = POJAV_ENVIRON.lock() else {
            return;
        };
        (
            st.runtime_java_vm,
            st.vm_glfw_class,
            st.method_glfw_set_window_attrib,
            st.showing_window,
        )
    };
    if win == 0 || runtime_vm.is_null() || glfw_cls.is_null() || mid_attrib.is_null() {
        return;
    }
    unsafe {
        if let Ok(jvm) = jni::JavaVM::from_raw(runtime_vm) {
            if let Ok(jvm_env) = jvm.attach_current_thread() {
                let raw_env = jvm_env.get_raw();
                if let Some(call_void) = (**raw_env).CallStaticVoidMethodA {
                    let args = [
                        jvalue { j: win },
                        jvalue { i: attrib },
                        jvalue { i: value },
                    ];
                    call_void(raw_env, glfw_cls, mid_attrib, args.as_ptr());
                    clear_exception(raw_env);
                }
            }
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeEnableGamepadDirectInput(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    let (dalvik_vm, bridge_clazz, mid_direct) = {
        let Ok(st) = POJAV_ENVIRON.lock() else {
            return JNI_FALSE;
        };
        (st.dalvik_java_vm, st.bridge_clazz, st.method_on_direct_input_enable)
    };
    if dalvik_vm.is_null() || bridge_clazz.is_null() || mid_direct.is_null() {
        return JNI_FALSE;
    }
    unsafe {
        if let Ok(jvm) = jni::JavaVM::from_raw(dalvik_vm) {
            if let Ok(dvm_env) = jvm.attach_current_thread() {
                let raw_env = dvm_env.get_raw();
                if let Some(call_void) = (**raw_env).CallStaticVoidMethodA {
                    call_void(raw_env, bridge_clazz, mid_direct, std::ptr::null());
                    clear_exception(raw_env);
                    return JNI_TRUE;
                }
            }
        }
    }
    JNI_FALSE
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeClipboard(
    env: JNIEnv,
    _class: JClass,
    action: jint,
    copy_src: JByteArray,
) -> jstring {
    let copy_text: Option<String> = if !copy_src.is_null() {
        env.convert_byte_array(&copy_src)
            .ok()
            .map(|bytes| {
                let trimmed = bytes.split(|&b| b == 0).next().unwrap_or(&bytes);
                String::from_utf8_lossy(trimmed).into_owned()
            })
    } else {
        None
    };

    let (dalvik_vm, bridge_clazz, mid_clip) = {
        let Ok(st) = POJAV_ENVIRON.lock() else {
            return std::ptr::null_mut();
        };
        (
            st.dalvik_java_vm,
            st.bridge_clazz,
            st.method_access_android_clipboard,
        )
    };
    if dalvik_vm.is_null() || bridge_clazz.is_null() || mid_clip.is_null() {
        return std::ptr::null_mut();
    }

    let pasted: Option<String> = unsafe {
        let Ok(jvm) = jni::JavaVM::from_raw(dalvik_vm) else {
            return std::ptr::null_mut();
        };
        let Ok(dvm_env) = jvm.attach_current_thread() else {
            return std::ptr::null_mut();
        };
        let raw_dvm = dvm_env.get_raw();
        let Some(call_obj) = (**raw_dvm).CallStaticObjectMethodA else {
            return std::ptr::null_mut();
        };

        let mut dvm_copy_jstr: jobject = std::ptr::null_mut();
        if let Some(ref text) = copy_text {
            if let (Some(new_utf), Ok(cs)) = ((**raw_dvm).NewStringUTF, CString::new(text.as_str())) {
                dvm_copy_jstr = new_utf(raw_dvm, cs.as_ptr()) as jobject;
            }
        }

        let args = [jvalue { i: action }, jvalue { l: dvm_copy_jstr }];
        let ret_obj = call_obj(raw_dvm, bridge_clazz, mid_clip, args.as_ptr());
        let had_ex = clear_exception(raw_dvm);

        let mut out_str = None;
        if !had_ex && !ret_obj.is_null() {
            if let (Some(get_chars), Some(rel_chars)) =
                ((**raw_dvm).GetStringUTFChars, (**raw_dvm).ReleaseStringUTFChars)
            {
                let cptr = get_chars(raw_dvm, ret_obj as jstring, std::ptr::null_mut());
                if !cptr.is_null() {
                    out_str = Some(CStr::from_ptr(cptr).to_string_lossy().into_owned());
                    rel_chars(raw_dvm, ret_obj as jstring, cptr);
                }
            }
            if let Some(del_local) = (**raw_dvm).DeleteLocalRef {
                del_local(raw_dvm, ret_obj);
            }
        }
        if !dvm_copy_jstr.is_null() {
            if let Some(del_local) = (**raw_dvm).DeleteLocalRef {
                del_local(raw_dvm, dvm_copy_jstr);
            }
        }
        out_str
    };

    if let Some(text) = pasted {
        if let Ok(jstr) = env.new_string(text) {
            return jstr.into_raw();
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_CriticalNativeTest_testCriticalNative(
    _env: JNIEnv,
    _class: JClass,
    _arg0: jint,
    _arg1: jint,
) {}

#[no_mangle]
pub extern "C" fn pojavStartPumping() {
    let monitor_update = {
        let Ok(mut st) = POJAV_ENVIRON.lock() else {
            return;
        };
        let counter = st.event_counter.load(Ordering::Acquire);
        st.in_event_count = counter;
        st.out_target_index = (st.out_event_index + counter) % EVENT_WINDOW_SIZE;

        if (st.c_last_x != st.cursor_x || st.c_last_y != st.cursor_y) && st.cb_cursor_pos != 0 {
            st.c_last_x = st.cursor_x;
            st.c_last_y = st.cursor_y;
            st.should_update_mouse = true;
        }

        if st.should_update_monitor_size {
            st.monitor_size_consumed = true;
            Some((
                st.runtime_java_vm,
                st.vm_glfw_class,
                st.method_internal_change_monitor_size,
                st.saved_width,
                st.saved_height,
            ))
        } else {
            None
        }
    };

    if let Some((runtime_vm, glfw_cls, mid_mon, w, h)) = monitor_update {
        if !runtime_vm.is_null() && !glfw_cls.is_null() && !mid_mon.is_null() {
            unsafe {
                if let Ok(jvm) = jni::JavaVM::from_raw(runtime_vm) {
                    if let Ok(jvm_env) = jvm.get_env() {
                        let raw_env = jvm_env.get_raw();
                        if let Some(call_void) = (**raw_env).CallStaticVoidMethodA {
                            let args = [jvalue { i: w }, jvalue { i: h }];
                            call_void(raw_env, glfw_cls, mid_mon, args.as_ptr());
                            clear_exception(raw_env);
                        }
                    }
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn pojavStopPumping() {
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        st.out_event_index = st.out_target_index;
        st.event_counter.fetch_sub(st.in_event_count, Ordering::AcqRel);
        st.should_update_mouse = false;
        if st.should_update_monitor_size && st.monitor_size_consumed {
            st.should_update_monitor_size = false;
            st.monitor_size_consumed = false;
        }
    }
}

#[no_mangle]
pub extern "C" fn pojavPumpEvents(window: *mut c_void) {
    let (
        batch,
        update_mouse,
        cx,
        cy,
        win_resize,
        cb_char,
        cb_char_mods,
        cb_cursor_enter,
        cb_cursor_pos,
        cb_key,
        cb_mouse_button,
        cb_scroll,
    ) = {
        let Ok(st) = POJAV_ENVIRON.lock() else {
            return;
        };
        let mut batch = Vec::with_capacity(st.in_event_count);
        let mut idx = st.out_event_index;
        for _ in 0..st.in_event_count {
            batch.push(st.events[idx]);
            idx = (idx + 1) % EVENT_WINDOW_SIZE;
        }
        let win_resize = if st.should_update_monitor_size {
            Some((
                st.runtime_java_vm,
                st.vm_glfw_class,
                st.method_internal_window_size_changed,
            ))
        } else {
            None
        };
        (
            batch,
            st.should_update_mouse,
            st.cursor_x,
            st.cursor_y,
            win_resize,
            st.cb_char,
            st.cb_char_mods,
            st.cb_cursor_enter,
            st.cb_cursor_pos,
            st.cb_key,
            st.cb_mouse_button,
            st.cb_scroll,
        )
    };

    unsafe {
        if update_mouse && cb_cursor_pos != 0 {
            let f: GlfwCursorPosFun = std::mem::transmute(cb_cursor_pos);
            f(window, cx.floor(), cy.floor());
        }

        if let Some((runtime_vm, glfw_cls, mid_win)) = win_resize {
            if !runtime_vm.is_null() && !glfw_cls.is_null() && !mid_win.is_null() {
                if let Ok(jvm) = jni::JavaVM::from_raw(runtime_vm) {
                    if let Ok(jvm_env) = jvm.get_env() {
                        let raw_env = jvm_env.get_raw();
                        if let Some(call_void) = (**raw_env).CallStaticVoidMethodA {
                            let args = [jvalue { j: window as usize as jlong }];
                            call_void(raw_env, glfw_cls, mid_win, args.as_ptr());
                            clear_exception(raw_env);
                        }
                    }
                }
            }
        }

        for ev in batch {
            match ev.event_type {
                EVENT_TYPE_CHAR if cb_char != 0 => {
                    let f: GlfwCharFun = std::mem::transmute(cb_char);
                    f(window, ev.i1 as u32);
                }
                EVENT_TYPE_CHAR_MODS if cb_char_mods != 0 => {
                    let f: GlfwCharModsFun = std::mem::transmute(cb_char_mods);
                    f(window, ev.i1 as u32, ev.i2 as c_int);
                }
                EVENT_TYPE_CURSOR_ENTER if cb_cursor_enter != 0 => {
                    let f: GlfwCursorEnterFun = std::mem::transmute(cb_cursor_enter);
                    f(window, ev.i1 as c_int);
                }
                EVENT_TYPE_KEY if cb_key != 0 => {
                    let f: GlfwKeyFun = std::mem::transmute(cb_key);
                    f(
                        window,
                        ev.i1 as c_int,
                        ev.i2 as c_int,
                        ev.i3 as c_int,
                        ev.i4 as c_int,
                    );
                }
                EVENT_TYPE_MOUSE_BUTTON if cb_mouse_button != 0 => {
                    let f: GlfwMouseButtonFun = std::mem::transmute(cb_mouse_button);
                    f(window, ev.i1 as c_int, ev.i2 as c_int, ev.i3 as c_int);
                }
                EVENT_TYPE_SCROLL if cb_scroll != 0 => {
                    let f: GlfwScrollFun = std::mem::transmute(cb_scroll);
                    f(window, (ev.i1 as f64) / 100.0, (ev.i2 as f64) / 100.0);
                }
                _ => {}
            }
        }
    }
}

// --- LWJGL 3.3.3 / ZalithLauncher 2 Compatibility Hooks ---

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_nativeInitializeGLFWNativeBridge(
    env: JNIEnv,
    _class: JClass,
) {
    unsafe {
        ensure_glfw_bridge_initialized(env.get_raw());
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeSetCursorShape(
    _env: JNIEnv,
    _class: JClass,
    _shape: jint,
) {}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_gwResetEventWait(
    _env: JNIEnv,
    _class: JClass,
) {
    let (lock, _) = &*EVENT_WAIT;
    if let Ok(mut ready) = lock.lock() {
        *ready = false;
    }
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_gwStopEventWait(
    _env: JNIEnv,
    _class: JClass,
) {
    notify_event_wait();
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_gwPostEmptyEvent(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    notify_event_wait();
    0
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_gwWaitEvents(
    _env: JNIEnv,
    _class: JClass,
    seconds: jdouble,
    timed: jboolean,
) -> jint {
    let (lock, cvar) = &*EVENT_WAIT;
    if let Ok(mut ready) = lock.lock() {
        if !*ready {
            let wait_dur = if timed != JNI_FALSE && seconds.is_finite() && seconds > 0.0 {
                Duration::from_secs_f64(seconds.min(0.25))
            } else {
                Duration::from_millis(16)
            };
            let _ = cvar.wait_timeout(ready, wait_dur);
        } else {
            *ready = false;
        }
    }
    0
}
