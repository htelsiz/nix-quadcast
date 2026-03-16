#!/usr/bin/env bash
# i3blocks script for Sliglight — shows current profile.
#
# Add to your i3blocks config:
#   [quadcast]
#   command=~/.config/i3blocks/i3blocks-example.sh
#   interval=5

profile=$(dbus-send --session --print-reply --dest=org.sliglight.Daemon \
    /org/sliglight/Daemon org.freedesktop.DBus.Properties.Get \
    string:"org.sliglight.Daemon" string:"CurrentProfile" 2>/dev/null \
    | grep variant | sed 's/.*string "//;s/"//')

connected=$(dbus-send --session --print-reply --dest=org.sliglight.Daemon \
    /org/sliglight/Daemon org.freedesktop.DBus.Properties.Get \
    string:"org.sliglight.Daemon" string:"IsConnected" 2>/dev/null \
    | grep variant | sed 's/.*boolean //;s/ *//')

if [ "$connected" = "true" ]; then
    echo "QC: ${profile:-N/A}"
else
    echo "QC: offline"
fi
