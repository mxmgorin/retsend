package com.retsend;

import android.content.Context;
import android.net.wifi.WifiManager;
import android.os.Build;
import android.os.Bundle;
import android.os.Environment;
import android.system.ErrnoException;
import android.system.Os;
import android.util.Log;

import java.io.File;

import org.libsdl.app.SDLActivity;

/**
 * SDL entry activity. SDL loads the libraries named here, in order, and then
 * calls the {@code SDL_main} the Rust cdylib exports.
 */
public class RetsendActivity extends SDLActivity {

    private static final String TAG = "retsend";

    /** Held for the app's lifetime: without it the Wi-Fi driver drops the
     *  multicast announces discovery is built on. */
    private WifiManager.MulticastLock multicastLock;

    @Override
    protected String[] getLibraries() {
        // SDL2 first, then libretsend.so, whose SDL_main is the entry point.
        return new String[] {"SDL2", "retsend"};
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        // The Rust side reads all of these at startup, so they must be set
        // before super.onCreate() loads the native libraries and starts SDL.
        File data = getFilesDir();
        setEnv("RETSEND_DATA_DIR", data.getAbsolutePath());
        setEnv("RETSEND_PANIC_FILE", new File(data, "retsend-panic.log").getAbsolutePath());
        setEnv("RETSEND_SAVE_DIR", defaultSaveDir());
        setEnv("RETSEND_BROWSER_ROOTS", browserRoots());
        // Only seeds the alias; the Settings screen owns it from then on.
        setEnv("RETSEND_ALIAS", Build.MODEL);
        // Phone screens are dense enough that the UI would be unreadable at 1:1
        // pixels. ~1.0 (mdpi) .. ~3.5 (xxxhdpi), applied to egui's zoom factor.
        setEnv("RETSEND_SCALE", String.valueOf(getResources().getDisplayMetrics().density));

        acquireMulticastLock();

        super.onCreate(savedInstanceState);
    }

    @Override
    protected void onDestroy() {
        if (multicastLock != null && multicastLock.isHeld()) {
            multicastLock.release();
        }
        super.onDestroy();
    }

    /** The public Download folder when the card is writable, else our own
     *  external directory, which needs no permission but is invisible to file
     *  managers on Android 11+. */
    private String defaultSaveDir() {
        if (Storage.hasAllFilesAccess(this)) {
            File shared = Environment.getExternalStoragePublicDirectory(
                    Environment.DIRECTORY_DOWNLOADS);
            if (shared != null) {
                return shared.getAbsolutePath();
            }
        }
        File own = getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS);
        return (own != null ? own : getFilesDir()).getAbsolutePath();
    }

    /** File-browser roots, `:`-separated: every storage volume the app can
     *  reach. Their paths are per-install, so the Rust side can't guess them. */
    private String browserRoots() {
        StringBuilder roots = new StringBuilder();
        boolean all = Storage.hasAllFilesAccess(this);
        for (File dir : getExternalFilesDirs(null)) {
            if (dir == null) {
                continue;
            }
            String path = dir.getAbsolutePath();
            // /storage/<volume>/Android/data/<pkg>/files -> /storage/<volume>,
            // which is the whole card (or the removable one) when granted.
            int volume = path.indexOf("/Android/");
            if (all && volume > 0) {
                append(roots, path.substring(0, volume));
            }
            append(roots, path);
        }
        return roots.toString();
    }

    private static void append(StringBuilder roots, String path) {
        if (roots.length() > 0) {
            roots.append(':');
        }
        roots.append(path);
    }

    private void acquireMulticastLock() {
        WifiManager wifi = (WifiManager) getApplicationContext()
                .getSystemService(Context.WIFI_SERVICE);
        if (wifi == null) {
            return;
        }
        multicastLock = wifi.createMulticastLock(TAG);
        multicastLock.setReferenceCounted(false);
        multicastLock.acquire();
    }

    private void setEnv(String key, String value) {
        try {
            Os.setenv(key, value, true);
        } catch (ErrnoException e) {
            Log.w(TAG, "failed to set env " + key + ": " + e.getMessage());
        }
    }
}
