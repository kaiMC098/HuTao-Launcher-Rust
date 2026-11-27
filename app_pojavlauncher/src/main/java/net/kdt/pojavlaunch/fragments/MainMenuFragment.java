package net.kdt.pojavlaunch.fragments;

import static net.kdt.pojavlaunch.Tools.openPath;
import static net.kdt.pojavlaunch.Tools.shareLog;

import android.app.ActivityManager;
import android.content.Context;
import android.content.Intent;
import android.os.Build;
import android.os.Bundle;
import android.text.SpannableStringBuilder;
import android.text.Spanned;
import android.text.style.ForegroundColorSpan;
import android.text.style.RelativeSizeSpan;
import android.view.View;
import android.widget.Button;
import android.widget.ImageButton;
import android.widget.ProgressBar;
import android.widget.TextView;
import android.widget.Toast;

import androidx.annotation.NonNull;
import androidx.annotation.Nullable;
import androidx.fragment.app.Fragment;

import com.kdt.mcgui.mcVersionSpinner;

import net.kdt.pojavlaunch.CustomControlsActivity;
import net.kdt.pojavlaunch.R;
import net.kdt.pojavlaunch.Tools;
import net.kdt.pojavlaunch.extra.ExtraConstants;
import net.kdt.pojavlaunch.extra.ExtraCore;
import net.kdt.pojavlaunch.prefs.LauncherPreferences;
import net.kdt.pojavlaunch.prefs.screens.LauncherPreferenceJavaFragment;
import net.kdt.pojavlaunch.progresskeeper.ProgressKeeper;
import net.kdt.pojavlaunch.rust.RustCoreBridge;
import net.kdt.pojavlaunch.value.launcherprofiles.LauncherProfiles;
import net.kdt.pojavlaunch.value.launcherprofiles.MinecraftProfile;

import java.io.File;
import java.util.Locale;

public class MainMenuFragment extends Fragment {
    public static final String TAG = "MainMenuFragment";
    private static final String HUTAO_DISCORD_URL = "https://discord.gg/2dqWuKJZES";

    private mcVersionSpinner mVersionSpinner;
    private Button mPlayButton;

    public MainMenuFragment(){
        super(R.layout.fragment_launcher);
    }

    @Override
    public void onViewCreated(@NonNull View view, @Nullable Bundle savedInstanceState) {
        Button mDiscordButton = view.findViewById(R.id.discord_button);
        Button mCustomControlButton = view.findViewById(R.id.custom_control_button);
        Button mInstallJarButton = view.findViewById(R.id.install_jar_button);
        Button mShareLogsButton = view.findViewById(R.id.share_logs_button);
        Button mOpenDirectoryButton = view.findViewById(R.id.open_files_button);
        Button mJavaSettingsButton = view.findViewById(R.id.java_settings_button);
        Button mAccountMenuButton = view.findViewById(R.id.account_menu_button);

        ImageButton mEditProfileButton = view.findViewById(R.id.edit_profile_button);
        mPlayButton = view.findViewById(R.id.play_button);
        mVersionSpinner = view.findViewById(R.id.mc_version_spinner);

        if (mDiscordButton != null) {
            mDiscordButton.setOnClickListener(v -> Tools.openURL(requireActivity(), HUTAO_DISCORD_URL));
        }
        if (mCustomControlButton != null) {
            mCustomControlButton.setOnClickListener(v -> startActivity(new Intent(requireContext(), CustomControlsActivity.class)));
            mCustomControlButton.setOnLongClickListener(v -> {
                Tools.swapFragment(requireActivity(), GamepadMapperFragment.class, GamepadMapperFragment.TAG, null);
                return true;
            });
        }
        if (mInstallJarButton != null) {
            mInstallJarButton.setOnClickListener(v -> runInstallerWithConfirmation(false));
            mInstallJarButton.setOnLongClickListener(v -> {
                runInstallerWithConfirmation(true);
                return true;
            });
        }
        if (mEditProfileButton != null) {
            mEditProfileButton.setOnClickListener(v -> mVersionSpinner.openProfileEditor(requireActivity()));
        }

        if (mJavaSettingsButton != null) {
            mJavaSettingsButton.setOnClickListener(v -> Tools.swapFragment(
                    requireActivity(),
                    LauncherPreferenceJavaFragment.class,
                    "JAVA_SETTINGS",
                    null
            ));
        }

        if (mAccountMenuButton != null) {
            mAccountMenuButton.setOnClickListener(v -> ExtraCore.setValue(ExtraConstants.SELECT_AUTH_METHOD, true));
        }

        mPlayButton.setOnClickListener(v -> ExtraCore.setValue(ExtraConstants.LAUNCH_GAME, true));

        if (mShareLogsButton != null) {
            mShareLogsButton.setOnClickListener(v -> shareLog(requireContext()));
        }

        if (mOpenDirectoryButton != null) {
            mOpenDirectoryButton.setOnClickListener(v -> {
                Tools.switchDemo(Tools.isDemoProfile(v.getContext()));
                if(Tools.isDemoProfile(v.getContext())){
                    Toast.makeText(v.getContext(), R.string.toast_not_available_demo, Toast.LENGTH_LONG).show();
                    return;
                }

                openPath(v.getContext(), getCurrentProfileDirectory(), false);
            });
        }

        updatePlayButtonLabel();
        updateRamAndRustStatus(view);
    }

