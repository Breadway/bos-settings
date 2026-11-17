#!/bin/bash
# Captures the shipping app and the prototype at the three parity widths on the
# headless sway output. Run from slint-prototype/.
#
# The shipping app's `--screenshot --width` only crops a 1920 render, so the
# narrower references come from a real window resized through sway.
set -u
export SWAYSOCK=$(ls /run/user/1000/sway-ipc.* | head -1)
export XDG_RUNTIME_DIR=/tmp/opencode/rt-ship
export WAYLAND_DISPLAY=wayland-2
mkdir -p /tmp/opencode/ref3 /tmp/opencode/shot3
VIEWS="bluetooth defaults desktop displays memory network power sound updates"
for w in 1920 960 860; do
  swaymsg output HEADLESS-1 resolution 1920x1200 >/dev/null
  for v in $VIEWS; do
    ref=/tmp/opencode/ref3/$v-$w.png
    if [ ! -s "$ref" ]; then
      if [ "$w" = 1920 ]; then
        env -u DBUS_SESSION_BUS_ADDRESS timeout 60 /usr/local/bin/bos-settings \
          --screenshot "$v" --output "$ref" --width "$w" --height 1200 \
          >/tmp/opencode/ref3/$v-$w.log 2>&1
      else
        env -u DBUS_SESSION_BUS_ADDRESS WEBKIT_DISABLE_COMPOSITING_MODE=1 \
          /usr/local/bin/bos-settings --page "$v" >/tmp/opencode/ref3/$v-$w.log 2>&1 &
        ref_pid=$!
        sleep 7
        swaymsg '[title="BOS Settings"] floating enable, resize set '"$w"' 1200, move position 0 0' >/dev/null
        sleep 2
        grim -g "0,0 ${w}x1200" "$ref"
        kill "$ref_pid" 2>/dev/null
        wait "$ref_pid" 2>/dev/null
      fi
      echo "ref $v $w exit=$? size=$(stat -c%s "$ref" 2>/dev/null)"
    fi
    ./target/debug/bos-settings-slint --page "$v" >/tmp/opencode/shot3/$v-$w.log 2>&1 &
    pid=$!
    sleep 2
    swaymsg "[title=\"BOS Settings\"] floating enable, resize set $w 1200, move position 0 0" >/dev/null
    sleep 2
    grim -g "0,0 ${w}x1200" /tmp/opencode/shot3/$v-$w.png
    kill "$pid" 2>/dev/null
    wait "$pid" 2>/dev/null
    echo "shot $v $w done"
  done
done
echo COMPLETE
