#!/bin/bash
set -ev

# This expects to be run inside `xvfb-run`
fluxbox > /dev/null 2>&1 &
sleep 5

mkdir -p e2e-tests/screenshots
ffmpeg -f x11grab -video_size 1280x1024 -i "$DISPLAY" \
    -codec:v libx264 -r 12 -pix_fmt yuv420p \
    e2e-tests/screenshots/recording.mp4 > /dev/null 2>&1 &
FFMPEG_PID=$!
trap "kill $FFMPEG_PID" EXIT

devbox run test-e2e