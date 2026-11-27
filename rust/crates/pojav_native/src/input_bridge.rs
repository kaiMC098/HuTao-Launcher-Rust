use crate::environ::*;
use jni::objects::{JByteArray, JClass, JDoubleArray};
use jni::sys::{jboolean, jchar, jdouble, jfloat, jint, jlong, jobject, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
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
            st.push_event(EVENT_TYPE_SCROLL, xoffset as i32, yoffset as i32, 0, 0);
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
pub extern "system" fn Java_org_lwjgl_glfw_GLFW_nglfwGetCursorPosA(
    env: JNIEnv,
    _class: JClass,
    _window: jlong,
    xpos: JDoubleArray,
    ypos: JDoubleArray,
) {
    if let Ok(st) = POJAV_ENVIRON.lock() {
        let _ = env.set_double_array_region(&xpos, 0, &[st.cursor_x]);
        let _ = env.set_double_array_region(&ypos, 0, &[st.cursor_y]);
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
pub extern "C" fn pojavPumpEvents(_window: *mut std::ffi::c_void) {}
