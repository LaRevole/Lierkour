#!/bin/sh
# Переносимый запуск: база и обложки лежат в ./data рядом с бинарником,
# а не в ~/.local/share/MyFilmLibrary.
#
# Использование:
#   ./run-portable.sh
#
# Скрипт и бинарник my-film-library должны лежать в одной папке.

set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

MY_FILM_LIBRARY_DATA_DIR="$here/data"
export MY_FILM_LIBRARY_DATA_DIR

exec "$here/my-film-library" "$@"
