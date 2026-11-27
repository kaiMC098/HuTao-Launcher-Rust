package net.kdt.pojavlaunch.multirt;

import static net.kdt.pojavlaunch.Tools.NATIVE_LIB_DIR;
import static org.apache.commons.io.FileUtils.listFiles;

import android.system.Os;
import android.util.Log;

import com.kdt.mcgui.ProgressLayout;

import net.kdt.pojavlaunch.R;
import net.kdt.pojavlaunch.Tools;
import net.kdt.pojavlaunch.utils.MathUtils;

import org.apache.commons.compress.archivers.tar.TarArchiveEntry;
import org.apache.commons.compress.archivers.tar.TarArchiveInputStream;
import org.apache.commons.compress.compressors.xz.XZCompressorInputStream;
import org.apache.commons.io.FileUtils;
import org.apache.commons.io.IOUtils;

import java.io.BufferedInputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.util.ArrayList;
import java.util.Collection;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.zip.GZIPInputStream;

public class MultiRTUtils {

    private static final HashMap<String,Runtime> sCache = new HashMap<>();

    private static final File RUNTIME_FOLDER = new File(Tools.MULTIRT_HOME);
    private static final String JAVA_VERSION_STR = "JAVA_VERSION=\"";
    private static final String OS_ARCH_STR = "OS_ARCH=\"";

    public static List<Runtime> getRuntimes() {
        if(!RUNTIME_FOLDER.exists() && !RUNTIME_FOLDER.mkdirs()) {
            throw new RuntimeException("Failed to create runtime directory");
        }

        ArrayList<Runtime> runtimes = new ArrayList<>();
        File[] files = RUNTIME_FOLDER.listFiles();
        if(files != null) for(File f : files) {
            runtimes.add(read(f.getName()));
        }
        else throw new RuntimeException("The runtime directory does not exist");

        return runtimes;
    }

    public static String getExactJreName(int majorVersion) {
        List<Runtime> runtimes = getRuntimes();
        for(Runtime r : runtimes)
            if(r.javaVersion == majorVersion)return r.name;

        return null;
    }

    public static String getNearestJreName(int majorVersion) {
        List<Runtime> runtimes = getRuntimes();
        MathUtils.RankedValue<Runtime> nearestRankedRuntime = MathUtils.findNearestPositive(majorVersion, runtimes, (runtime)->runtime.javaVersion);
        if(nearestRankedRuntime == null) return null;
        Runtime nearestRuntime = nearestRankedRuntime.value;
        if(nearestRuntime == null) return null;
        return nearestRuntime.name;
    }

    public static void installRuntimeNamed(String nativeLibDir, InputStream runtimeInputStream, String name) throws IOException {
        File dest = new File(RUNTIME_FOLDER,"/"+name);
        if(dest.exists()) FileUtils.deleteDirectory(dest);
        try {
            uncompressTarXZ(runtimeInputStream,dest);
            runtimeInputStream.close();
            flattenSingleRootDirIfNeeded(dest);
            unpack200(nativeLibDir,RUNTIME_FOLDER + "/" + name);
            forceReread(name);
            if(read(name).versionString == null) throw new IOException("Runtime " + name + " is invalid after unpacking (missing 'release' or libjvm.so)");
        } catch (IOException | RuntimeException e) {
            cleanupFailedInstall(dest, name);
            throw e;
        } finally {
            ProgressLayout.clearProgress(ProgressLayout.UNPACK_RUNTIME);
        }
    }

    public static void postPrepare(String name) throws IOException {
        File dest = new File(RUNTIME_FOLDER,"/" + name);
        if(!dest.exists()) return;
        Runtime runtime = forceReread(name);
        String libFolder = "lib";
        if(runtime.arch != null && new File(dest,libFolder + "/" + runtime.arch).exists()) libFolder = libFolder + "/" + runtime.arch;
        File ftIn = new File(dest, libFolder + "/libfreetype.so.6");
        File ftOut = new File(dest, libFolder + "/libfreetype.so");
        if (ftIn.exists() && (!ftOut.exists() || ftIn.length() != ftOut.length())) {
            if(!ftIn.renameTo(ftOut)) throw new IOException("Failed to rename freetype");
        }

        // Refresh libraries
        copyDummyNativeLib("libawt_xawt.so", dest, libFolder);
    }

    public static void installRuntimeNamedBinpack(InputStream universalFileInputStream, InputStream platformBinsInputStream, String name, String binpackVersion) throws IOException {
        File dest = new File(RUNTIME_FOLDER,"/"+name);
        if(dest.exists()) FileUtils.deleteDirectory(dest);
        try {
            installRuntimeNamedNoRemove(universalFileInputStream,dest);
            installRuntimeNamedNoRemove(platformBinsInputStream,dest);
            flattenSingleRootDirIfNeeded(dest);

            unpack200(NATIVE_LIB_DIR,RUNTIME_FOLDER + "/" + name);

            // Kiểm tra runtime hợp lệ TRƯỚC khi ghi pojav_version,
            // để runtime hỏng không bị đánh dấu là "đã cài xong".
            forceReread(name);
            if(read(name).versionString == null) {
                throw new IOException("Runtime " + name + " is invalid after unpacking (missing 'release' or libjvm.so)");
            }

            File binpack_verfile = new File(RUNTIME_FOLDER,"/"+name+"/pojav_version");
            try (FileOutputStream fos = new FileOutputStream(binpack_verfile)) {
                fos.write(binpackVersion.getBytes());
            }
            forceReread(name);
        } catch (IOException | RuntimeException e) {
            cleanupFailedInstall(dest, name);
            throw e;
        } finally {
            ProgressLayout.clearProgress(ProgressLayout.UNPACK_RUNTIME);
        }
    }

