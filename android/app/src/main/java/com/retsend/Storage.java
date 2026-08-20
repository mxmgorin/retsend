package com.retsend;

import android.content.Context;
import android.content.pm.PackageManager;
import android.os.Build;
import android.os.Environment;

/** Whether the app may use plain file I/O across the card. */
final class Storage {

    private Storage() {}

    static boolean hasAllFilesAccess(Context context) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            return Environment.isExternalStorageManager();
        }
        // Up to API 29 the legacy runtime permission is the same thing, given
        // android:requestLegacyExternalStorage in the manifest.
        return context.checkSelfPermission(android.Manifest.permission.WRITE_EXTERNAL_STORAGE)
                == PackageManager.PERMISSION_GRANTED;
    }
}