    private void updatePlayButtonLabel() {
        if (mPlayButton == null) return;
        String subtitle = "Minecraft: Java Edition";
        try {
            String currentProfile = LauncherPreferences.DEFAULT_PREF.getString(LauncherPreferences.PREF_KEY_CURRENT_PROFILE, null);
            if (Tools.isValidString(currentProfile)) {
                LauncherProfiles.load();
                if (LauncherProfiles.mainProfileJson != null && LauncherProfiles.mainProfileJson.profiles != null) {
                    MinecraftProfile prof = LauncherProfiles.mainProfileJson.profiles.get(currentProfile);
                    if (prof != null && Tools.isValidString(prof.lastVersionId)) {
                        subtitle = "Minecraft " + prof.lastVersionId;
                    }
                }
            }
        } catch (Throwable ignored) {}

        String title = "CHƠI (PLAY)";
        SpannableStringBuilder ssb = new SpannableStringBuilder(title + "\n" + subtitle);
        ssb.setSpan(new RelativeSizeSpan(1.08f), 0, title.length(), Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
        ssb.setSpan(new RelativeSizeSpan(0.72f), title.length() + 1, ssb.length(), Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
        ssb.setSpan(new ForegroundColorSpan(0xFFFFE5DC), title.length() + 1, ssb.length(), Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
        mPlayButton.setText(ssb);
    }

    private void updateRamAndRustStatus(View root) {
        if (root == null) return;
        TextView memText = root.findViewById(R.id.hutao_memory_text);
        ProgressBar memBar = root.findViewById(R.id.hutao_memory_bar);
        if (memText != null && memBar != null) {
            try {
                ActivityManager am = (ActivityManager) requireContext().getSystemService(Context.ACTIVITY_SERVICE);
                ActivityManager.MemoryInfo mi = new ActivityManager.MemoryInfo();
                if (am != null) {
                    am.getMemoryInfo(mi);
                    double totalGb = mi.totalMem / (1024.0 * 1024.0 * 1024.0);
                    double usedGb = (mi.totalMem - mi.availMem) / (1024.0 * 1024.0 * 1024.0);
                    int pct = (int) Math.min(100, Math.max(5, Math.round((usedGb / Math.max(0.1, totalGb)) * 100.0)));
                    memText.setText(String.format(Locale.US, "Memory: %.1f GB / %.1f GB", usedGb, totalGb));
                    memBar.setProgress(pct);
                }
            } catch (Throwable ignored) {}
        }

        TextView rustStatus = root.findViewById(R.id.hutao_rust_status_text);
        if (rustStatus != null) {
            String abi = Build.SUPPORTED_ABIS.length > 0 ? Build.SUPPORTED_ABIS[0] : "aarch64";
            if ("arm64-v8a".equals(abi)) abi = "aarch64";
            boolean ok = RustCoreBridge.isInitialized();
            rustStatus.setText(ok
                    ? "[Rust] Loaded jniLibs successfully. Host: Android_" + abi
                    : "[Rust] Native bridge standby. Host: Android_" + abi);
        }
    }

    private File getCurrentProfileDirectory() {
        String currentProfile = LauncherPreferences.DEFAULT_PREF.getString(LauncherPreferences.PREF_KEY_CURRENT_PROFILE, null);
        if(!Tools.isValidString(currentProfile)) return new File(Tools.DIR_GAME_NEW);
        LauncherProfiles.load();
        MinecraftProfile profileObject = LauncherProfiles.mainProfileJson.profiles.get(currentProfile);
        if(profileObject == null) return new File(Tools.DIR_GAME_NEW);
        return Tools.getGameDirPath(profileObject);
    }

    @Override
    public void onResume() {
        super.onResume();
        mVersionSpinner.reloadProfiles();
        updatePlayButtonLabel();
        updateRamAndRustStatus(getView());
    }

    private void runInstallerWithConfirmation(boolean isCustomArgs) {
        if(Tools.isLocalProfile(requireContext()) || Tools.isDemoProfile(requireContext())){
            Toast.makeText(requireContext(), R.string.toast_not_available_demo, Toast.LENGTH_LONG).show();
            return;
        }

        if (ProgressKeeper.getTaskCount() == 0)
            Tools.installMod(requireActivity(), isCustomArgs);
        else
            Toast.makeText(requireContext(), R.string.tasks_ongoing, Toast.LENGTH_LONG).show();
    }
}
