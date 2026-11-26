use jni::objects::{JClass, JObject, JString};
use jni::sys::{jboolean, jint, jintArray, jlong, JNI_FALSE};
use jni::JNIEnv;

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_AWTInputBridge_nativeSendData(
    _env: JNIEnv, _class: JClass, _t: jint, _i1: jint, _i2: jint, _i3: jint, _i4: jint,
) {}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_AWTInputBridge_nativeClipboardReceived(
    _env: JNIEnv, _class: JClass, _data: JString, _mime: JString,
) {}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_AWTInputBridge_nativeMoveWindow(
    _env: JNIEnv, _class: JClass, _xoff: jint, _yoff: jint,
) {}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_utils_JREUtils_renderAWTScreenFrame(
    _env: JNIEnv, _class: JClass,
) -> jintArray {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "system" fn Java_net_java_openjdk_cacio_ctc_CTCClipboard_nQuerySystemClipboard(
    _env: JNIEnv, _class: JClass,
) {}

#[no_mangle]
pub extern "system" fn Java_net_java_openjdk_cacio_ctc_CTCClipboard_nPutClipboardData(
    _env: JNIEnv, _class: JClass, _data: JString, _mime: JString,
) {}

#[no_mangle]
pub extern "system" fn Java_net_java_openjdk_cacio_ctc_CTCDesktopPeer_openFile(
    _env: JNIEnv, _class: JClass, _path: JString,
) {}

#[no_mangle]
pub extern "system" fn Java_net_java_openjdk_cacio_ctc_CTCDesktopPeer_openUri(
    _env: JNIEnv, _class: JClass, _uri: JString,
) {}

#[no_mangle] pub extern "system" fn Java_java_awt_AWTEvent_initIDs(_e: JNIEnv, _c: JClass) {}
#[no_mangle] pub extern "system" fn Java_java_awt_Component_initIDs(_e: JNIEnv, _c: JClass) {}
#[no_mangle] pub extern "system" fn Java_java_awt_Container_initIDs(_e: JNIEnv, _c: JClass) {}
#[no_mangle] pub extern "system" fn Java_java_awt_Window_initIDs(_e: JNIEnv, _c: JClass) {}
#[no_mangle] pub extern "system" fn Java_java_awt_Frame_initIDs(_e: JNIEnv, _c: JClass) {}
#[no_mangle] pub extern "system" fn Java_java_awt_Dialog_initIDs(_e: JNIEnv, _c: JClass) {}
#[no_mangle] pub extern "system" fn Java_java_awt_Cursor_initIDs(_e: JNIEnv, _c: JClass) {}
#[no_mangle] pub extern "system" fn Java_java_awt_Cursor_finalizeImpl(_e: JNIEnv, _c: JClass, _p: jlong) {}
#[no_mangle] pub extern "system" fn Java_java_awt_AWTEvent_nativeSetSource(_e: JNIEnv, _o: JObject, _s: JObject) {}
#[no_mangle] pub extern "system" fn Java_sun_awt_SunToolkit_closeSplashScreen(_e: JNIEnv, _c: JClass) {}
#[no_mangle] pub extern "system" fn Java_sun_awt_UNIXToolkit_check_1gtk(_e: JNIEnv, _c: JClass, _v: jint) -> jboolean { JNI_FALSE }
#[no_mangle] pub extern "system" fn Java_sun_awt_UNIXToolkit_get_1gtk_1version(_e: JNIEnv, _c: JClass) -> jint { 1 }
#[no_mangle] pub extern "system" fn Java_sun_awt_UNIXToolkit_load_1gtk(_e: JNIEnv, _c: JClass, _v: jint, _b: jboolean) -> jboolean { JNI_FALSE }
#[no_mangle] pub extern "system" fn Java_sun_awt_UNIXToolkit_unload_1gtk(_e: JNIEnv, _c: JClass) -> jboolean { JNI_FALSE }
