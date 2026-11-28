package net.kdt.pojavlaunch.prefs;

import android.content.SharedPreferences;
import android.util.Log;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;

/**
 * MobileGlues renderer configuration compatible with ZalithLauncher 2's MobileGluesRenderer.
 */
public final class MobileGluesConfig {
    private static final String TAG = "MobileGluesConfig";

    public static final String KEY_SFPEW_LAYER = "mg_sfpew_layer";
    public static final String KEY_ENABLE_ANGLE = "mg_enable_angle";
    public static final String KEY_ENABLE_NOERROR = "mg_enable_noerror";
    public static final String KEY_CUSTOM_GL_VERSION = "mg_custom_gl_version";
    public static final String KEY_MAX_GLSL_CACHE = "mg_max_glsl_cache";
    public static final String KEY_EXT_TIMER_QUERY = "mg_ext_timer_query";
    public static final String KEY_EXT_COMPUTE_SHADER = "mg_ext_compute_shader";
    public static final String KEY_EXT_DSA = "mg_ext_dsa";
    public static final String KEY_DEPTH_CLEAR_FIX = "mg_depth_clear_fix";
    public static final String KEY_FSR1_SETTING = "mg_fsr1_setting";
    public static final String KEY_MULTIDRAW_PRESET = "mg_multidraw_preset";

    public static final String[] MULTIDRAW_ORDER_DEFAULT = new String[]{
            "native", "multiindirect", "multibasevertex", "multiarrays",
            "indirect", "basevertex", "unroll", "compute"
    };

    private MobileGluesConfig() {}

    private static SharedPreferences prefs() {
        return LauncherPreferences.DEFAULT_PREF;
    }

    public static boolean isSfpewEnabled() {
        return prefs() == null || prefs().getBoolean(KEY_SFPEW_LAYER, true);
    }

    public static int getEnableAngle() {
        return prefs() == null ? 0 : prefs().getInt(KEY_ENABLE_ANGLE, 0);
    }

    public static int getEnableNoError() {
        return prefs() == null ? 0 : prefs().getInt(KEY_ENABLE_NOERROR, 0);
    }

    public static int getCustomGlVersion() {
        return prefs() == null ? 0 : prefs().getInt(KEY_CUSTOM_GL_VERSION, 0);
    }

    public static int getMaxGlslCacheSize() {
        return prefs() == null ? 32 : prefs().getInt(KEY_MAX_GLSL_CACHE, 32);
    }

    public static boolean isExtTimerQueryEnabled() {
        return prefs() == null || prefs().getBoolean(KEY_EXT_TIMER_QUERY, true);
    }

    public static boolean isExtComputeShaderEnabled() {
        return prefs() != null && prefs().getBoolean(KEY_EXT_COMPUTE_SHADER, false);
    }

    public static boolean isExtDsaEnabled() {
        return prefs() != null && prefs().getBoolean(KEY_EXT_DSA, false);
    }

    public static boolean isDepthClearFixEnabled() {
        return prefs() != null && prefs().getBoolean(KEY_DEPTH_CLEAR_FIX, false);
    }

    public static int getFsr1Setting() {
        return prefs() == null ? 0 : prefs().getInt(KEY_FSR1_SETTING, 0);
    }

    public static int getMultidrawPreset() {
        return prefs() == null ? 0 : prefs().getInt(KEY_MULTIDRAW_PRESET, 0);
    }

    public static void resetDefaults() {
        SharedPreferences p = prefs();
        if (p == null) return;
        p.edit()
                .putBoolean(KEY_SFPEW_LAYER, true)
                .putInt(KEY_ENABLE_ANGLE, 0)
                .putInt(KEY_ENABLE_NOERROR, 0)
                .putInt(KEY_CUSTOM_GL_VERSION, 0)
                .putInt(KEY_MAX_GLSL_CACHE, 32)
                .putBoolean(KEY_EXT_TIMER_QUERY, true)
                .putBoolean(KEY_EXT_COMPUTE_SHADER, false)
                .putBoolean(KEY_EXT_DSA, false)
                .putBoolean(KEY_DEPTH_CLEAR_FIX, false)
                .putInt(KEY_FSR1_SETTING, 0)
                .putInt(KEY_MULTIDRAW_PRESET, 0)
                .apply();
    }

    public static void writeConfigJson(File mgDir) {
        if (!mgDir.exists() && !mgDir.mkdirs()) {
            Log.w(TAG, "Failed to create MobileGlues dir: " + mgDir);
            return;
        }
        File cfgFile = new File(mgDir, "config.json");
        File tmpFile = new File(mgDir, "config.json.tmp");

        try {
            JSONObject json = new JSONObject();
            json.put("enableANGLE", getEnableAngle());
            json.put("enableNoError", getEnableNoError());
            json.put("customGLVersion", getCustomGlVersion());
            json.put("maxGlslCacheSize", getMaxGlslCacheSize());
            json.put("enableExtTimerQuery", isExtTimerQueryEnabled());
            json.put("enableExtComputeShader", isExtComputeShaderEnabled());
            json.put("enableExtDirectStateAccess", isExtDsaEnabled());
            json.put("angleDepthClearFixMode", isDepthClearFixEnabled());
            json.put("fsr1Setting", getFsr1Setting());

            int preset = getMultidrawPreset();
            JSONArray orderArr = new JSONArray();
            if (preset > 0 && preset < MULTIDRAW_ORDER_DEFAULT.length) {
                orderArr.put(MULTIDRAW_ORDER_DEFAULT[preset]);
                for (int i = 0; i < MULTIDRAW_ORDER_DEFAULT.length; i++) {
                    if (i != preset) orderArr.put(MULTIDRAW_ORDER_DEFAULT[i]);
                }
            } else {
                for (String item : MULTIDRAW_ORDER_DEFAULT) orderArr.put(item);
            }
            json.put("multidrawOrder", orderArr);

            byte[] bytes = json.toString(2).getBytes(StandardCharsets.UTF_8);
            try (FileOutputStream fos = new FileOutputStream(tmpFile)) {
                fos.write(bytes);
                fos.getFD().sync();
            }
            if (cfgFile.exists()) cfgFile.delete();
            if (!tmpFile.renameTo(cfgFile)) {
                try (FileOutputStream fos = new FileOutputStream(cfgFile)) {
                    fos.write(bytes);
                }
                tmpFile.delete();
            }
        } catch (Exception e) {
            Log.e(TAG, "Failed to write MobileGlues config.json", e);
            if (tmpFile.exists()) tmpFile.delete();
        }
    }
}
