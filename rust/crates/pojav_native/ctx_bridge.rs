use crate::environ::POJAV_ENVIRON;
use crate::soc_optimizer::apply_soc_runtime_optimizations;
use jni::objects::{JClass, JObject, JValue};
use jni::sys::jlong;
use jni::JNIEnv;
use once_cell::sync::Lazy;
use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Mutex;

// --- Android NDK ANativeWindow ABI ---
#[repr(C)]
pub struct ANativeWindowBuffer {
    pub width: i32,
    pub height: i32,
    pub stride: i32,
    pub format: i32,
    pub bits: *mut c_void,
    pub reserved: [u32; 6],
}

#[cfg_attr(target_os = "android", link(name = "android"))]
extern "C" {
    fn ANativeWindow_fromSurface(env: *mut jni::sys::JNIEnv, surface: jni::sys::jobject) -> *mut c_void;
    fn ANativeWindow_release(window: *mut c_void);
    fn ANativeWindow_acquire(window: *mut c_void);
    fn ANativeWindow_getWidth(window: *mut c_void) -> i32;
    fn ANativeWindow_getHeight(window: *mut c_void) -> i32;
    fn ANativeWindow_setBuffersGeometry(window: *mut c_void, width: i32, height: i32, format: i32) -> i32;
    fn ANativeWindow_lock(window: *mut c_void, out_buffer: *mut ANativeWindowBuffer, in_out_dirty_bounds: *mut c_void) -> i32;
    fn ANativeWindow_unlockAndPost(window: *mut c_void) -> i32;
}

// --- EGL Constants & Function Table ---
type EGLDisplay = *mut c_void;
type EGLConfig = *mut c_void;
type EGLContext = *mut c_void;
type EGLSurface = *mut c_void;
type EGLBoolean = c_uint;
type EGLint = i32;
type EGLenum = c_uint;

const EGL_FALSE: EGLBoolean = 0;
const EGL_TRUE: EGLBoolean = 1;
const EGL_NO_DISPLAY: EGLDisplay = std::ptr::null_mut();
const EGL_NO_CONTEXT: EGLContext = std::ptr::null_mut();
const EGL_NO_SURFACE: EGLSurface = std::ptr::null_mut();
const EGL_DEFAULT_DISPLAY: *mut c_void = std::ptr::null_mut();

const EGL_ALPHA_SIZE: EGLint = 0x3021;
const EGL_BLUE_SIZE: EGLint = 0x3022;
const EGL_GREEN_SIZE: EGLint = 0x3023;
const EGL_RED_SIZE: EGLint = 0x3024;
const EGL_DEPTH_SIZE: EGLint = 0x3025;
const EGL_NATIVE_VISUAL_ID: EGLint = 0x302E;
const EGL_SURFACE_TYPE: EGLint = 0x3033;
const EGL_NONE: EGLint = 0x3038;
const EGL_RENDERABLE_TYPE: EGLint = 0x3040;
const EGL_HEIGHT: EGLint = 0x3056;
const EGL_WIDTH: EGLint = 0x3057;
const EGL_CONTEXT_CLIENT_VERSION: EGLint = 0x3098;
const EGL_PBUFFER_BIT: EGLint = 0x0001;
const EGL_WINDOW_BIT: EGLint = 0x0004;
const EGL_OPENGL_ES2_BIT: EGLint = 0x0004;
const EGL_OPENGL_ES_API: EGLenum = 0x30A0;
const EGL_OPENGL_API: EGLenum = 0x30A2;
const EGL_BAD_SURFACE: EGLint = 0x300D;

const AHARDWAREBUFFER_FORMAT_R8G8B8X8_UNORM: i32 = 2;
const WINDOW_FORMAT_RGBX_8888: i32 = 2;

const STATE_RENDERER_ALIVE: u8 = 0;
const STATE_RENDERER_NEW_WINDOW: u8 = 1;

const RENDERER_GL4ES: i32 = 1;
const RENDERER_VK_ZINK: i32 = 2;
const RENDERER_VULKAN: i32 = 4;

const GLFW_CLIENT_API: c_int = 0x22001;
const GLFW_NO_API: c_int = 0;
const GLFW_OPENGL_API: c_int = 0x30001;

