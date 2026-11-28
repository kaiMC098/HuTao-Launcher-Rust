package net.kdt.pojavlaunch.prefs;

import android.app.AlertDialog;
import android.content.Context;
import android.content.SharedPreferences;
import android.graphics.Color;
import android.text.InputType;
import android.view.ViewGroup;
import android.widget.ArrayAdapter;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Spinner;
import android.widget.Switch;
import android.widget.TextView;
import android.widget.Toast;

import net.kdt.pojavlaunch.Tools;

import java.io.File;

/**
 * Interactive MobileGlues configuration dialog mirroring ZalithLauncher 2's options.
 */
public final class MobileGluesSettingsDialog {
    private static final String[] ANGLE_OPTIONS = {
            "Tắt khi có thể (Mặc định)",
            "Bật khi có thể",
            "Buộc tắt (Force Disable)",
            "Buộc bật (Force Enable)"
    };

    private static final String[] NOERROR_OPTIONS = {
            "Tự động (Mặc định)",
            "Tắt (Off)",
            "Bỏ qua lỗi Shader",
            "Bỏ qua lỗi Framebuffer"
    };

    private static final String[] GL_VERSION_LABELS = {
            "Tự động (Mặc định)",
            "OpenGL 4.6", "OpenGL 4.5", "OpenGL 4.4", "OpenGL 4.3",
            "OpenGL 4.2", "OpenGL 4.1", "OpenGL 4.0", "OpenGL 3.3", "OpenGL 3.2"
    };
    private static final int[] GL_VERSION_VALUES = {0, 46, 45, 44, 43, 42, 41, 40, 33, 32};

    private static final String[] FSR1_OPTIONS = {
            "Tắt (Mặc định)",
            "Ultra Quality (Siêu nét)",
            "Quality (Chất lượng)",
            "Balanced (Cân bằng)",
            "Performance (Hiệu năng)"
    };

    private static final String[] MULTIDRAW_OPTIONS = {
            "Tự động (Native ưu tiên)",
            "Ưu tiên MultiIndirect",
            "Ưu tiên MultiBaseVertex",
            "Ưu tiên MultiArrays",
            "Ưu tiên Indirect",
            "Ưu tiên BaseVertex",
            "Ưu tiên Unroll (Tương thích cao)",
            "Ưu tiên Compute"
    };

    private MobileGluesSettingsDialog() {}

