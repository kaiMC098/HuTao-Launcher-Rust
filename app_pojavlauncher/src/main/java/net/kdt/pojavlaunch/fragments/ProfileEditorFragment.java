package net.kdt.pojavlaunch.fragments;

import android.app.AlertDialog;
import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.Color;
import android.os.Build;
import android.os.Bundle;
import android.util.Base64;
import android.util.Base64OutputStream;
import android.util.Log;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.EditText;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.Spinner;
import android.widget.TextView;
import android.widget.Toast;

import androidx.activity.result.ActivityResultLauncher;
import androidx.annotation.NonNull;
import androidx.annotation.Nullable;
import androidx.fragment.app.Fragment;

import net.kdt.pojavlaunch.PojavApplication;
import net.kdt.pojavlaunch.R;
import net.kdt.pojavlaunch.Tools;
import net.kdt.pojavlaunch.extra.ExtraConstants;
import net.kdt.pojavlaunch.extra.ExtraCore;
import net.kdt.pojavlaunch.multirt.MultiRTUtils;
import net.kdt.pojavlaunch.multirt.RTSpinnerAdapter;
import net.kdt.pojavlaunch.multirt.Runtime;
import net.kdt.pojavlaunch.prefs.LauncherPreferences;
import net.kdt.pojavlaunch.profiles.ProfileIconCache;
import net.kdt.pojavlaunch.profiles.VersionSelectorDialog;
import net.kdt.pojavlaunch.utils.CropperUtils;
import net.kdt.pojavlaunch.value.launcherprofiles.LauncherProfiles;
import net.kdt.pojavlaunch.value.launcherprofiles.MinecraftProfile;

import org.apache.commons.io.FileUtils;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Locale;

public class ProfileEditorFragment extends Fragment implements CropperUtils.CropperListener {
    public static final String TAG = "ProfileEditorFragment";
    public static final String DELETED_PROFILE = "deleted_profile";

    private String mProfileKey;
    private MinecraftProfile mTempProfile = null;
    private String mValueToConsume = "";
    private Button mSaveButton, mDeleteButton, mControlSelectButton, mGameDirButton, mVersionSelectButton;
    private Button mIsolateButton, mCloneButton;
    private Spinner mDefaultRuntime, mDefaultRenderer;
    private EditText mDefaultName, mDefaultJvmArgument;
    private TextView mDefaultPath, mDefaultVersion, mDefaultControl;
    private ImageView mProfileIcon;
    private final ActivityResultLauncher<?> mCropperLauncher = CropperUtils.registerCropper(this, this);

    private List<String> mRenderNames;

    public ProfileEditorFragment() {
        super(R.layout.fragment_profile_editor);
    }

    @Nullable
    @Override
    public View onCreateView(@NonNull LayoutInflater inflater, @Nullable ViewGroup container, @Nullable Bundle savedInstanceState) {
        String value = (String) ExtraCore.consumeValue(ExtraConstants.FILE_SELECTOR);
        if (value != null) {
            if (mValueToConsume.equals(FileSelectorFragment.BUNDLE_SELECT_FOLDER)) {
                mTempProfile.gameDir = value;
            } else {
                mTempProfile.controlFile = value;
            }
        }
        return super.onCreateView(inflater, container, savedInstanceState);
    }

