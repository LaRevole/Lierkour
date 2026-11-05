#!/usr/bin/env bash
set -euo pipefail

# Скрипт сборки Lierkour для Linux (CachyOS / Arch / Ubuntu / Fedora)
echo "=== Сборка Lierkour для Linux ==="

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$ROOT_DIR"

# 1. Проверка Node.js и npm
if ! command -v npm &> /dev/null; then
    echo "❌ Ошибка: npm не установлен."
    echo "На Arch/CachyOS установите: sudo pacman -S nodejs npm"
    echo "На Ubuntu/Debian установите: sudo apt install nodejs npm"
    exit 1
fi

# 2. Проверка Rust и Cargo
if ! command -v cargo &> /dev/null; then
    echo "❌ Ошибка: cargo не установлен."
    echo "Установите Rust через rustup: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# 3. Сборка фронтенда
echo "📦 Шаг 1/3: Сборка фронтенда..."
cd "$ROOT_DIR/frontend"
npm install
npm run build
cd "$ROOT_DIR"

# 4. Сборка бинарного файла приложения (Tauri 2 backend)
#    custom-protocol обязателен: без него release-бинарник пытается открыть
#    dev-сервер http://127.0.0.1:5173 и падает с Connection refused.
echo "🦀 Шаг 2/3: Компиляция Rust backend..."
cargo build --release --features custom-protocol \
    --manifest-path "$ROOT_DIR/src-tauri/Cargo.toml"

BINARY="$ROOT_DIR/target/release/lierkour-app"

if [ -f "$BINARY" ]; then
    echo "✅ Шаг 3/3: Бинарный файл успешно собран:"
    ls -lh "$BINARY"
    echo ""
    echo "Для запуска выполните:"
    echo "  $BINARY"
    echo "или для переносимого режима (хранение данных в ./data рядом):"
    echo "  $ROOT_DIR/packaging/linux/run-portable.sh"
    echo ""
    echo "Для работы под Wayland в KDE Plasma / CachyOS приложение готово из коробки!"
fi

