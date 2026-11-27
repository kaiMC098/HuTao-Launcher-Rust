use crate::environ::*;
use jni::objects::{JByteArray, JClass, JDoubleArray, JObject, JString, JValue};
use jni::sys::{jboolean, jchar, jdouble, jfloat, jint, jlong, jobject, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use std::ffi::{c_int, c_void};
use std::sync::atomic::Ordering;

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
    _env: JNIEnv,
    _class: JClass,
    input_ready: jboolean,
) -> jboolean {
    if let Ok(mut env_state) = POJAV_ENVIRON.lock() {
        env_state.is_input_ready = input_ready != JNI_FALSE;
        if env_state.is_use_stack_queue_call { JNI_TRUE } else { JNI_FALSE }
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
    if let Ok(mut env_state) = POJAV_ENVIRON.lock() {
        env_state.is_grabbing = grabbing;
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
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        if st.is_input_ready {
            st.push_event(EVENT_TYPE_CHAR, codepoint as i32, 0, 0, 0);
            return JNI_TRUE;
        }
    }
    JNI_FALSE
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeSendCharMods(
    _env: JNIEnv,
    _class: JClass,
    codepoint: jchar,
    mods: jint,
) -> jboolean {
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        if st.is_input_ready {
            st.push_event(EVENT_TYPE_CHAR_MODS, codepoint as i32, mods, 0, 0);
            return JNI_TRUE;
        }
    }
    JNI_FALSE
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
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        if st.is_input_ready {
            if !st.key_down_buffer.is_null() {
                let idx = (key - 31).max(0) as isize;
                unsafe { *st.key_down_buffer.offset(idx) = action as i8; }
            }
            st.push_event(EVENT_TYPE_KEY, key, scancode, action, mods);
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
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        if st.is_input_ready {
            st.cursor_x = x as f64;
            st.cursor_y = y as f64;
            st.should_update_mouse = true;
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
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        if st.is_input_ready {
            if !st.mouse_down_buffer.is_null() {
                let idx = button.max(0) as isize;
                unsafe { *st.mouse_down_buffer.offset(idx) = action as i8; }
            }
            st.push_event(EVENT_TYPE_MOUSE_BUTTON, button, action, mods, 0);
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
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        if st.is_input_ready {
            st.push_event(EVENT_TYPE_SCROLL, (xoffset * 100.0) as i32, (yoffset * 100.0) as i32, 0, 0);
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
}

// --- GLFW Native Callback Registrations (required by LWJGL3 GLFW.java) ---

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_nglfwSetCharCallback(
    _env: JNIEnv,
    _class: JClass,
    _window: jlong,
    ptr: jlong,
) -> jlong {
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
    _env: JNIEnv,
    _class: JClass,
    _window: jlong,
    ptr: jlong,
) -> jlong {
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
    _env: JNIEnv,
    _class: JClass,
    _window: jlong,
    ptr: jlong,
) -> jlong {
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
    _env: JNIEnv,
    _class: JClass,
    window: jlong,
) {
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
    if !xpos.is_null() {
        let _ = env.call_method(&xpos, "put", "(ID)Ljava/nio/DoubleBuffer;", &[JValue::Int(0), JValue::Double(cx)]);
    }
    if !ypos.is_null() {
        let _ = env.call_method(&ypos, "put", "(ID)Ljava/nio/DoubleBuffer;", &[JValue::Int(0), JValue::Double(cy)]);
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
    _attrib: jint,
    _value: jint,
) {}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeEnableGamepadDirectInput(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    JNI_TRUE
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_glfw_CallbackBridge_nativeClipboard(
    _env: JNIEnv,
    _class: JClass,
    _action: jint,
    _copy_src: JByteArray,
) -> jstring {
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
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        let counter = st.event_counter.load(Ordering::Acquire);
        st.in_event_count = counter;
        st.out_target_index = (st.out_event_index + counter) % EVENT_WINDOW_SIZE;
    }
}

#[no_mangle]
pub extern "C" fn pojavStopPumping() {
    if let Ok(mut st) = POJAV_ENVIRON.lock() {
        st.out_event_index = st.out_target_index;
        st.event_counter.fetch_sub(st.in_event_count, Ordering::AcqRel);
        st.should_update_mouse = false;
    }
}

#[no_mangle]
pub extern "C" fn pojavPumpEvents(window: *mut c_void) {
    // Snapshot queued events & callback addresses under lock, then drop lock before invoking callbacks
    let (batch, update_mouse, cx, cy, cb_char, cb_char_mods, cb_cursor_enter, cb_cursor_pos, cb_key, cb_mouse_button, cb_scroll) = {
        let Ok(st) = POJAV_ENVIRON.lock() else {
            return;
        };
        let mut batch = Vec::with_capacity(st.in_event_count);
        let mut idx = st.out_event_index;
        for _ in 0..st.in_event_count {
            batch.push(st.events[idx]);
            idx = (idx + 1) % EVENT_WINDOW_SIZE;
        }
        (
            batch,
            st.should_update_mouse,
            st.cursor_x,
            st.cursor_y,
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
            f(window, cx, cy);
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
                    f(window, ev.i1 as c_int, ev.i2 as c_int, ev.i3 as c_int, ev.i4 as c_int);
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