    @Override
    public void onViewCreated(@NonNull View view, @Nullable Bundle savedInstanceState) {
        bindViews(view);

        Tools.RenderersList renderersList = Tools.getCompatibleRenderers(view.getContext());
        mRenderNames = renderersList.rendererIds;
        List<String> renderList = new ArrayList<>(renderersList.rendererDisplayNames.length + 1);
        renderList.addAll(Arrays.asList(renderersList.rendererDisplayNames));
        renderList.add(view.getContext().getString(R.string.global_default));
        mDefaultRenderer.setAdapter(new ArrayAdapter<>(getContext(), R.layout.item_simple_list_1, renderList));

        mSaveButton.setOnClickListener(v -> {
            ProfileIconCache.dropIcon(mProfileKey);
            save();
            Tools.backToMainMenu(requireActivity());
        });

        mDeleteButton.setOnClickListener(v -> showDeleteVersionDialog());

        if (mIsolateButton != null) {
            mIsolateButton.setOnClickListener(v -> {
                String baseName = mDefaultName.getText().toString().trim();
                if (baseName.isEmpty()) baseName = mDefaultVersion.getText().toString().trim();
                if (baseName.isEmpty()) baseName = "isolated";
                String safeDir = baseName.replaceAll("[^a-zA-Z0-9._-]", "_");
                String isolatedPath = "./versions/" + safeDir;
                mTempProfile.gameDir = isolatedPath;
                mDefaultPath.setText(isolatedPath);
                Toast.makeText(requireContext(), "Đã tách thư mục riêng: " + isolatedPath, Toast.LENGTH_SHORT).show();
            });
        }

        if (mCloneButton != null) {
            mCloneButton.setOnClickListener(v -> {
                syncInputsToTempProfile();
                String newKey = LauncherProfiles.getFreeProfileKey();
                MinecraftProfile cloned = new MinecraftProfile(mTempProfile);
                String baseName = (cloned.name == null || cloned.name.trim().isEmpty()) ? cloned.lastVersionId : cloned.name;
                cloned.name = baseName + " (Copy)";
                LauncherProfiles.mainProfileJson.profiles.put(newKey, cloned);
                LauncherProfiles.write();
                ExtraCore.setValue(ExtraConstants.REFRESH_VERSION_SPINNER, newKey);
                Toast.makeText(requireContext(), "Đã nhân bản: " + cloned.name, Toast.LENGTH_SHORT).show();
                Tools.backToMainMenu(requireActivity());
            });
        }

        View.OnClickListener gameDirListener = getGameDirListener();
        mGameDirButton.setOnClickListener(gameDirListener);
        mDefaultPath.setOnClickListener(gameDirListener);

        View.OnClickListener controlSelectListener = getControlSelectListener();
        mControlSelectButton.setOnClickListener(controlSelectListener);
        mDefaultControl.setOnClickListener(controlSelectListener);

        View.OnClickListener versionSelectListener = getVersionSelectListener();
        mVersionSelectButton.setOnClickListener(versionSelectListener);
        mDefaultVersion.setOnClickListener(versionSelectListener);

        mProfileIcon.setOnClickListener(v -> CropperUtils.startCropper(mCropperLauncher));

        loadValues(LauncherPreferences.DEFAULT_PREF.getString(LauncherPreferences.PREF_KEY_CURRENT_PROFILE, ""), view.getContext());
    }