    public static void show(Context context) {
        if (context == null) return;
        float density = context.getResources().getDisplayMetrics().density;
        int pad = (int) (14 * density);
        int gap = (int) (10 * density);

        ScrollView scroll = new ScrollView(context);
        LinearLayout root = new LinearLayout(context);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(pad, pad, pad, pad);
        root.setBackgroundColor(Color.parseColor("#240A0D"));
        scroll.addView(root);

        Switch swSfpew = addSwitch(root, context, "Lớp tương thích SFPEW (Chạy 1.12.2 – 1.16.5)",
                "Bật libSimpleFPEWrapper giúp MobileGlues chạy ổn định trên phiên bản cũ.",
                MobileGluesConfig.isSfpewEnabled(), gap);

        Spinner spAngle = addSpinner(root, context, "Chế độ ANGLE (enableANGLE)", ANGLE_OPTIONS,
                MobileGluesConfig.getEnableAngle(), gap);

        Spinner spNoError = addSpinner(root, context, "Bỏ qua lỗi OpenGL (enableNoError)", NOERROR_OPTIONS,
                MobileGluesConfig.getEnableNoError(), gap);

        int glIdx = 0;
        int curGl = MobileGluesConfig.getCustomGlVersion();
        for (int i = 0; i < GL_VERSION_VALUES.length; i++) {
            if (GL_VERSION_VALUES[i] == curGl) {
                glIdx = i;
                break;
            }
        }
        Spinner spGlVer = addSpinner(root, context, "Phiên bản OpenGL (customGLVersion)", GL_VERSION_LABELS,
                glIdx, gap);

        Spinner spFsr1 = addSpinner(root, context, "Siêu phân giải AMD FSR 1 (fsr1Setting)", FSR1_OPTIONS,
                MobileGluesConfig.getFsr1Setting(), gap);

        Spinner spMultidraw = addSpinner(root, context, "Chế độ MultiDraw (multidrawOrder)", MULTIDRAW_OPTIONS,
                MobileGluesConfig.getMultidrawPreset(), gap);

        EditText etCache = addNumberField(root, context, "Bộ đệm Shader GLSL (MB, -1 = không giới hạn)",
                MobileGluesConfig.getMaxGlslCacheSize(), gap);

        Switch swTimer = addSwitch(root, context, "Tiện ích Timer Query (enableExtTimerQuery)",
                "Hỗ trợ đo thời gian khung hình cho các mod tối ưu.",
                MobileGluesConfig.isExtTimerQueryEnabled(), gap);

        Switch swCompute = addSwitch(root, context, "Compute Shader (enableExtComputeShader)",
                "Tắt mặc định để tránh crash trên GPU MediaTek Mali.",
                MobileGluesConfig.isExtComputeShaderEnabled(), gap);

        Switch swDsa = addSwitch(root, context, "Direct State Access (enableExtDirectStateAccess)",
                "Chỉ bật khi shader/mod yêu cầu rõ DSA.",
                MobileGluesConfig.isExtDsaEnabled(), gap);

        Switch swDepth = addSwitch(root, context, "Sửa lỗi xóa chiều sâu ANGLE (angleDepthClearFixMode)",
                "Bật nếu gặp lỗi nhấp nháy mặt nước/khối khi dùng ANGLE.",
                MobileGluesConfig.isDepthClearFixEnabled(), gap);

        new AlertDialog.Builder(context)
                .setTitle("Tùy chỉnh MobileGlues (Zalith2)")
                .setView(scroll)
                .setPositiveButton("Lưu cấu hình", (dialog, which) -> {
                    int cacheMb = 32;
                    try {
                        String raw = etCache.getText().toString().trim();
                        if (raw.isEmpty()) cacheMb = -1;
                        else cacheMb = Integer.parseInt(raw);
                    } catch (NumberFormatException ignored) {}

                    SharedPreferences p = LauncherPreferences.DEFAULT_PREF;
                    if (p != null) {
                        p.edit()
                                .putBoolean(MobileGluesConfig.KEY_SFPEW_LAYER, swSfpew.isChecked())
                                .putInt(MobileGluesConfig.KEY_ENABLE_ANGLE, spAngle.getSelectedItemPosition())
                                .putInt(MobileGluesConfig.KEY_ENABLE_NOERROR, spNoError.getSelectedItemPosition())
                                .putInt(MobileGluesConfig.KEY_CUSTOM_GL_VERSION, GL_VERSION_VALUES[spGlVer.getSelectedItemPosition()])
                                .putInt(MobileGluesConfig.KEY_FSR1_SETTING, spFsr1.getSelectedItemPosition())
                                .putInt(MobileGluesConfig.KEY_MULTIDRAW_PRESET, spMultidraw.getSelectedItemPosition())
                                .putInt(MobileGluesConfig.KEY_MAX_GLSL_CACHE, cacheMb)
                                .putBoolean(MobileGluesConfig.KEY_EXT_TIMER_QUERY, swTimer.isChecked())
                                .putBoolean(MobileGluesConfig.KEY_EXT_COMPUTE_SHADER, swCompute.isChecked())
                                .putBoolean(MobileGluesConfig.KEY_EXT_DSA, swDsa.isChecked())
                                .putBoolean(MobileGluesConfig.KEY_DEPTH_CLEAR_FIX, swDepth.isChecked())
                                .apply();
                    }
                    MobileGluesConfig.writeConfigJson(new File(Tools.DIR_DATA, "mobileglues"));
                    Toast.makeText(context, "Đã lưu cấu hình MobileGlues!", Toast.LENGTH_SHORT).show();
                })
                .setNeutralButton("Đặt mặc định", (dialog, which) -> {
                    MobileGluesConfig.resetDefaults();
                    MobileGluesConfig.writeConfigJson(new File(Tools.DIR_DATA, "mobileglues"));
                    Toast.makeText(context, "Đã khôi phục mặc định MobileGlues!", Toast.LENGTH_SHORT).show();
                })
                .setNegativeButton("Đóng", null)
                .show();
    }

    private static Switch addSwitch(LinearLayout root, Context ctx, String title, String subtitle, boolean checked, int gap) {
        Switch sw = new Switch(ctx);
        sw.setText(title);
        sw.setTextColor(Color.WHITE);
        sw.setChecked(checked);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        lp.topMargin = gap;
        root.addView(sw, lp);

        if (subtitle != null && !subtitle.isEmpty()) {
            TextView sub = new TextView(ctx);
            sub.setText(subtitle);
            sub.setTextColor(Color.parseColor("#C7A5A8"));
            sub.setTextSize(11f);
            root.addView(sub);
        }
        return sw;
    }

    private static Spinner addSpinner(LinearLayout root, Context ctx, String label, String[] items, int selectedIdx, int gap) {
        TextView tv = new TextView(ctx);
        tv.setText(label);
        tv.setTextColor(Color.parseColor("#F5B041"));
        tv.setTextSize(12f);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        lp.topMargin = gap;
        root.addView(tv, lp);

        Spinner sp = new Spinner(ctx);
        ArrayAdapter<String> adapter = new ArrayAdapter<>(ctx, android.R.layout.simple_spinner_dropdown_item, items);
        sp.setAdapter(adapter);
        if (selectedIdx >= 0 && selectedIdx < items.length) {
            sp.setSelection(selectedIdx);
        }
        root.addView(sp);
        return sp;
    }

    private static EditText addNumberField(LinearLayout root, Context ctx, String label, int currentVal, int gap) {
        TextView tv = new TextView(ctx);
        tv.setText(label);
        tv.setTextColor(Color.parseColor("#F5B041"));
        tv.setTextSize(12f);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        lp.topMargin = gap;
        root.addView(tv, lp);

        EditText et = new EditText(ctx);
        et.setInputType(InputType.TYPE_CLASS_NUMBER | InputType.TYPE_NUMBER_FLAG_SIGNED);
        et.setText(String.valueOf(currentVal));
        et.setTextColor(Color.WHITE);
        root.addView(et);
        return et;
    }
}
