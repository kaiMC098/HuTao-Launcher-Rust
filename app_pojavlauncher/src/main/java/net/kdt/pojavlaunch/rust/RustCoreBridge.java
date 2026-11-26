package net.kdt.pojavlaunch.rust;

import androidx.annotation.Keep;

@Keep
public final class RustCoreBridge {
    private static volatile boolean sInitialized = false;

    static {
        try {
            System.loadLibrary("pojavexec");
            System.loadLibrary("pojavexec_awt");
            System.loadLibrary("linkerhook");
            sInitialized = true;
        } catch (Throwable t) {
            t.printStackTrace();
        }
    }

    private RustCoreBridge() {}

    public static void init() {
        if (sInitialized) {
            try {
                nativeApplySocOptimizations();
            } catch (Throwable ignored) {}
        }
    }

    public static boolean isInitialized() {
        return sInitialized;
    }

    public static native String nativeGetUiSnapshotJson();
    public static native String nativeLoginOffline(String username);
    public static native String nativeSearchModrinth(String query, int limit);
    public static native int nativeMapAndroidKeyToGlfw(int androidKeycode);
    public static native String nativeDetectSocProfileJson();
    public static native String nativeApplySocOptimizations();
}