    private void showDeleteVersionDialog() {
        Context ctx = requireContext();
        String verId = mDefaultVersion.getText().toString().trim();
        File versionDir = (!verId.isEmpty() && !verId.startsWith("latest-"))
                ? new File(Tools.DIR_HOME_VERSION, verId)
                : null;
        File nativesCacheDir = (!verId.isEmpty() && !verId.startsWith("latest-"))
                ? new File(Tools.DIR_CACHE, "natives/" + verId)
                : null;

        File customGameDir = null;
        String rawGameDir = mDefaultPath.getText().toString().trim();
        if (!rawGameDir.isEmpty() && !rawGameDir.equals(".minecraft")) {
            File candidate = Tools.getGameDirPath(mTempProfile);
            File defaultRoot = new File(Tools.DIR_GAME_NEW);
            try {
                if (candidate.exists() && !candidate.getCanonicalPath().equals(defaultRoot.getCanonicalPath())) {
                    customGameDir = candidate;
                }
            } catch (IOException ignored) {}
        }

        long versionBytes = (versionDir != null && versionDir.exists()) ? safeDirSize(versionDir) : 0L;
        long nativeBytes = (nativesCacheDir != null && nativesCacheDir.exists()) ? safeDirSize(nativesCacheDir) : 0L;
        long gameDirBytes = (customGameDir != null && customGameDir.exists()) ? safeDirSize(customGameDir) : 0L;
        double versionMb = (versionBytes + nativeBytes) / (1024.0 * 1024.0);
        double gameDirMb = gameDirBytes / (1024.0 * 1024.0);

        LinearLayout container = new LinearLayout(ctx);
        container.setOrientation(LinearLayout.VERTICAL);
        int pad = (int) (16 * ctx.getResources().getDisplayMetrics().density);
        container.setPadding(pad, pad / 2, pad, pad / 2);

        TextView info = new TextView(ctx);
        info.setTextColor(Color.WHITE);
        info.setText(String.format(
                Locale.US,
                "Phiên bản: %s\nDữ liệu tệp phiên bản (.jar/.json/natives): %.1f MB\n\nChọn cách xóa bên dưới:",
                verId.isEmpty() ? "(Chưa chọn)" : verId,
                versionMb
        ));
        container.addView(info);

        final CheckBox cbDeleteIsolatedDir = new CheckBox(ctx);
        final File finalCustomGameDir = customGameDir;
        if (finalCustomGameDir != null) {
            cbDeleteIsolatedDir.setText(String.format(
                    Locale.US,
                    "Xóa luôn thư mục tách riêng (saves/mods): %.1f MB",
                    gameDirMb
            ));
            cbDeleteIsolatedDir.setTextColor(Color.parseColor("#FF9E8A"));
            cbDeleteIsolatedDir.setChecked(false);
            container.addView(cbDeleteIsolatedDir);
        }

        final File finalVersionDir = versionDir;
        final File finalNativesDir = nativesCacheDir;

        new AlertDialog.Builder(ctx)
                .setTitle("Xóa phiên bản & dữ liệu")
                .setView(container)
                .setPositiveButton("Xóa sạch phiên bản", (d, w) -> {
                    boolean deleteGameDir = finalCustomGameDir != null && cbDeleteIsolatedDir.isChecked();
                    PojavApplication.sExecutorService.execute(() -> {
                        long freed = 0L;
                        try {
                            if (finalVersionDir != null && finalVersionDir.exists()) {
                                freed += safeDirSize(finalVersionDir);
                                FileUtils.deleteDirectory(finalVersionDir);
                            }
                            if (finalNativesDir != null && finalNativesDir.exists()) {
                                freed += safeDirSize(finalNativesDir);
                                FileUtils.deleteDirectory(finalNativesDir);
                            }
                            if (deleteGameDir && finalCustomGameDir.exists()) {
                                freed += safeDirSize(finalCustomGameDir);
                                FileUtils.deleteDirectory(finalCustomGameDir);
                            }
                        } catch (Throwable e) {
                            Log.e(TAG, "Error deleting version files", e);
                        }
                        final double freedMb = freed / (1024.0 * 1024.0);
                        Tools.runOnUiThread(() -> {
                            removeProfileEntryOnly();
                            if (isAdded()) {
                                Toast.makeText(
                                        requireContext(),
                                        String.format(Locale.US, "Đã xóa phiên bản và giải phóng %.1f MB!", freedMb),
                                        Toast.LENGTH_LONG
                                ).show();
                                Tools.removeCurrentFragment(requireActivity());
                            }
                        });
                    });
                })
                .setNeutralButton("Chỉ xóa cấu hình", (d, w) -> {
                    removeProfileEntryOnly();
                    Tools.removeCurrentFragment(requireActivity());
                })
                .setNegativeButton("Hủy", null)
                .show();
    }

    private void removeProfileEntryOnly() {
        if (LauncherProfiles.mainProfileJson != null
                && LauncherProfiles.mainProfileJson.profiles != null
                && LauncherProfiles.mainProfileJson.profiles.size() > 1) {
            ProfileIconCache.dropIcon(mProfileKey);
            LauncherProfiles.mainProfileJson.profiles.remove(mProfileKey);
            LauncherProfiles.write();
            ExtraCore.setValue(ExtraConstants.REFRESH_VERSION_SPINNER, DELETED_PROFILE);
        }
    }

    private long safeDirSize(File dir) {
        try {
            return FileUtils.sizeOfDirectory(dir);
        } catch (Throwable ignored) {
            return 0L;
        }
    }

