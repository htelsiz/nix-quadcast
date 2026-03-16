#!/usr/bin/env bash
# Waybar custom module for Sliglight — shows current profile and connection status.
#
# Add to your Waybar config:
#   "custom/quadcast": {
#       "exec": "~/.config/waybar/scripts/waybar-example.sh",
#       "interval": 5,
#       "return-type": "json"
#   }

profile=$(dbus-send --session --print-reply --dest=org.sliglight.Daemon \
    /org/sliglight/Daemon org.freedesktop.DBus.Properties.Get \
    string:"org.sliglight.Daemon" string:"CurrentProfile" 2>/dev/null \
    | grep variant | sed 's/.*string "//;s/"//')

connected=$(dbus-send --session --print-reply --dest=org.sliglight.Daemon \
    /org/sliglight/Daemon org.freedesktop.DBus.Properties.Get \
    string:"org.sliglight.Daemon" string:"IsConnected" 2>/dev/null \
    | grep variant | sed 's/.*boolean //;s/ *//')

if [ "$connected" = "true" ]; then
    icon="🎤"
    class="connected"
else
    icon="🎤"
    class="disconnected"
fi

text="${icon} ${profile:-N/A}"
tooltip="QuadCast 2S: ${profile:-Unknown} (${connected:-offline})"

printf '{"text": "%s", "tooltip": "%s", "class": "%s"}\n' "$text" "$tooltip" "$class"