#[derive(Clone, Copy)]
struct EglApi {
    egl_get_display: unsafe extern "C" fn(*mut c_void) -> EGLDisplay,
    egl_initialize: unsafe extern "C" fn(EGLDisplay, *mut EGLint, *mut EGLint) -> EGLBoolean,
    egl_choose_config: unsafe extern "C" fn(EGLDisplay, *const EGLint, *mut EGLConfig, EGLint, *mut EGLint) -> EGLBoolean,
    egl_get_config_attrib: unsafe extern "C" fn(EGLDisplay, EGLConfig, EGLint, *mut EGLint) -> EGLBoolean,
    egl_bind_api: unsafe extern "C" fn(EGLenum) -> EGLBoolean,
    egl_create_context: unsafe extern "C" fn(EGLDisplay, EGLConfig, EGLContext, *const EGLint) -> EGLContext,
    egl_create_window_surface: unsafe extern "C" fn(EGLDisplay, EGLConfig, *mut c_void, *const EGLint) -> EGLSurface,
    egl_create_pbuffer_surface: unsafe extern "C" fn(EGLDisplay, EGLConfig, *const EGLint) -> EGLSurface,
    egl_make_current: unsafe extern "C" fn(EGLDisplay, EGLSurface, EGLSurface, EGLContext) -> EGLBoolean,
    egl_swap_buffers: unsafe extern "C" fn(EGLDisplay, EGLSurface) -> EGLBoolean,
    egl_swap_interval: unsafe extern "C" fn(EGLDisplay, EGLint) -> EGLBoolean,
    egl_destroy_surface: unsafe extern "C" fn(EGLDisplay, EGLSurface) -> EGLBoolean,
    egl_destroy_context: unsafe extern "C" fn(EGLDisplay, EGLContext) -> EGLBoolean,
    egl_terminate: unsafe extern "C" fn(EGLDisplay) -> EGLBoolean,
    egl_release_thread: unsafe extern "C" fn() -> EGLBoolean,
    egl_get_error: unsafe extern "C" fn() -> EGLint,
    egl_query_surface: unsafe extern "C" fn(EGLDisplay, EGLSurface, EGLint, *mut EGLint) -> EGLBoolean,
}

unsafe impl Send for EglApi {}
unsafe impl Sync for EglApi {}

#[derive(Clone, Copy)]
struct OsmesaApi {
    osmesa_create_context: unsafe extern "C" fn(c_uint, *mut c_void) -> *mut c_void,
    osmesa_make_current: unsafe extern "C" fn(*mut c_void, *mut c_void, c_uint, i32, i32) -> u8,
    osmesa_pixel_store: unsafe extern "C" fn(i32, i32),
    gl_finish: unsafe extern "C" fn(),
}

unsafe impl Send for OsmesaApi {}
unsafe impl Sync for OsmesaApi {}

#[repr(C)]
pub struct GlRenderWindow {
    pub state: u8,
    pub native_surface: *mut c_void,
    pub new_native_surface: *mut c_void,
    pub config: EGLConfig,
    pub format: EGLint,
    pub context: EGLContext,
    pub surface: EGLSurface,
}

unsafe impl Send for GlRenderWindow {}
unsafe impl Sync for GlRenderWindow {}

#[repr(C)]
pub struct OsmRenderWindow {
    pub state: u8,
    pub native_surface: *mut c_void,
    pub new_native_surface: *mut c_void,
    pub buffer: ANativeWindowBuffer,
    pub last_stride: i32,
    pub disable_rendering: bool,
    pub context: *mut c_void,
}

unsafe impl Send for OsmRenderWindow {}
unsafe impl Sync for OsmRenderWindow {}

struct BridgeState {
    egl: Option<EglApi>,
    osmesa: Option<OsmesaApi>,
    egl_display: EGLDisplay,
    current_gl: *mut GlRenderWindow,
    main_gl: *mut GlRenderWindow,
    current_osm: *mut OsmRenderWindow,
    main_osm: *mut OsmRenderWindow,
}

unsafe impl Send for BridgeState {}
unsafe impl Sync for BridgeState {}

static BRIDGE: Lazy<Mutex<BridgeState>> = Lazy::new(|| {
    Mutex::new(BridgeState {
        egl: None,
        osmesa: None,
        egl_display: EGL_NO_DISPLAY,
        current_gl: std::ptr::null_mut(),
        main_gl: std::ptr::null_mut(),
        current_osm: std::ptr::null_mut(),
        main_osm: std::ptr::null_mut(),
    })
});

static mut NO_RENDER_BUFFER: [u8; 4] = [0; 4];
static GL4ES_CURRENT_BUNDLE: AtomicPtr<GlRenderWindow> = AtomicPtr::new(std::ptr::null_mut());

fn getenv_str(key: &str) -> Option<String> {
    let ck = CString::new(key).ok()?;
    unsafe {
        let p = libc::getenv(ck.as_ptr());
        if p.is_null() {
            None
        } else {
            Some(CStr::from_ptr(p).to_string_lossy().into_owned())
        }
    }
}