    private View.OnClickListener getGameDirListener() {
        return v -> {
            Bundle bundle = new Bundle(2);
            bundle.putBoolean(FileSelectorFragment.BUNDLE_SELECT_FOLDER, true);
            bundle.putString(FileSelectorFragment.BUNDLE_ROOT_PATH, Tools.DIR_GAME_HOME);
            bundle.putBoolean(FileSelectorFragment.BUNDLE_SHOW_FILE, false);
            mValueToConsume = FileSelectorFragment.BUNDLE_SELECT_FOLDER;

            Tools.swapFragment(requireActivity(),
                    FileSelectorFragment.class, FileSelectorFragment.TAG, bundle);
        };
    }

    private View.OnClickListener getControlSelectListener() {
        return v -> {
            Bundle bundle = new Bundle(3);
            bundle.putBoolean(FileSelectorFragment.BUNDLE_SELECT_FOLDER, false);
            bundle.putString(FileSelectorFragment.BUNDLE_ROOT_PATH, Tools.CTRLMAP_PATH);
            mValueToConsume = FileSelectorFragment.BUNDLE_SELECT_FILE;

            Tools.swapFragment(requireActivity(),
                    FileSelectorFragment.class, FileSelectorFragment.TAG, bundle);
        };
    }

    private View.OnClickListener getVersionSelectListener() {
        return v -> VersionSelectorDialog.open(v.getContext(), false, (id, snapshot) -> {
            mTempProfile.lastVersionId = id;
            mDefaultVersion.setText(id);
        });
    }

    private void loadValues(@NonNull String profile, @NonNull Context context) {
        if (mTempProfile == null) {
            mTempProfile = getProfile(profile);
        }
        mProfileIcon.setImageDrawable(
                ProfileIconCache.fetchIcon(getResources(), mProfileKey, mTempProfile.icon)
        );

        List<Runtime> runtimes = MultiRTUtils.getRuntimes();
        int jvmIndex = runtimes.indexOf(new Runtime("<Default>"));
        if (mTempProfile.javaDir != null) {
            String selectedRuntime = mTempProfile.javaDir.substring(Tools.LAUNCHERPROFILES_RTPREFIX.length());
            int nindex = runtimes.indexOf(new Runtime(selectedRuntime));
            if (nindex != -1) jvmIndex = nindex;
        }
        mDefaultRuntime.setAdapter(new RTSpinnerAdapter(context, runtimes));
        if (jvmIndex == -1) jvmIndex = runtimes.size() - 1;
        mDefaultRuntime.setSelection(jvmIndex);

        int rendererIndex = mDefaultRenderer.getAdapter().getCount() - 1;
        if (mTempProfile.pojavRendererName != null) {
            int nindex = mRenderNames.indexOf(mTempProfile.pojavRendererName);
            if (nindex != -1) rendererIndex = nindex;
        }
        mDefaultRenderer.setSelection(rendererIndex);

        mDefaultVersion.setText(mTempProfile.lastVersionId);
        mDefaultJvmArgument.setText(mTempProfile.javaArgs == null ? "" : mTempProfile.javaArgs);
        mDefaultName.setText(mTempProfile.name);
        mDefaultPath.setText(mTempProfile.gameDir == null ? "" : mTempProfile.gameDir);
        mDefaultControl.setText(mTempProfile.controlFile == null ? "" : mTempProfile.controlFile);
    }

    private MinecraftProfile getProfile(@NonNull String profile) {
        MinecraftProfile minecraftProfile;
        if (getArguments() == null) {
            LauncherProfiles.load();
            MinecraftProfile originalProfile = LauncherProfiles.mainProfileJson.profiles.get(profile);
            if (originalProfile != null) minecraftProfile = new MinecraftProfile(originalProfile);
            else minecraftProfile = MinecraftProfile.createTemplate();
            mProfileKey = profile;
        } else {
            minecraftProfile = MinecraftProfile.createTemplate();
            mProfileKey = LauncherProfiles.getFreeProfileKey();
        }
        return minecraftProfile;
    }

