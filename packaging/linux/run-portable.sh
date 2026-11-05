#!/bin/sh
# Переносимый запуск: база, обложки и скриншоты лежат в ./data рядом с бинарником,
# а не в ~/.local/share/com.lierkour.desktop.
#
# Использование:
#   ./run-portable.sh
#
# Скрипт и бинарник lierkour-app должны лежать в одной папке.

set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

LIERKOUR_DATA_DIR="$here/data"
export LIERKOUR_DATA_DIR
MY_FILM_LIBRARY_DATA_DIR="$here/data"
export MY_FILM_LIBRARY_DATA_DIR

# Опция для устранения возможных сбоев буферов WebKitGTK под Wayland:
if [ -n "${DISABLE_DMABUF:-}" ]; then
  export WEBKIT_DISABLE_DMABUF_RENDERER=1
fi

if [ -f "$here/lierkour-app" ]; then
  exec "$here/lierkour-app" "$@"
elif [ -f "$here/myfilmlibrary-app" ]; then
  exec "$here/myfilmlibrary-app" "$@"
else
  echo "Error: executable lierkour-app not found in $here" >&2
  exit 1
fi
