#!/bin/sh
# Keep bundled AppImage libraries out of the system Chromium process.
unset LD_LIBRARY_PATH GTK_PATH XDG_DATA_DIRS
exec /usr/bin/chromium "$@"