unsafe fn resolve_egl_proc(
    dl_handle: *mut c_void,
    get_proc: Option<unsafe extern "C" fn(*const c_char) -> *mut c_void>,
    name: &str,
) -> *mut c_void {
    let cname = CString::new(name).unwrap();
    if let Some(gpa) = get_proc {
        let p = gpa(cname.as_ptr());
        if !p.is_null() {
            return p;
        }
    }
    libc::dlsym(dl_handle, cname.as_ptr())
}

unsafe fn dlsym_egl() -> Option<EglApi> {
    let mut handle = std::ptr::null_mut();
    if let Some(custom_egl) = getenv_str("POJAVEXEC_EGL") {
        if let Ok(c_custom) = CString::new(custom_egl) {
            handle = libc::dlopen(c_custom.as_ptr(), libc::RTLD_LOCAL | libc::RTLD_LAZY);
        }
    }
    if handle.is_null() {
        let default_egl = CString::new("libEGL.so").unwrap();
        handle = libc::dlopen(default_egl.as_ptr(), libc::RTLD_LOCAL | libc::RTLD_LAZY);
    }
    if handle.is_null() {
        return None;
    }

    let gpa_name = CString::new("eglGetProcAddress").unwrap();
    let gpa_sym = libc::dlsym(handle, gpa_name.as_ptr());
    let get_proc: Option<unsafe extern "C" fn(*const c_char) -> *mut c_void> = if gpa_sym.is_null() {
        None
    } else {
        Some(std::mem::transmute(gpa_sym))
    };

    macro_rules! load_egl {
        ($sym:expr) => {{
            let p = resolve_egl_proc(handle, get_proc, $sym);
            if p.is_null() {
                return None;
            }
            std::mem::transmute(p)
        }};
    }

    Some(EglApi {
        egl_get_display: load_egl!("eglGetDisplay"),
        egl_initialize: load_egl!("eglInitialize"),
        egl_choose_config: load_egl!("eglChooseConfig"),
        egl_get_config_attrib: load_egl!("eglGetConfigAttrib"),
        egl_bind_api: load_egl!("eglBindAPI"),
        egl_create_context: load_egl!("eglCreateContext"),
        egl_create_window_surface: load_egl!("eglCreateWindowSurface"),
        egl_create_pbuffer_surface: load_egl!("eglCreatePbufferSurface"),
        egl_make_current: load_egl!("eglMakeCurrent"),
        egl_swap_buffers: load_egl!("eglSwapBuffers"),
        egl_swap_interval: load_egl!("eglSwapInterval"),
        egl_destroy_surface: load_egl!("eglDestroySurface"),
        egl_destroy_context: load_egl!("eglDestroyContext"),
        egl_terminate: load_egl!("eglTerminate"),
        egl_release_thread: load_egl!("eglReleaseThread"),
        egl_get_error: load_egl!("eglGetError"),
        egl_query_surface: load_egl!("eglQuerySurface"),
    })
}

unsafe fn dlsym_osmesa() -> Option<OsmesaApi> {
    let mut handle = std::ptr::null_mut();
    for candidate in ["libOSMesa.so.8", "libOSMesa.so"] {
        let cs = CString::new(candidate).unwrap();
        handle = libc::dlopen(cs.as_ptr(), libc::RTLD_LOCAL | libc::RTLD_LAZY);
        if !handle.is_null() {
            break;
        }
    }
    if handle.is_null() {
        return None;
    }

    let gpa_name = CString::new("OSMesaGetProcAddress").unwrap();
    let gpa_sym = libc::dlsym(handle, gpa_name.as_ptr());
    if gpa_sym.is_null() {
        return None;
    }
    let gpa: unsafe extern "C" fn(*const c_char) -> *mut c_void = std::mem::transmute(gpa_sym);

    macro_rules! load_osm {
        ($sym:expr) => {{
            let cname = CString::new($sym).unwrap();
            let p = gpa(cname.as_ptr());
            if p.is_null() {
                return None;
            }
            std::mem::transmute(p)
        }};
    }

    Some(OsmesaApi {
        osmesa_create_context: load_osm!("OSMesaCreateContext"),
        osmesa_make_current: load_osm!("OSMesaMakeCurrent"),
        osmesa_pixel_store: load_osm!("OSMesaPixelStore"),
        gl_finish: load_osm!("glFinish"),
    })
}

