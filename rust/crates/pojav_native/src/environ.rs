use jni::sys::{jboolean, jbyte, jclass, jlong, jmethodID, JNIEnv, JavaVM};
use once_cell::sync::Lazy;
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub const EVENT_WINDOW_SIZE: usize = 8000;

pub const EVENT_TYPE_CHAR: i32 = 1000;
pub const EVENT_TYPE_CHAR_MODS: i32 = 1001;
pub const EVENT_TYPE_CURSOR_ENTER: i32 = 1002;
pub const EVENT_TYPE_KEY: i32 = 1005;
pub const EVENT_TYPE_MOUSE_BUTTON: i32 = 1006;
pub const EVENT_TYPE_SCROLL: i32 = 1007;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GLFWInputEvent {
    pub event_type: i32,
    pub i1: i32,
    pub i2: i32,
    pub i3: i32,
    pub i4: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GLFWGamepadState {
    pub buttons: [u8; 15],
    pub axes: [f32; 6],
}

impl Default for GLFWGamepadState {
    fn default() -> Self {
        Self {
            buttons: [0; 15],
            axes: [0.0; 6],
        }
    }
}

pub struct PojavEnviron {
    pub pojav_window: *mut c_void,
    pub config_renderer: i32,
    pub force_vsync: bool,
    pub event_counter: AtomicUsize,
    pub events: [GLFWInputEvent; EVENT_WINDOW_SIZE],
    pub out_event_index: usize,
    pub out_target_index: usize,
    pub in_event_index: usize,
    pub in_event_count: usize,
    pub cursor_x: f64,
    pub cursor_y: f64,
    pub c_last_x: f64,
    pub c_last_y: f64,
    pub is_grabbing: jboolean,
    pub key_down_buffer: *mut jbyte,
    pub mouse_down_buffer: *mut jbyte,
    pub runtime_java_vm: *mut JavaVM,
    pub glfw_thread_vm_env: *mut JNIEnv,
    pub dalvik_java_vm: *mut JavaVM,
    pub bridge_clazz: jclass,
    pub vm_glfw_class: jclass,
    pub method_access_android_clipboard: jmethodID,
    pub method_on_grab_state_changed: jmethodID,
    pub method_on_direct_input_enable: jmethodID,
    pub method_glfw_set_window_attrib: jmethodID,
    pub method_internal_window_size_changed: jmethodID,
    pub method_internal_change_monitor_size: jmethodID,
    pub showing_window: jlong,
    pub is_input_ready: bool,
    pub is_cursor_entered: bool,
    pub is_use_stack_queue_call: bool,
    pub should_update_mouse: bool,
    pub should_update_monitor_size: bool,
    pub monitor_size_consumed: bool,
    pub saved_width: i32,
    pub saved_height: i32,
    pub gamepad_state: GLFWGamepadState,
}

unsafe impl Send for PojavEnviron {}
unsafe impl Sync for PojavEnviron {}

impl PojavEnviron {
    pub const fn new() -> Self {
        Self {
            pojav_window: std::ptr::null_mut(),
            config_renderer: 1,
            force_vsync: false,
            event_counter: AtomicUsize::new(0),
            events: [GLFWInputEvent {
                event_type: 0,
                i1: 0,
                i2: 0,
                i3: 0,
                i4: 0,
            }; EVENT_WINDOW_SIZE],
            out_event_index: 0,
            out_target_index: 0,
            in_event_index: 0,
            in_event_count: 0,
            cursor_x: 0.0,
            cursor_y: 0.0,
            c_last_x: 0.0,
            c_last_y: 0.0,
            is_grabbing: 0,
            key_down_buffer: std::ptr::null_mut(),
            mouse_down_buffer: std::ptr::null_mut(),
            runtime_java_vm: std::ptr::null_mut(),
            glfw_thread_vm_env: std::ptr::null_mut(),
            dalvik_java_vm: std::ptr::null_mut(),
            bridge_clazz: std::ptr::null_mut(),
            vm_glfw_class: std::ptr::null_mut(),
            method_access_android_clipboard: std::ptr::null_mut(),
            method_on_grab_state_changed: std::ptr::null_mut(),
            method_on_direct_input_enable: std::ptr::null_mut(),
            method_glfw_set_window_attrib: std::ptr::null_mut(),
            method_internal_window_size_changed: std::ptr::null_mut(),
            method_internal_change_monitor_size: std::ptr::null_mut(),
            showing_window: 0,
            is_input_ready: false,
            is_cursor_entered: false,
            is_use_stack_queue_call: false,
            should_update_mouse: false,
            should_update_monitor_size: false,
            monitor_size_consumed: false,
            saved_width: 1280,
            saved_height: 720,
            gamepad_state: GLFWGamepadState {
                buttons: [0; 15],
                axes: [0.0; 6],
            },
        }
    }

    pub fn push_event(&mut self, event_type: i32, i1: i32, i2: i32, i3: i32, i4: i32) {
        self.events[self.in_event_index] = GLFWInputEvent {
            event_type,
            i1,
            i2,
            i3,
            i4,
        };
        self.in_event_index = (self.in_event_index + 1) % EVENT_WINDOW_SIZE;
        self.event_counter.fetch_add(1, Ordering::Release);
    }
}

pub static POJAV_ENVIRON: Lazy<Mutex<PojavEnviron>> = Lazy::new(|| Mutex::new(PojavEnviron::new()));
