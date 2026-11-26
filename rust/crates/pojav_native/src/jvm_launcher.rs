use jni::objects::{JClass, JObjectArray, JString};
use jni::sys::jint;
use jni::JNIEnv;
use std::ffi::{c_char, c_int, CString};

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

unsafe fn launch_jvm_native(args: Vec<CString>) -> i32 {
    let lib_name = CString::new("libjli.so").unwrap();
    let libjli = libc::dlopen(lib_name.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL);
    if libjli.is_null() {
        return -1;
    }

    let mut clean_sa: libc::sigaction = std::mem::zeroed();
    for sigid in libc::SIGHUP..libc::NSIG {
        if sigid == libc::SIGSEGV {
            clean_sa.sa_sigaction = libc::SIG_IGN;
        } else {
            clean_sa.sa_sigaction = libc::SIG_DFL;
        }
        libc::sigaction(sigid, &clean_sa, std::ptr::null_mut());
    }

    let sym_name = CString::new("JLI_Launch").unwrap();
    let sym = libc::dlsym(libjli, sym_name.as_ptr());
    if sym.is_null() {
        return -1;
    }
    let jli_launch: JliLaunchFn = std::mem::transmute(sym);

    let mut argv_ptrs: Vec<*mut c_char> = args.iter().map(|s| s.as_ptr() as *mut c_char).collect();
    let full_ver = CString::new("1.8.0-internal").unwrap();
    let dot_ver = CString::new("1.8").unwrap();
    let prog = argv_ptrs.first().copied().unwrap_or(full_ver.as_ptr() as *mut c_char);

    jli_launch(
        argv_ptrs.len() as c_int,
        argv_ptrs.as_mut_ptr(),
        0,
        std::ptr::null(),
        0,
        std::ptr::null(),
        full_ver.as_ptr(),
        dot_ver.as_ptr(),
        prog,
        prog,
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
            }
        }
    }
    unsafe { launch_jvm_native(c_args) }
}
