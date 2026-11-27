use crate::affinity_stdio::{log_native, start_stdio_redirect};
use jni::objects::{JClass, JObjectArray, JString};
use jni::sys::jint;
use jni::JNIEnv;
use std::ffi::{c_char, c_int, CStr, CString};
use std::fs;
use std::path::{Path, PathBuf};

type JliLaunchFn = unsafe extern "C" fn(
    argc: c_int,
    argv: *mut *mut c_char,
    jargc: c_int,
    jargv: *const *const c_char,
    appclassc: c_int,
    appclassv: *const *const c_char,
    fullversion: *const c_char,
    dotversion: *const c_char,
    pname: *const c_char,
    lname: *const c_char,
    javaargs: u8,
    cpwildcard: u8,
    javaw: u8,
    ergo: c_int,
) -> c_int;

fn get_env_string(key: &str) -> Option<String> {
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

fn find_file_recursive(dir: &Path, target_name: &str, max_depth: usize) -> Option<PathBuf> {
    if max_depth == 0 {
        return None;
    }
    let entries = fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                if fname == target_name {
                    return Some(path);
                }
            }
        } else if path.is_dir() {
            subdirs.push(path);
        }
    }
    for sub in subdirs {
        if let Some(found) = find_file_recursive(&sub, target_name, max_depth - 1) {
            return Some(found);
        }
    }
    None
}

unsafe fn last_dlerror() -> String {
    let e = libc::dlerror();
    if e.is_null() {
        "unknown dlerror".to_string()
    } else {
        CStr::from_ptr(e).to_string_lossy().into_owned()
    }
}

unsafe fn resolve_jli_launch() -> Option<JliLaunchFn> {
    let sym_name = CString::new("JLI_Launch").unwrap();

    // 1. Check if JREUtils.initJavaRuntime already loaded libjli.so with RTLD_GLOBAL
    let direct_sym = libc::dlsym(libc::RTLD_DEFAULT, sym_name.as_ptr());
    if !direct_sym.is_null() {
        log_native("[Rust JVM] Resolved JLI_Launch via RTLD_DEFAULT");
        return Some(std::mem::transmute(direct_sym));
    }

    // 2. Gather candidate full paths for libjli.so from LD_LIBRARY_PATH & JAVA_HOME
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(ld_path) = get_env_string("LD_LIBRARY_PATH") {
        for dir in ld_path.split(':') {
            if !dir.is_empty() {
                let p = Path::new(dir).join("libjli.so");
                if p.is_file() && !candidates.contains(&p) {
                    candidates.push(p);
                }
            }
        }
    }
    if let Some(java_home) = get_env_string("JAVA_HOME") {
        let jh = Path::new(&java_home);
        // Ensure libjvm.so is preloaded by full path if needed
        if let Some(jvm_path) = find_file_recursive(jh, "libjvm.so", 5) {
            if let Ok(c_jvm) = CString::new(jvm_path.to_string_lossy().as_ref()) {
                let h_jvm = libc::dlopen(c_jvm.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL);
                if h_jvm.is_null() {
                    log_native(&format!(
                        "[Rust JVM] Preload libjvm.so ({}) failed: {}",
                        jvm_path.display(),
                        last_dlerror()
                    ));
                }
            }
        }
        if let Some(jli_path) = find_file_recursive(jh, "libjli.so", 5) {
            if !candidates.contains(&jli_path) {
                candidates.push(jli_path);
            }
        }
    }

    for cand in &candidates {
        if let Ok(c_path) = CString::new(cand.to_string_lossy().as_ref()) {
            let handle = libc::dlopen(c_path.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL);
            if !handle.is_null() {
                let sym = libc::dlsym(handle, sym_name.as_ptr());
                if !sym.is_null() {
                    log_native(&format!("[Rust JVM] Loaded JLI_Launch from {}", cand.display()));
                    return Some(std::mem::transmute(sym));
                }
                log_native(&format!(
                    "[Rust JVM] Opened {} but dlsym(JLI_Launch) failed: {}",
                    cand.display(),
                    last_dlerror()
                ));
            } else {
                log_native(&format!(
                    "[Rust JVM] dlopen({}) failed: {}",
                    cand.display(),
                    last_dlerror()
                ));
            }
        }
    }

    // 3. Final fallback: bare name dlopen("libjli.so")
    let bare = CString::new("libjli.so").unwrap();
    let handle = libc::dlopen(bare.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL);
    if !handle.is_null() {
        let sym = libc::dlsym(handle, sym_name.as_ptr());
        if !sym.is_null() {
            log_native("[Rust JVM] Loaded JLI_Launch via bare libjli.so");
            return Some(std::mem::transmute(sym));
        }
    }

    log_native(&format!(
        "[Rust JVM] FATAL: Could not load libjli.so (candidates tried: {}, last error: {})",
        candidates.len(),
        last_dlerror()
    ));
    None
}

unsafe fn launch_jvm_native(args: Vec<CString>) -> i32 {
    start_stdio_redirect();

    let Some(jli_launch) = resolve_jli_launch() else {
        return -1;
    };

    // Preserve Android ART libsigchain handlers (SIGSEGV/SIGBUS/SIGABRT/SIGKILL/SIGSTOP)
    let mut clean_sa: libc::sigaction = std::mem::zeroed();
    clean_sa.sa_sigaction = libc::SIG_DFL;
    for sigid in libc::SIGHUP..32 {
        if sigid != libc::SIGKILL
            && sigid != libc::SIGSTOP
            && sigid != libc::SIGSEGV
            && sigid != libc::SIGBUS
            && sigid != libc::SIGABRT
        {
            libc::sigaction(sigid, &clean_sa, std::ptr::null_mut());
        }
    }

    let mut argv_ptrs: Vec<*mut c_char> = args.iter().map(|s| s.as_ptr() as *mut c_char).collect();
    let argc = argv_ptrs.len() as c_int;
    let full_ver = CString::new("1.8.0-internal").unwrap();
    let dot_ver = CString::new("1.8").unwrap();
    let prog_name = CString::new("java").unwrap();
    let launcher_name = CString::new("openjdk").unwrap();
    argv_ptrs.push(std::ptr::null_mut());

    log_native(&format!("[Rust JVM] Invoking JLI_Launch with {} arguments...", argc));

    jli_launch(
        argc,
        argv_ptrs.as_mut_ptr(),
        0,
        std::ptr::null(),
        0,
        std::ptr::null(),
        full_ver.as_ptr(),
        dot_ver.as_ptr(),
        prog_name.as_ptr(),
        launcher_name.as_ptr(),
        0,
        1,
        0,
        0,
    )
}

#[no_mangle]
pub extern "system" fn Java_com_oracle_dalvik_VMLauncher_launchJVM(
    mut env: JNIEnv,
    _class: JClass,
    args_array: JObjectArray,
) -> jint {
    if args_array.is_null() {
        return 0;
    }
    let len = env.get_array_length(&args_array).unwrap_or(0);
    let mut c_args = Vec::with_capacity(len as usize);
    for i in 0..len {
        if let Ok(obj) = env.get_object_array_element(&args_array, i) {
            let jstr = JString::from(obj);
            if let Ok(rust_str) = env.get_string(&jstr) {
                if let Ok(cs) = CString::new(rust_str.to_str().unwrap_or_default()) {
                    c_args.push(cs);
                }
            };
        }
    }
    unsafe { launch_jvm_native(c_args) }
}
