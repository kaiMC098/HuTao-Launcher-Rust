package net.kdt.pojavlaunch.utils;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.graphics.Color;
import android.graphics.Typeface;
import android.widget.HorizontalScrollView;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;

import net.kdt.pojavlaunch.Tools;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileReader;
import java.util.ArrayDeque;

/**
 * In-app quick log viewer & 1-tap clipboard copy for latestlog.txt / latestcrash.txt.
 */
public final class LogViewerDialog {
    private static final int MAX_LINES = 250;

    private LogViewerDialog() {}

    public static void show(Activity activity) {
        if (activity == null) return;
        File logFile = new File(Tools.DIR_GAME_HOME, "latestlog.txt");
        File crashFile = new File(Tools.DIR_GAME_HOME, "latestcrash.txt");
        if (!logFile.exists() && crashFile.exists()) {
            showFile(activity, crashFile);
        } else {
            showFile(activity, logFile);
        }
    }

    private static void showFile(Activity activity, File targetFile) {
        final String content = readTail(targetFile, MAX_LINES);

        ScrollView vScroll = new ScrollView(activity);
        HorizontalScrollView hScroll = new HorizontalScrollView(activity);
        TextView tv = new TextView(activity);
        tv.setText(content);
        tv.setTypeface(Typeface.MONOSPACE);
        tv.setTextSize(10.5f);
        tv.setTextColor(Color.parseColor("#F5E9E2"));
        tv.setTextIsSelectable(true);
        int pad = (int) (10 * activity.getResources().getDisplayMetrics().density);
        tv.setPadding(pad, pad, pad, pad);
        tv.setBackgroundColor(Color.parseColor("#1A0709"));

        hScroll.addView(tv);
        vScroll.addView(hScroll);
        vScroll.post(() -> vScroll.fullScroll(ScrollView.FOCUS_DOWN));

        AlertDialog.Builder builder = new AlertDialog.Builder(activity)
                .setTitle("Nhật ký: " + targetFile.getName())
                .setView(vScroll)
                .setPositiveButton("Sao chép Log", (d, w) -> {
                    ClipboardManager cm = (ClipboardManager) activity.getSystemService(Context.CLIPBOARD_SERVICE);
                    if (cm != null) {
                        cm.setPrimaryClip(ClipData.newPlainText(targetFile.getName(), content));
                        Toast.makeText(activity, "Đã sao chép " + targetFile.getName() + " vào bộ nhớ tạm!", Toast.LENGTH_SHORT).show();
                    }
                })
                .setNeutralButton("Chia sẻ tệp", (d, w) -> Tools.shareLog(activity))
                .setNegativeButton("Đóng", null);

        builder.show();
    }

    private static String readTail(File file, int maxLines) {
        if (!file.exists() || !file.isFile()) {
            return "(Chưa có tệp " + file.getName() + " — hãy thử mở game trước)";
        }
        ArrayDeque<String> deque = new ArrayDeque<>(maxLines + 1);
        try (BufferedReader br = new BufferedReader(new FileReader(file))) {
            String line;
            while ((line = br.readLine()) != null) {
                if (deque.size() >= maxLines) deque.removeFirst();
                deque.addLast(line);
            }
        } catch (Exception e) {
            return "Lỗi đọc log: " + e.getMessage();
        }
        if (deque.isEmpty()) return "(Tệp log đang trống)";
        StringBuilder sb = new StringBuilder();
        for (String s : deque) {
            sb.append(s).append('\n');
        }
        return sb.toString();
    }
}
