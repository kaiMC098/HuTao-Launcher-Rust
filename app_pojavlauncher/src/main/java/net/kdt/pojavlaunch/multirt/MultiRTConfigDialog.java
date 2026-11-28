package net.kdt.pojavlaunch.multirt;

import android.annotation.SuppressLint;
import android.app.AlertDialog;
import android.content.Context;
import android.widget.Button;
import android.widget.Toast;

import androidx.activity.result.ActivityResultLauncher;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;

import net.kdt.pojavlaunch.R;
import net.kdt.pojavlaunch.tasks.AsyncAssetManager;

public class MultiRTConfigDialog {
    private AlertDialog mDialog;
    private RecyclerView mDialogView;

    public void show() {
        refresh();
        mDialog.show();
    }

    @SuppressLint("NotifyDataSetChanged")
    public void refresh() {
        MultiRTUtils.clearCache();
        RecyclerView.Adapter<?> adapter = mDialogView.getAdapter();
        if (adapter != null) adapter.notifyDataSetChanged();
    }

    public void prepare(Context activity, ActivityResultLauncher<Object> installJvmLauncher) {
        mDialogView = new RecyclerView(activity);
        mDialogView.setLayoutManager(new LinearLayoutManager(activity, LinearLayoutManager.VERTICAL, false));
        RTRecyclerViewAdapter adapter = new RTRecyclerViewAdapter();
        mDialogView.setAdapter(adapter);

        mDialog = new AlertDialog.Builder(activity)
                .setTitle(R.string.multirt_config_title)
                .setView(mDialogView)
                .setPositiveButton(R.string.multirt_config_add, (dialog, which) -> installJvmLauncher.launch(null))
                .setNegativeButton("Sửa Java", null)
                .setNeutralButton(R.string.multirt_delete_runtime, null)
                .create();

        mDialog.setOnShowListener(dialog -> {
            Button deleteBtn = ((AlertDialog) dialog).getButton(AlertDialog.BUTTON_NEUTRAL);
            deleteBtn.setOnClickListener(view -> {
                boolean isEditing = !adapter.getIsEditing();
                adapter.setIsEditing(isEditing);
                deleteBtn.setText(isEditing ? R.string.multirt_config_setdefault : R.string.multirt_delete_runtime);
            });

            Button repairBtn = ((AlertDialog) dialog).getButton(AlertDialog.BUTTON_NEGATIVE);
            repairBtn.setOnClickListener(view -> {
                repairBtn.setEnabled(false);
                Toast.makeText(activity, "Đang sửa và giải nén lại JRE 8/17/21/25...", Toast.LENGTH_SHORT).show();
                AsyncAssetManager.forceRepairRuntimes(activity.getAssets(), () -> {
                    repairBtn.setEnabled(true);
                    refresh();
                    Toast.makeText(activity, "Đã khôi phục Java Runtime thành công!", Toast.LENGTH_SHORT).show();
                });
            });
        });
    }
}
