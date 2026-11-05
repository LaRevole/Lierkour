#!/bin/sh
# Переносимый запуск: база и обложки лежат в ./data рядом с бинарником,
# а не в ~/.local/share/com.myfilmlibrary.desktop.
#
# Использование:
#   ./run-portable.sh
#
# Скрипт и бинарник myfilmlibrary-app должны лежать в одной папке.

set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

MY_FILM_LIBRARY_DATA_DIR="$here/data"
export MY_FILM_LIBRARY_DATA_DIR

# Опция для устранения возможных сбоев буферов WebKitGTK под Wayland:
if [ -n "${DISABLE_DMABUF:-}" ]; then
  export WEBKIT_DISABLE_DMABUF_RENDERER=1
fi

if [ -f "$here/myfilmlibrary-app" ]; then
  exec "$here/myfilmlibrary-app" "$@"
elif [ -f "$here/my-film-library" ]; then
  exec "$here/my-film-library" "$@"
else
  echo "Error: executable myfilmlibrary-app not found in $here" >&2
  exit 1
fi