    /** Xóa thư mục runtime cài dở để không còn runtime "hỏng" nằm trong danh sách. */
    private static void cleanupFailedInstall(File dest, String name) {
        Log.e("MultiRT", "Install of runtime " + name + " failed, removing partial files");
        sCache.remove(name);
        try {
            if(dest.exists()) FileUtils.deleteDirectory(dest);
        } catch (IOException ioe) {
            Log.e("MultiRT", "Could not remove partial runtime " + name, ioe);
        }
    }

    private static void flattenSingleRootDirIfNeeded(File dest) throws IOException {
        if (new File(dest, "release").exists()) return;
        File[] children = dest.listFiles();
        if (children == null) return;
        File candidateDir = null;
        for (File c : children) {
            if (c.isDirectory() && new File(c, "release").exists()) {
                candidateDir = c;
                break;
            }
        }
        if (candidateDir == null) return;
        File[] innerFiles = candidateDir.listFiles();
        if (innerFiles != null) {
            for (File f : innerFiles) {
                File target = new File(dest, f.getName());
                if (target.exists()) FileUtils.forceDelete(target);
                FileUtils.moveToDirectory(f, dest, false);
            }
        }
        FileUtils.deleteDirectory(candidateDir);
    }

    private static boolean hasValidJvmBinary(File runtimeDir) {
        if (!runtimeDir.isDirectory()) return false;
        Collection<File> soFiles = listFiles(runtimeDir, new String[]{"so"}, true);
        for (File f : soFiles) {
            if ("libjvm.so".equals(f.getName()) && f.length() > 1024) {
                return true;
            }
        }
        return false;
    }

    public static String readInternalRuntimeVersion(String name) {
        File runtimeDir = new File(RUNTIME_FOLDER, name);
        File versionFile = new File(runtimeDir, "pojav_version");
        try {
            if (versionFile.exists() && hasValidJvmBinary(runtimeDir)) {
                return Tools.read(versionFile.getAbsolutePath());
            } else {
                return null;
            }
        } catch (IOException e) {
            e.printStackTrace();
            return null;
        }
    }

    public static void removeRuntimeNamed(String name) throws IOException {
        File dest = new File(RUNTIME_FOLDER,"/"+name);
        if(dest.exists()) {
            FileUtils.deleteDirectory(dest);
            sCache.remove(name);
        }
    }

    public static File getRuntimeHome(String name) {
        File dest = new File(RUNTIME_FOLDER, name);
        Log.i("MiltiRTUitls", "Dest exists? "+dest.exists());
        if((!dest.exists()) || MultiRTUtils.forceReread(name).versionString == null) throw new RuntimeException("Selected runtime is broken!");
        return dest;
    }

    public static Runtime forceReread(String name) {
        sCache.remove(name);
        return read(name);
    }

    public static Runtime read(String name) {
        Runtime returnRuntime = sCache.get(name);
        if(returnRuntime != null) return returnRuntime;
        File runtimeDir = new File(RUNTIME_FOLDER, name);
        File release = new File(runtimeDir, "release");
        if(!release.exists() || !hasValidJvmBinary(runtimeDir)) {
            return new Runtime(name);
        }
        try {
            String content = Tools.read(release.getAbsolutePath());
            String javaVersion = Tools.extractUntilCharacter(content, JAVA_VERSION_STR, '"');
            String osArch = Tools.extractUntilCharacter(content, OS_ARCH_STR, '"');
            if(javaVersion != null && osArch != null) {
                String[] javaVersionSplit = javaVersion.split("\\.");
                int javaVersionInt;
                if (javaVersionSplit[0].equals("1")) {
                    javaVersionInt = Integer.parseInt(javaVersionSplit[1]);
                } else {
                    javaVersionInt = Integer.parseInt(javaVersionSplit[0]);
                }
                returnRuntime = new Runtime(name, javaVersion, osArch, javaVersionInt);
            }else{
                returnRuntime =  new Runtime(name);
            }
        }catch(IOException e) {
            returnRuntime =  new Runtime(name);
        }
        sCache.put(name, returnRuntime);
        return returnRuntime;
    }

