#!/usr/bin/env bash
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
browser="$(command -v google-chrome || command -v chromium || command -v chromium-browser)" || {
  printf '%s\n' "O teste de PDF exige Chrome ou Chromium no PATH." >&2
  exit 1
}

app_dir="$(mktemp -d)"
trap 'rm -rf -- "$app_dir"' EXIT
mkdir -p "$app_dir/usr/lib/gtk-3.0" "$app_dir/usr/share"

# Check the environment received by the browser, then generate a real PDF.
cat > "$app_dir/check-browser" <<'SH'
#!/bin/sh
case ":${LD_LIBRARY_PATH-}:${GTK_PATH-}:${XDG_DATA_DIRS-}:" in
  *":$APPDIR/"*|*":$APPDIR:"*)
    printf '%s\n' "O Chromium recebeu caminhos internos do AppImage." >&2
    exit 1
    ;;
esac
exec "$OPETS_PDF_TEST_BROWSER" "$@"
SH
chmod +x "$app_dir/check-browser"

cd "$repo/src-tauri"
APPDIR="$app_dir" \
  LD_LIBRARY_PATH="$app_dir/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
  GTK_PATH="$app_dir/usr/lib/gtk-3.0${GTK_PATH:+:$GTK_PATH}" \
  XDG_DATA_DIRS="$app_dir/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}" \
  CHROME_BIN="$app_dir/check-browser" OPETS_PDF_TEST_BROWSER="$browser" \
  cargo test --locked --lib pdf_service::tests::renders_html_to_a_real_pdf \
    -- --exact --ignored --nocapture
