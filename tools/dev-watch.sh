#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
command -v inotifywait >/dev/null || { printf 'Install inotify-tools to use development watch mode.\n' >&2; exit 1; }
command -v cargo >/dev/null
command -v flock >/dev/null
mkdir -p target

exec 9>target/visus-dev.lock
flock -n 9 || { printf 'A Visus development watcher is already running.\n' >&2; exit 1; }
app_pid=""

stop_app() {
    if [[ -n "$app_pid" ]]; then
        kill -TERM "$app_pid" 2>/dev/null || true
        wait "$app_pid" 2>/dev/null || true
        app_pid=""
    fi
}

cleanup() {
    stop_app
    if [[ -n "${watch_pid:-}" ]]; then
        kill -TERM "$watch_pid" 2>/dev/null || true
        wait "$watch_pid" 2>/dev/null || true
    fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

rebuild() {
    printf '\n[visus-dev] Building saved changes...\n'
    if cargo build -p visus-app --features dev --profile dev-fast; then
        stop_app
        target/dev-fast/visus-app 9>&- &
        app_pid=$!
        printf '[visus-dev] Started PID %s. Watching for saves.\n' "$app_pid"
    else
        printf '[visus-dev] Build failed; keeping the last working instance.\n' >&2
    fi
}

coproc FILE_EVENTS {
    exec inotifywait --monitor --recursive --quiet \
        --event close_write,moved_to,moved_from,create,delete \
        --format '%w%f' crates Cargo.toml Cargo.lock 9>&-
}
watch_pid=$FILE_EVENTS_PID
exec {events_fd}<&"${FILE_EVENTS[0]}"
rebuild
while IFS= read -r -u "$events_fd" changed; do
    while IFS= read -r -t 0.25 -u "$events_fd" changed; do :; done
    rebuild
done