unsafe fn gl_swap_surface_internal(egl: &EglApi, display: EGLDisplay, bundle: *mut GlRenderWindow) {
    if !(*bundle).native_surface.is_null() {
        ANativeWindow_release((*bundle).native_surface);
    }
    if !(*bundle).surface.is_null() {
        (egl.egl_destroy_surface)(display, (*bundle).surface);
    }
    if !(*bundle).new_native_surface.is_null() {
        (*bundle).native_surface = (*bundle).new_native_surface;
        (*bundle).new_native_surface = std::ptr::null_mut();
        ANativeWindow_acquire((*bundle).native_surface);
        ANativeWindow_setBuffersGeometry((*bundle).native_surface, 0, 0, (*bundle).format);
        (*bundle).surface = (egl.egl_create_window_surface)(
            display,
            (*bundle).config,
            (*bundle).native_surface,
            std::ptr::null(),
        );
    } else {
        (*bundle).native_surface = std::ptr::null_mut();
        let pbuffer_attrs = [EGL_WIDTH, 1, EGL_HEIGHT, 1, EGL_NONE];
        (*bundle).surface = (egl.egl_create_pbuffer_surface)(display, (*bundle).config, pbuffer_attrs.as_ptr());
    }
}

unsafe extern "C" fn gl4esi_get_display_dimensions(width: *mut c_int, height: *mut c_int) {
    let bundle = GL4ES_CURRENT_BUNDLE.load(Ordering::Acquire);
    if !bundle.is_null() {
        if let Ok(st) = BRIDGE.lock() {
            if let Some(egl) = st.egl {
                let surf = (*bundle).surface;
                let rw = (egl.egl_query_surface)(st.egl_display, surf, EGL_WIDTH, width);
                let rh = (egl.egl_query_surface)(st.egl_display, surf, EGL_HEIGHT, height);
                if rw == EGL_TRUE && rh == EGL_TRUE {
                    return;
                }
            }
        }
    }
    if !width.is_null() {
        *width = 0;
    }
    if !height.is_null() {
        *height = 0;
    }
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_utils_JREUtils_setupBridgeWindow(
    env: JNIEnv,
    _class: JClass,
    surface: JObject,
) {
    unsafe {
        let win = ANativeWindow_fromSurface(env.get_native_interface(), surface.into_raw());
        let renderer = if let Ok(mut pe) = POJAV_ENVIRON.lock() {
            pe.pojav_window = win;
            pe.config_renderer
        } else {
            RENDERER_GL4ES
        };

        if let Ok(st) = BRIDGE.lock() {
            if renderer == RENDERER_VK_ZINK {
                if !st.main_osm.is_null() {
                    (*st.main_osm).state = STATE_RENDERER_NEW_WINDOW;
                    (*st.main_osm).new_native_surface = win;
                }
            } else if !st.main_gl.is_null() {
                (*st.main_gl).state = STATE_RENDERER_NEW_WINDOW;
                (*st.main_gl).new_native_surface = win;
            }
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_utils_JREUtils_releaseBridgeWindow(
    _env: JNIEnv,
    _class: JClass,
) {
    if let Ok(mut pe) = POJAV_ENVIRON.lock() {
        if !pe.pojav_window.is_null() {
            unsafe {
                ANativeWindow_release(pe.pojav_window);
            }
            pe.pojav_window = std::ptr::null_mut();
        }
    }
}

fn load_vulkan_internal() -> *mut c_void {
    let soc = apply_soc_runtime_optimizations();
    let api_level = crate::soc_optimizer::get_android_api_level();

    // Chỉ nạp Adreno Turnip trên thiết bị Snapdragon/Adreno API >= 28 khi POJAV_LOAD_TURNIP được bật.
    // Trên MediaTek Dimensity / Mali / Exynos luôn dùng libvulkan.so hệ thống để đảm bảo ổn định.
    if api_level >= 28 && soc.enable_turnip && getenv_str("POJAV_LOAD_TURNIP").is_some() {
        if let (Some(native_dir), Some(cache_dir)) = (getenv_str("POJAV_NATIVEDIR"), getenv_str("TMPDIR")) {
            let linkerhook_path = format!("{}/liblinkerhook.so", native_dir);
            let turnip_path = format!("{}/libvulkan_freedreno.so", native_dir);
            if let (Ok(c_lh), Ok(c_tp), Ok(_)) = (
                CString::new(linkerhook_path),
                CString::new(turnip_path),
                CString::new(cache_dir),
            ) {
                unsafe {
                    let lh = libc::dlopen(c_lh.as_ptr(), libc::RTLD_LOCAL | libc::RTLD_NOW);
                    if !lh.is_null() {
                        let tp = libc::dlopen(c_tp.as_ptr(), libc::RTLD_LOCAL | libc::RTLD_NOW);
                        if !tp.is_null() {
                            let vk = CString::new("libvulkan.so").unwrap();
                            let vk_ptr = libc::dlopen(vk.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL);
                            if !vk_ptr.is_null() {
                                return vk_ptr;
                            }
                        }
                        libc::dlclose(lh);
                    }
                }
            }
        }
    }

    let vk_name = CString::new("libvulkan.so").unwrap();
    unsafe { libc::dlopen(vk_name.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL) }
}

fn maybe_load_vulkan() -> *mut c_void {
    if let Some(hex_ptr) = getenv_str("VULKAN_PTR") {
        if let Ok(addr) = usize::from_str_radix(hex_ptr.trim_start_matches("0x"), 16) {
            if addr != 0 {
                return addr as *mut c_void;
            }
        }
    }
    let ptr = load_vulkan_internal();
    if !ptr.is_null() {
        if let (Ok(k), Ok(v)) = (CString::new("VULKAN_PTR"), CString::new(format!("{:x}", ptr as usize))) {
            unsafe {
                libc::setenv(k.as_ptr(), v.as_ptr(), 1);
            }
        }
    }
    ptr
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_vulkan_VK_getVulkanDriverHandle(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    maybe_load_vulkan() as jlong
}

#[no_mangle]
pub extern "system" fn Java_org_lwjgl_opengl_PojavRendererInit_nativeInitGl4esInternals(
    mut env: JNIEnv,
    _class: JClass,
    function_provider: JObject,
) {
    let Ok(sym_name) = env.new_string("set_getmainfbsize") else {
        return;
    };
    let sym_obj = JObject::from(sym_name);
    if let Ok(val) = env.call_method(
        &function_provider,
        "getFunctionAddress",
        "(Ljava/lang/CharSequence;)J",
        &[JValue::Object(&sym_obj)],
    ) {
        if let Ok(addr) = val.j() {
            if addr != 0 {
                unsafe {
                    let set_fn: unsafe extern "C" fn(unsafe extern "C" fn(*mut c_int, *mut c_int)) =
                        std::mem::transmute(addr as usize);
                    set_fn(gl4esi_get_display_dimensions);
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn pojavInitOpenGL() -> c_int {
    let force_vsync = getenv_str("FORCE_VSYNC").as_deref() == Some("true");
    let renderer_str = getenv_str("POJAV_RENDERER").unwrap_or_else(|| "opengles2".into());

    let mode = if renderer_str == "vulkan_zink" {
        RENDERER_VK_ZINK
    } else {
        RENDERER_GL4ES
    };

    if let Ok(mut pe) = POJAV_ENVIRON.lock() {
        pe.force_vsync = force_vsync;
        pe.config_renderer = mode;
    }

    unsafe {
        let mut st = match BRIDGE.lock() {
            Ok(g) => g,
            Err(_) => return -1,
        };

        if mode == RENDERER_VK_ZINK {
            maybe_load_vulkan();
            if let (Ok(k), Ok(v)) = (CString::new("GALLIUM_DRIVER"), CString::new("zink")) {
                libc::setenv(k.as_ptr(), v.as_ptr(), 1);
            }
            if st.osmesa.is_none() {
                st.osmesa = dlsym_osmesa();
            }
            if st.osmesa.is_some() && !st.main_osm.is_null() {
                (*st.main_osm).state = STATE_RENDERER_NEW_WINDOW;
                if let Ok(pe) = POJAV_ENVIRON.lock() {
                    (*st.main_osm).new_native_surface = pe.pojav_window;
                }
            }
        } else {
            if st.egl.is_none() {
                st.egl = dlsym_egl();
            }
            if let Some(egl) = st.egl {
                if st.egl_display == EGL_NO_DISPLAY {
                    let disp = (egl.egl_get_display)(EGL_DEFAULT_DISPLAY);
                    if disp != EGL_NO_DISPLAY
                        && (egl.egl_initialize)(disp, std::ptr::null_mut(), std::ptr::null_mut()) == EGL_TRUE
                    {
                        st.egl_display = disp;
                    }
                }
                if !st.main_gl.is_null() {
                    (*st.main_gl).state = STATE_RENDERER_NEW_WINDOW;
                    if let Ok(pe) = POJAV_ENVIRON.lock() {
                        (*st.main_gl).new_native_surface = pe.pojav_window;
                    }
                }
            }
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn pojavInit() -> c_int {
    let win = if let Ok(mut pe) = POJAV_ENVIRON.lock() {
        if pe.pojav_window.is_null() {
            return 0;
        }
        unsafe {
            ANativeWindow_acquire(pe.pojav_window);
            pe.saved_width = ANativeWindow_getWidth(pe.pojav_window);
            pe.saved_height = ANativeWindow_getHeight(pe.pojav_window);
            ANativeWindow_setBuffersGeometry(
                pe.pojav_window,
                pe.saved_width,
                pe.saved_height,
                AHARDWAREBUFFER_FORMAT_R8G8B8X8_UNORM,
            );
        }
        pe.pojav_window
    } else {
        return 0;
    };

    if !win.is_null() {
        pojavInitOpenGL();
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn pojavSetWindowHint(hint: c_int, value: c_int) {
    if hint != GLFW_CLIENT_API {
        return;
    }
    if let Ok(mut pe) = POJAV_ENVIRON.lock() {
        match value {
            GLFW_NO_API => pe.config_renderer = RENDERER_VULKAN,
            GLFW_OPENGL_API => {}
            _ => {}
        }
    }
}

#[no_mangle]
pub extern "C" fn pojavCreateContext(context_src: *mut c_void) -> *mut c_void {
    let mode = POJAV_ENVIRON.lock().map(|p| p.config_renderer).unwrap_or(RENDERER_GL4ES);
    if mode == RENDERER_VULKAN {
        return POJAV_ENVIRON.lock().map(|p| p.pojav_window).unwrap_or(std::ptr::null_mut());
    }

    unsafe {
        let st = match BRIDGE.lock() {
            Ok(g) => g,
            Err(_) => return std::ptr::null_mut(),
        };

        if mode == RENDERER_VK_ZINK {
            let Some(osm) = st.osmesa else {
                return std::ptr::null_mut();
            };
            let share_ctx = if !context_src.is_null() {
                (*(context_src as *mut OsmRenderWindow)).context
            } else {
                std::ptr::null_mut()
            };
            // GL_RGBA = 0x1908
            let ctx = (osm.osmesa_create_context)(0x1908, share_ctx);
            if ctx.is_null() {
                return std::ptr::null_mut();
            }
            let win = Box::new(OsmRenderWindow {
                state: STATE_RENDERER_ALIVE,
                native_surface: std::ptr::null_mut(),
                new_native_surface: std::ptr::null_mut(),
                buffer: std::mem::zeroed(),
                last_stride: 0,
                disable_rendering: false,
                context: ctx,
            });
            return Box::into_raw(win) as *mut c_void;
        }

        let Some(egl) = st.egl else {
            return std::ptr::null_mut();
        };
        if st.egl_display == EGL_NO_DISPLAY {
            return std::ptr::null_mut();
        }

        let egl_attrs = [
            EGL_BLUE_SIZE, 8,
            EGL_GREEN_SIZE, 8,
            EGL_RED_SIZE, 8,
            EGL_ALPHA_SIZE, 8,
            EGL_DEPTH_SIZE, 24,
            EGL_SURFACE_TYPE, EGL_WINDOW_BIT | EGL_PBUFFER_BIT,
            EGL_RENDERABLE_TYPE, EGL_OPENGL_ES2_BIT,
            EGL_NONE,
        ];
        let mut num_configs: EGLint = 0;
        if (egl.egl_choose_config)(st.egl_display, egl_attrs.as_ptr(), std::ptr::null_mut(), 0, &mut num_configs) != EGL_TRUE
            || num_configs == 0
        {
            return std::ptr::null_mut();
        }

        let mut config: EGLConfig = std::ptr::null_mut();
        let mut format: EGLint = 0;
        (egl.egl_choose_config)(st.egl_display, egl_attrs.as_ptr(), &mut config, 1, &mut num_configs);
        (egl.egl_get_config_attrib)(st.egl_display, config, EGL_NATIVE_VISUAL_ID, &mut format);

        let renderer_env = getenv_str("POJAV_RENDERER").unwrap_or_default();
        if renderer_env.starts_with("opengles3_desktopgl") {
            (egl.egl_bind_api)(EGL_OPENGL_API);
        } else {
            (egl.egl_bind_api)(EGL_OPENGL_ES_API);
        }

        let mut libgl_es: EGLint = getenv_str("LIBGL_ES")
            .and_then(|v| v.parse::<EGLint>().ok())
            .unwrap_or(2);
        if libgl_es <= 0 || libgl_es > i16::MAX as EGLint {
            libgl_es = 2;
        }

        let ctx_attrs = [EGL_CONTEXT_CLIENT_VERSION, libgl_es, EGL_NONE];
        let share_ctx = if !context_src.is_null() {
            (*(context_src as *mut GlRenderWindow)).context
        } else {
            EGL_NO_CONTEXT
        };
        let context = (egl.egl_create_context)(st.egl_display, config, share_ctx, ctx_attrs.as_ptr());
        if context == EGL_NO_CONTEXT {
            return std::ptr::null_mut();
        }

        let bundle = Box::new(GlRenderWindow {
            state: STATE_RENDERER_ALIVE,
            native_surface: std::ptr::null_mut(),
            new_native_surface: std::ptr::null_mut(),
            config,
            format,
            context,
            surface: EGL_NO_SURFACE,
        });
        Box::into_raw(bundle) as *mut c_void
    }
}

#[no_mangle]
pub extern "C" fn pojavMakeCurrent(window: *mut c_void) {
    let mode = POJAV_ENVIRON.lock().map(|p| p.config_renderer).unwrap_or(RENDERER_GL4ES);
    let pojav_win = POJAV_ENVIRON.lock().map(|p| p.pojav_window).unwrap_or(std::ptr::null_mut());

    unsafe {
        let mut st = match BRIDGE.lock() {
            Ok(g) => g,
            Err(_) => return,
        };

        if mode == RENDERER_VK_ZINK {
            let Some(osm) = st.osmesa else {
                return;
            };
            if window.is_null() {
                (osm.osmesa_make_current)(std::ptr::null_mut(), std::ptr::null_mut(), 0, 0, 0);
                st.current_osm = std::ptr::null_mut();
                return;
            }
            let bundle = window as *mut OsmRenderWindow;
            st.current_osm = bundle;
            if st.main_osm.is_null() {
                st.main_osm = bundle;
                (*bundle).new_native_surface = pojav_win;
            }
            if (*bundle).native_surface.is_null() {
                if !(*bundle).new_native_surface.is_null() {
                    (*bundle).native_surface = (*bundle).new_native_surface;
                    (*bundle).new_native_surface = std::ptr::null_mut();
                    ANativeWindow_acquire((*bundle).native_surface);
                    ANativeWindow_setBuffersGeometry((*bundle).native_surface, 0, 0, WINDOW_FORMAT_RGBX_8888);
                    (*bundle).disable_rendering = false;
                } else {
                    (*bundle).buffer.bits = std::ptr::addr_of_mut!(NO_RENDER_BUFFER) as *mut c_void;
                    (*bundle).buffer.width = 1;
                    (*bundle).buffer.height = 1;
                    (*bundle).buffer.stride = 0;
                    (*bundle).disable_rendering = true;
                }
                (*bundle).state = STATE_RENDERER_ALIVE;
            }
            // GL_UNSIGNED_BYTE = 0x1401, OSMESA_Y_UP = 0x11
            (osm.osmesa_make_current)(
                (*bundle).context,
                (*bundle).buffer.bits,
                0x1401,
                (*bundle).buffer.width.max(1),
                (*bundle).buffer.height.max(1),
            );
            (osm.osmesa_pixel_store)(0x11, 0);
            return;
        }

        let Some(egl) = st.egl else {
            return;
        };
        if window.is_null() {
            if (egl.egl_make_current)(st.egl_display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT) == EGL_TRUE {
                st.current_gl = std::ptr::null_mut();
                GL4ES_CURRENT_BUNDLE.store(std::ptr::null_mut(), Ordering::Release);
            }
            return;
        }

        let bundle = window as *mut GlRenderWindow;
        let mut has_set_main = false;
        if st.main_gl.is_null() {
            st.main_gl = bundle;
            (*bundle).new_native_surface = pojav_win;
            has_set_main = true;
        }
        if (*bundle).surface.is_null() {
            gl_swap_surface_internal(&egl, st.egl_display, bundle);
        }
        if (egl.egl_make_current)(st.egl_display, (*bundle).surface, (*bundle).surface, (*bundle).context) == EGL_TRUE {
            st.current_gl = bundle;
            GL4ES_CURRENT_BUNDLE.store(bundle, Ordering::Release);
        } else if has_set_main {
            (*bundle).new_native_surface = std::ptr::null_mut();
            gl_swap_surface_internal(&egl, st.egl_display, bundle);
            st.main_gl = std::ptr::null_mut();
        }
    }
}

#[no_mangle]
pub extern "C" fn pojavSwapBuffers() {
    let mode = POJAV_ENVIRON.lock().map(|p| p.config_renderer).unwrap_or(RENDERER_GL4ES);

    unsafe {
        let st = match BRIDGE.lock() {
            Ok(g) => g,
            Err(_) => return,
        };

        if mode == RENDERER_VK_ZINK {
            let (Some(osm), bundle) = (st.osmesa, st.current_osm) else {
                return;
            };
            if bundle.is_null() {
                return;
            }
            if (*bundle).state == STATE_RENDERER_NEW_WINDOW {
                if !(*bundle).native_surface.is_null() {
                    ANativeWindow_release((*bundle).native_surface);
                }
                if !(*bundle).new_native_surface.is_null() {
                    (*bundle).native_surface = (*bundle).new_native_surface;
                    (*bundle).new_native_surface = std::ptr::null_mut();
                    ANativeWindow_acquire((*bundle).native_surface);
                    ANativeWindow_setBuffersGeometry((*bundle).native_surface, 0, 0, WINDOW_FORMAT_RGBX_8888);
                    (*bundle).disable_rendering = false;
                } else {
                    (*bundle).native_surface = std::ptr::null_mut();
                    (*bundle).disable_rendering = true;
                }
                (*bundle).state = STATE_RENDERER_ALIVE;
            }
            if !(*bundle).native_surface.is_null() && !(*bundle).disable_rendering {
                if ANativeWindow_lock((*bundle).native_surface, &mut (*bundle).buffer, std::ptr::null_mut()) != 0 {
                    (*bundle).disable_rendering = true;
                }
            }
            // GL_UNSIGNED_BYTE = 0x1401, OSMESA_ROW_LENGTH = 0x10
            (osm.osmesa_make_current)(
                (*bundle).context,
                (*bundle).buffer.bits,
                0x1401,
                (*bundle).buffer.width.max(1),
                (*bundle).buffer.height.max(1),
            );
            if (*bundle).buffer.stride != (*bundle).last_stride {
                (osm.osmesa_pixel_store)(0x10, (*bundle).buffer.stride);
                (*bundle).last_stride = (*bundle).buffer.stride;
            }
            (osm.gl_finish)();
            if !(*bundle).native_surface.is_null() && !(*bundle).disable_rendering {
                if ANativeWindow_unlockAndPost((*bundle).native_surface) != 0 {
                    (*bundle).disable_rendering = true;
                }
            }
            return;
        }

        let (Some(egl), bundle) = (st.egl, st.current_gl) else {
            return;
        };
        if bundle.is_null() {
            return;
        }
        if (*bundle).state == STATE_RENDERER_NEW_WINDOW {
            (egl.egl_make_current)(st.egl_display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
            gl_swap_surface_internal(&egl, st.egl_display, bundle);
            (egl.egl_make_current)(st.egl_display, (*bundle).surface, (*bundle).surface, (*bundle).context);
            (*bundle).state = STATE_RENDERER_ALIVE;
        }
        if !(*bundle).surface.is_null() {
            if (egl.egl_swap_buffers)(st.egl_display, (*bundle).surface) != EGL_TRUE
                && (egl.egl_get_error)() == EGL_BAD_SURFACE
            {
                (egl.egl_make_current)(st.egl_display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
                (*bundle).new_native_surface = std::ptr::null_mut();
                gl_swap_surface_internal(&egl, st.egl_display, bundle);
                (egl.egl_make_current)(st.egl_display, (*bundle).surface, (*bundle).surface, (*bundle).context);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn pojavGetCurrentContext() -> *mut c_void {
    let mode = POJAV_ENVIRON.lock().map(|p| p.config_renderer).unwrap_or(RENDERER_GL4ES);
    if let Ok(st) = BRIDGE.lock() {
        if mode == RENDERER_VK_ZINK {
            return st.current_osm as *mut c_void;
        }
        return st.current_gl as *mut c_void;
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pojavSwapInterval(interval: c_int) {
    let force_vsync = POJAV_ENVIRON.lock().map(|p| p.force_vsync).unwrap_or(false);
    let actual_interval = if force_vsync { 1 } else { interval };
    if let Ok(st) = BRIDGE.lock() {
        if let Some(egl) = st.egl {
            if st.egl_display != EGL_NO_DISPLAY {
                unsafe {
                    (egl.egl_swap_interval)(st.egl_display, actual_interval);
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn pojavTerminate() {
    if let Ok(mut st) = BRIDGE.lock() {
        if let Some(egl) = st.egl {
            if st.egl_display != EGL_NO_DISPLAY {
                unsafe {
                    (egl.egl_make_current)(st.egl_display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
                    if !st.main_gl.is_null() {
                        if !(*st.main_gl).surface.is_null() {
                            (egl.egl_destroy_surface)(st.egl_display, (*st.main_gl).surface);
                        }
                        if !(*st.main_gl).context.is_null() {
                            (egl.egl_destroy_context)(st.egl_display, (*st.main_gl).context);
                        }
                    }
                    (egl.egl_terminate)(st.egl_display);
                    (egl.egl_release_thread)();
                }
                st.egl_display = EGL_NO_DISPLAY;
            }
        }
    }
}
