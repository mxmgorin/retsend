package com.retsend;

import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.content.SharedPreferences;
import android.net.Uri;
import android.os.Build;
import android.provider.Settings;
import android.util.Log;

/**
 * Asks for all-files access, then starts {@link RetsendActivity}.
 *
 * <p>It is a separate activity because the save folder and browser roots are
 * derived from the granted permissions and read once, before SDL starts: asking
 * from inside the SDL activity would settle those paths a launch too early, and
 * the save folder is persisted to config.toml on first run.
 */
public class RetsendLauncherActivity extends Activity {

    private static final String TAG = "retsend";
    private static final String PREFS = "retsend";
    private static final String KEY_ASKED = "storage_asked";

    /** Set while the grant UI is up; the return trip runs onResume again. */
    private boolean asking;

    @Override
    protected void onResume() {
        super.onResume();

        if (!asking && !Storage.hasAllFilesAccess(this) && !alreadyAsked()) {
            markAsked();
            if (requestAllFilesAccess()) {
                asking = true;
                return;
            }
        }

        // Whatever the answer: denied only costs the shared folders.
        startActivity(new Intent(this, RetsendActivity.class));
        finish();
    }

    /** Whether any grant UI came up — false means carry on without it. */
    private boolean requestAllFilesAccess() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) {
            requestPermissions(
                    new String[] {android.Manifest.permission.WRITE_EXTERNAL_STORAGE}, 1);
            return true;
        }
        Intent perApp = new Intent(
                Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION,
                Uri.parse("package:" + getPackageName()));
        Intent list = new Intent(Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION);
        for (Intent intent : new Intent[] {perApp, list}) {
            try {
                startActivity(intent);
                return true;
            } catch (ActivityNotFoundException e) {
                Log.w(TAG, "no screen for " + intent.getAction());
            }
        }
        return false;
    }

    private boolean alreadyAsked() {
        return prefs().getBoolean(KEY_ASKED, false);
    }

    private void markAsked() {
        prefs().edit().putBoolean(KEY_ASKED, true).apply();
    }

    private SharedPreferences prefs() {
        return getSharedPreferences(PREFS, MODE_PRIVATE);
    }
}