    private void bindViews(@NonNull View view) {
        mDefaultControl = view.findViewById(R.id.vprof_editor_ctrl_spinner);
        mDefaultRuntime = view.findViewById(R.id.vprof_editor_spinner_runtime);
        mDefaultRenderer = view.findViewById(R.id.vprof_editor_profile_renderer);
        mDefaultVersion = view.findViewById(R.id.vprof_editor_version_spinner);

        mDefaultPath = view.findViewById(R.id.vprof_editor_path);
        mDefaultName = view.findViewById(R.id.vprof_editor_profile_name);
        mDefaultJvmArgument = view.findViewById(R.id.vprof_editor_jre_args);

        mSaveButton = view.findViewById(R.id.vprof_editor_save_button);
        mDeleteButton = view.findViewById(R.id.vprof_editor_delete_button);
        mIsolateButton = view.findViewById(R.id.vprof_editor_isolate_button);
        mCloneButton = view.findViewById(R.id.vprof_editor_clone_button);
        mControlSelectButton = view.findViewById(R.id.vprof_editor_ctrl_button);
        mVersionSelectButton = view.findViewById(R.id.vprof_editor_version_button);
        mGameDirButton = view.findViewById(R.id.vprof_editor_path_button);
        mProfileIcon = view.findViewById(R.id.vprof_editor_profile_icon);
    }

    private void syncInputsToTempProfile() {
        mTempProfile.lastVersionId = mDefaultVersion.getText().toString();
        mTempProfile.controlFile = mDefaultControl.getText().toString();
        mTempProfile.name = mDefaultName.getText().toString();
        mTempProfile.javaArgs = mDefaultJvmArgument.getText().toString();
        mTempProfile.gameDir = mDefaultPath.getText().toString();

        if (mTempProfile.controlFile.isEmpty()) mTempProfile.controlFile = null;
        if (mTempProfile.javaArgs.isEmpty()) mTempProfile.javaArgs = null;
        if (mTempProfile.gameDir.isEmpty()) mTempProfile.gameDir = null;

        Runtime selectedRuntime = (Runtime) mDefaultRuntime.getSelectedItem();
        if (selectedRuntime != null) {
            mTempProfile.javaDir = (selectedRuntime.name.equals("<Default>") || selectedRuntime.versionString == null)
                    ? null : Tools.LAUNCHERPROFILES_RTPREFIX + selectedRuntime.name;
        }

        if (mDefaultRenderer.getSelectedItemPosition() == mRenderNames.size()) mTempProfile.pojavRendererName = null;
        else mTempProfile.pojavRendererName = mRenderNames.get(mDefaultRenderer.getSelectedItemPosition());
    }

    private void save() {
        syncInputsToTempProfile();
        LauncherProfiles.mainProfileJson.profiles.put(mProfileKey, mTempProfile);
        LauncherProfiles.write();
        ExtraCore.setValue(ExtraConstants.REFRESH_VERSION_SPINNER, mProfileKey);
    }

    @Override
    public void onCropped(Bitmap contentBitmap) {
        mProfileIcon.setImageBitmap(contentBitmap);
        ByteArrayOutputStream byteArrayOutputStream = new ByteArrayOutputStream();
        try (Base64OutputStream base64OutputStream = new Base64OutputStream(byteArrayOutputStream, Base64.NO_WRAP)) {
            contentBitmap.compress(
                    Build.VERSION.SDK_INT < Build.VERSION_CODES.R
                            ? Bitmap.CompressFormat.WEBP
                            : Bitmap.CompressFormat.WEBP_LOSSY,
                    60,
                    base64OutputStream
            );
            base64OutputStream.flush();
            byteArrayOutputStream.flush();
        } catch (IOException e) {
            Tools.showErrorRemote(e);
            return;
        }
        String iconLine = new String(byteArrayOutputStream.toByteArray(), StandardCharsets.UTF_8);
        mTempProfile.icon = "data:image/webp;base64," + iconLine;
    }

    @Override
    public void onFailed(Exception exception) {
        Tools.showErrorRemote(exception);
    }
}
