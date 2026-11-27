package net.kdt.pojavlaunch.tasks;


import static net.kdt.pojavlaunch.Architecture.archAsString;
import static net.kdt.pojavlaunch.PojavApplication.sExecutorService;

import android.content.Context;
import android.content.res.AssetManager;
import android.util.Log;

import com.kdt.mcgui.ProgressLayout;

import net.kdt.pojavlaunch.Tools;
import net.kdt.pojavlaunch.multirt.MultiRTUtils;

import org.apache.commons.io.FileUtils;

import java.io.File;
import java.io.FileInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;

public class AsyncAssetManager {

    private static volatile CountDownLatch sRuntimeUnpackLatch = new CountDownLatch(0);

    private AsyncAssetManager(){}

    public static void waitForRuntimeUnpack() {
        CountDownLatch latch = sRuntimeUnpackLatch;
        if (latch != null && latch.getCount() > 0) {
            try {
                Log.i("JREAuto", "Waiting for bundled Java runtime unpack to finish...");
                latch.await(90, TimeUnit.SECONDS);
            } catch (InterruptedException ignored) {
                Thread.currentThread().interrupt();
            }
        }
    }

    /**
     * Unpack all bundled internal Java runtimes (JRE 8, JRE 17, JRE 21) if included in the APK.
     * @param am App AssetManager
     */
    public static void unpackRuntime(AssetManager am) {
        final String[][] bundledRuntimes = new String[][]{
                {"components/jre", "Internal"},
                {"components/jre-new", "Internal-17"},
                {"components/jre-21", "Internal-21"}
        };

        boolean anyPending = false;
        for (String[] entry : bundledRuntimes) {
            if (needsUnpack(am, entry[0], entry[1])) {
                anyPending = true;
                break;
            }
        }
        if (!anyPending) return;

        final CountDownLatch latch = new CountDownLatch(1);
        sRuntimeUnpackLatch = latch;

        sExecutorService.execute(() -> {
            try {
                String arch = archAsString(Tools.DEVICE_ARCHITECTURE);
                for (String[] entry : bundledRuntimes) {
                    String assetDir = entry[0];
                    String runtimeName = entry[1];
                    try {
                        String rtVersion = Tools.read(am.open(assetDir + "/version"));
                        String currentVersion = MultiRTUtils.readInternalRuntimeVersion(runtimeName);
                        if (rtVersion == null || rtVersion.equals(currentVersion)) continue;

                        InputStream uniStream = am.open(assetDir + "/universal.tar.xz");
                        InputStream binStream = am.open(assetDir + "/bin-" + arch + ".tar.xz");
                        MultiRTUtils.installRuntimeNamedBinpack(uniStream, binStream, runtimeName, rtVersion);
                        MultiRTUtils.postPrepare(runtimeName);
                        Log.i("JREAuto", "Unpacked bundled runtime: " + runtimeName + " (" + arch + ")");
                    } catch (java.io.FileNotFoundException e) {
                        Log.i("JREAuto", "Runtime " + runtimeName + " not bundled for " + arch + ": " + e.getMessage());
                    } catch (IOException | RuntimeException e) {
                        Log.e("JREAuto", "Failed to unpack runtime " + runtimeName + " for " + arch, e);
                    }
                }
            } finally {
                ProgressLayout.clearProgress(ProgressLayout.UNPACK_RUNTIME);
                latch.countDown();
            }
        });
    }

    private static boolean needsUnpack(AssetManager am, String assetDir, String runtimeName) {
        try {
            String rtVersion = Tools.read(am.open(assetDir + "/version"));
            String currentVersion = MultiRTUtils.readInternalRuntimeVersion(runtimeName);
            return rtVersion != null && !rtVersion.equals(currentVersion);
        } catch (IOException ignored) {
            return false;
        }
    }

    /** Unpack single files, with no regard to version tracking */
    public static void unpackSingleFiles(Context ctx){
        ProgressLayout.setProgress(ProgressLayout.EXTRACT_SINGLE_FILES, 0);
        sExecutorService.execute(() -> {
            try {
                Tools.copyAssetFile(ctx, "options.txt", Tools.DIR_GAME_NEW, false);
                Tools.copyAssetFile(ctx, "default.json", Tools.CTRLMAP_PATH, false);

                Tools.copyAssetFile(ctx, "launcher_profiles.json", Tools.DIR_GAME_NEW, false);
                Tools.copyAssetFile(ctx,"resolv.conf",Tools.DIR_DATA, false);
            } catch (IOException e) {
                Log.e("AsyncAssetManager", "Failed to unpack critical components !");
            }
            ProgressLayout.clearProgress(ProgressLayout.EXTRACT_SINGLE_FILES);
        });
    }

    public static void unpackComponents(Context ctx){
        ProgressLayout.setProgress(ProgressLayout.EXTRACT_COMPONENTS, 0);
        sExecutorService.execute(() -> {
            try {
                unpackComponent(ctx, "caciocavallo", false);
                unpackComponent(ctx, "caciocavallo17", false);
                unpackComponent(ctx, "lwjgl3", false);
                unpackComponent(ctx, "security", true);
                unpackComponent(ctx, "arc_dns_injector", true);
                unpackComponent(ctx, "forge_installer", true);
            } catch (IOException e) {
                Log.e("AsyncAssetManager", "Failed to unpack components !", e);
            }
            ProgressLayout.clearProgress(ProgressLayout.EXTRACT_COMPONENTS);
        });
    }

    private static void unpackComponent(Context ctx, String component, boolean privateDirectory) throws IOException {
        AssetManager am = ctx.getAssets();
        String rootDir = privateDirectory ? Tools.DIR_DATA : Tools.DIR_GAME_HOME;

        File versionFile = new File(rootDir + "/" + component + "/version");
        InputStream is = am.open("components/" + component + "/version");
        if(!versionFile.exists()) {
            if (versionFile.getParentFile().exists() && versionFile.getParentFile().isDirectory()) {
                FileUtils.deleteDirectory(versionFile.getParentFile());
            }
            versionFile.getParentFile().mkdir();

            Log.i("UnpackPrep", component + ": Pack was installed manually, or does not exist, unpacking new...");
            String[] fileList = am.list("components/" + component);
            for(String s : fileList) {
                Tools.copyAssetFile(ctx, "components/" + component + "/" + s, rootDir + "/" + component, true);
            }
        } else {
            FileInputStream fis = new FileInputStream(versionFile);
            String release1 = Tools.read(is);
            String release2 = Tools.read(fis);
            if (!release1.equals(release2)) {
                if (versionFile.getParentFile().exists() && versionFile.getParentFile().isDirectory()) {
                    FileUtils.deleteDirectory(versionFile.getParentFile());
                }
                versionFile.getParentFile().mkdir();

                String[] fileList = am.list("components/" + component);
                for (String fileName : fileList) {
                    Tools.copyAssetFile(ctx, "components/" + component + "/" + fileName, rootDir + "/" + component, true);
                }
            } else {
                Log.i("UnpackPrep", component + ": Pack is up-to-date with the launcher, continuing...");
            }
        }
    }
}
