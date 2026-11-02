#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
for executable in cargo magick ffmpeg; do
    command -v "$executable" >/dev/null
done
cargo build --profile dev-fast -p visus-app
binary="$PWD/target/dev-fast/visus-app"
capture_dir="$(mktemp -d)"
trap 'rm -rf "$capture_dir"' EXIT

capture() {
    env -u XDG_RUNTIME_DIR VISUS_FAST=1 VISUS_ANIM_T=0 "$binary" --snapshot "$@"
}

for state in idle music; do
    if [[ "$state" == idle ]]; then
        VISUS_PLAYING=0 capture closed "$capture_dir/pill_$state.png" unplugged:72
    else
        VISUS_ART=1,2,3 VISUS_PLAYING=1 capture closed "$capture_dir/pill_$state.png" charging:76
    fi
    magick "$capture_dir/pill_$state.png" -trim +repage -quality 92 "docs/assets/shots/pill_$state.webp"
done
VISUS_PLAYING=0 capture download "$capture_dir/download.png"
magick "$capture_dir/download.png" -trim +repage -quality 92 docs/assets/shots/pill_download.webp
for style in pills bars card compact; do
    VISUS_HUD="$style" capture game "$capture_dir/hud_$style.png"
    magick "$capture_dir/hud_$style.png" -trim +repage -quality 92 "docs/assets/shots/hud_$style.webp"
done
cp docs/assets/shots/hud_pills.webp docs/assets/shots/pill_game.webp

for page in timer notes clipboard calendar windows palette; do
    capture "$page" "$capture_dir/$page.png"
    magick "$capture_dir/$page.png" -trim +repage -resize 1004x -quality 92 "docs/assets/shots/$page.webp"
done
for style in classic flip led; do
    VISUS_ART=1,2,3 VISUS_CLOCK="$style" capture music "$capture_dir/open_$style.png"
    magick "$capture_dir/open_$style.png" -trim +repage -resize 1004x -quality 92 "docs/assets/shots/open_$style.webp"
done
for section in Clock Appearance Mascot Gaming Integrations Behavior Alerts; do
    capture settings "$capture_dir/settings_$section.png" "$section"
    magick "$capture_dir/settings_$section.png" -trim +repage -resize 888x -quality 94 "docs/assets/shots/settings_${section,,}.webp"
done
cp docs/assets/shots/settings_clock.webp docs/assets/shots/settings.webp
for style in attached floating soft; do
    VISUS_PILL="$style" VISUS_PLAYING=0 capture closed "$capture_dir/style_$style.png" unplugged:72
    magick "$capture_dir/style_$style.png" -background '#35383b' -alpha remove -crop 720x160+600+0 +repage -resize 720x160 -quality 94 "docs/assets/shots/style_$style.webp"
done
for style in eye sauron cat robot; do
    VISUS_FRAMES=24 VISUS_STEP_MS=100 capture "$style" "$capture_dir/mascot_$style.png" 52
    ffmpeg -y -loglevel error -threads 1 -filter_complex_threads 1 -framerate 10 -i "$capture_dir/mascot_${style}_%03d.png" \
        -filter_complex 'crop=320:80:800:0,scale=560:140:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse=dither=bayer' \
        -loop 0 "docs/assets/gifs/$style.gif"
done
"$binary" --settings-preview docs/assets/shots/settings_sidebar_clock.png Clock
"$binary" --settings-morph-preview "$capture_dir/morph"
ffmpeg -y -loglevel error -threads 1 -filter_complex_threads 1 -framerate 60 -i "$capture_dir/morph-%03d.png" \
    -filter_complex 'fps=30,scale=720:-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse=dither=bayer' \
    -loop 0 docs/assets/gifs/settings_morph.gif

for frame in $(seq 0 29); do
    printf -v filename '%03d' "$frame"
    VISUS_ART=1,2,3 VISUS_TICKS="$((frame * 2))" capture music "$capture_dir/music_$filename.png"
done
ffmpeg -y -loglevel error -threads 1 -filter_complex_threads 1 -framerate 30 -i "$capture_dir/music_%03d.png" \
    -filter_complex 'crop=1544:520:188:0,scale=1000:-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse=dither=bayer' \
    -loop 0 docs/assets/music.gif
magick identify -format '%f %wx%h\n' docs/assets/shots/*.webp