    /**
     * Unpacks all .pack files into .jar Serves only for java 8, as java 9 brought project jigsaw
     * @param nativeLibraryDir The native lib path, required to execute the unpack200 binary
     * @param runtimePath The path to the runtime to walk into
     */
    private static void unpack200(String nativeLibraryDir, String runtimePath) {

        File basePath = new File(runtimePath);
        Collection<File> files = listFiles(basePath, new String[]{"pack"}, true);

        File workdir = new File(nativeLibraryDir);

        ProcessBuilder processBuilder = new ProcessBuilder().directory(workdir);
        for(File jarFile : files){
            try{
                Process process = processBuilder.command("./libunpack200.so", "-r", jarFile.getAbsolutePath(), jarFile.getAbsolutePath().replace(".pack", "")).start();
                process.waitFor();
            }catch (InterruptedException | IOException e) {
                Log.e("MULTIRT", "Failed to unpack the runtime !");
            }
        }
    }

    @SuppressWarnings("SameParameterValue")
    private static void copyDummyNativeLib(String name, File dest, String libFolder) throws IOException {
        File targetDir = new File(dest, libFolder);
        if (!targetDir.exists() && !targetDir.mkdirs()) return;
        File fileLib = new File(targetDir, name);
        FileInputStream is = new FileInputStream(new File(NATIVE_LIB_DIR, name));
        FileOutputStream os = new FileOutputStream(fileLib);
        IOUtils.copy(is, os);
        is.close();
        os.close();
    }

    private static void installRuntimeNamedNoRemove(InputStream runtimeInputStream, File dest) throws IOException {
        uncompressTarXZ(runtimeInputStream,dest);
        runtimeInputStream.close();
    }

    private static void uncompressTarXZ(final InputStream tarFileInputStream, final File dest) throws IOException {
        net.kdt.pojavlaunch.utils.FileUtils.ensureDirectory(dest);

        byte[] buffer = new byte[8192];
        List<Map.Entry<File, String>> pendingSymlinks = new ArrayList<>();
        TarArchiveInputStream tarIn = new TarArchiveInputStream(openCompressed(tarFileInputStream));
        TarArchiveEntry tarEntry = tarIn.getNextTarEntry();
        while (tarEntry != null) {

            final String tarEntryName = tarEntry.getName();
            ProgressLayout.setProgress(ProgressLayout.UNPACK_RUNTIME, 100, R.string.global_unpacking, tarEntryName);

            File destPath = new File(dest, tarEntryName);
            net.kdt.pojavlaunch.utils.FileUtils.ensureParentDirectory(destPath);
            if (tarEntry.isSymbolicLink()) {
                pendingSymlinks.add(new java.util.AbstractMap.SimpleEntry<>(destPath, tarEntry.getLinkName()));
            } else if (tarEntry.isDirectory()) {
                net.kdt.pojavlaunch.utils.FileUtils.ensureDirectory(destPath);
            } else if (!destPath.exists() || destPath.length() != tarEntry.getSize()) {
                FileOutputStream os = new FileOutputStream(destPath);
                IOUtils.copyLarge(tarIn, os, buffer);
                os.close();
            }
            tarEntry = tarIn.getNextTarEntry();
        }
        tarIn.close();

        // Resolve symbolic links AFTER all files are extracted
        for (int pass = 0; pass < 2; pass++) {
            for (Map.Entry<File, String> linkEntry : pendingSymlinks) {
                File linkFile = linkEntry.getKey();
                if (linkFile.exists()) continue;
                String linkTarget = linkEntry.getValue();
                try {
                    Os.symlink(linkTarget, linkFile.getAbsolutePath());
                } catch (Throwable ignored) {}
                if (!linkFile.exists()) {
                    File resolvedTarget = new File(linkFile.getParentFile(), linkTarget);
                    try {
                        resolvedTarget = resolvedTarget.getCanonicalFile();
                        if (resolvedTarget.isFile()) {
                            FileUtils.copyFile(resolvedTarget, linkFile);
                        }
                    } catch (Throwable e) {
                        Log.w("MultiRT", "Could not materialize symlink " + linkFile + " -> " + linkTarget + ": " + e);
                    }
                }
            }
        }
    }

    /**
     * Tự nhận dạng định dạng nén bằng magic bytes (không tin vào đuôi tên file).
     * File bin-*.tar.xz trong APK đôi khi thực chất là .tar.gz -> XZ sẽ ném
     * "Input is not in the XZ format" và runtime bị bỏ dở (hiện "Java đã gặp xung đột").
     */
    private static InputStream openCompressed(InputStream raw) throws IOException {
        BufferedInputStream in = new BufferedInputStream(raw, 64 * 1024);
        in.mark(8);
        byte[] magic = new byte[6];
        int read = 0;
        while (read < magic.length) {
            int r = in.read(magic, read, magic.length - read);
            if (r < 0) break;
            read += r;
        }
        in.reset();
        if (read >= 6 && (magic[0] & 0xFF) == 0xFD && magic[1] == '7' && magic[2] == 'z'
                && magic[3] == 'X' && magic[4] == 'Z' && magic[5] == 0x00) {
            return new XZCompressorInputStream(in);
        }
        if (read >= 2 && (magic[0] & 0xFF) == 0x1F && (magic[1] & 0xFF) == 0x8B) {
            return new GZIPInputStream(in, 64 * 1024);
        }
        throw new IOException("Unsupported runtime archive format (neither XZ nor GZIP)");
    }
}
