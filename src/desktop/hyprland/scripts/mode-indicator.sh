#!/bin/bash
# HorizonOS Desktop Mode Indicator
# Shows current desktop personality mode

MODE="${1:-kde}"
INDICATOR_FILE="/tmp/horizonos-desktop-mode"

# Write current mode
echo "$MODE" > "$INDICATOR_FILE"

# Function to get mode name
get_mode_name() {
    case "$1" in
        "kde") echo "KDE Plasma Style" ;;
        "gnome") echo "GNOME Style" ;;
        "mac") echo "macOS Style" ;;
        "win11") echo "Windows 11 Style" ;;
        "i3") echo "i3 Tiling Style" ;;
        *) echo "Custom Style" ;;
    esac
}

# Show notification
if command -v notify-send &> /dev/null; then
    MODE_NAME=$(get_mode_name "$MODE")
    notify-send -i preferences-desktop-theme \
        "HorizonOS Desktop Mode" \
        "Now using: $MODE_NAME" \
        -t 5000
fi

# Export for other scripts
export HORIZONOS_DESKTOP_MODE="$MODE"