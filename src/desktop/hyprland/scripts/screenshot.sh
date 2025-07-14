#!/bin/bash
# HorizonOS Hyprland Screenshot Tool - KDE Spectacle-like

# Create screenshots directory if it doesn't exist
SCREENSHOT_DIR="$HOME/Pictures/Screenshots"
mkdir -p "$SCREENSHOT_DIR"

# Get current date and time
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Function to show notification
notify() {
    notify-send -i spectacle "Screenshot" "$1" -t 3000
}

# Parse command line arguments
MODE="${1:-region}"
COPY_TO_CLIPBOARD="${2:-true}"

case "$MODE" in
    "full")
        # Full screen screenshot
        if [ "$COPY_TO_CLIPBOARD" = "true" ]; then
            grim - | wl-copy
            notify "Full screen screenshot copied to clipboard"
        else
            FILE="$SCREENSHOT_DIR/Screenshot_${TIMESTAMP}.png"
            grim "$FILE"
            notify "Screenshot saved to $FILE"
        fi
        ;;
    
    "region")
        # Region selection screenshot
        if [ "$COPY_TO_CLIPBOARD" = "true" ]; then
            grim -g "$(slurp -d -c '#3daee9' -b '#31363b44' -w 2)" - | wl-copy
            notify "Region screenshot copied to clipboard"
        else
            FILE="$SCREENSHOT_DIR/Screenshot_${TIMESTAMP}.png"
            grim -g "$(slurp -d -c '#3daee9' -b '#31363b44' -w 2)" "$FILE"
            notify "Screenshot saved to $FILE"
        fi
        ;;
    
    "window")
        # Active window screenshot
        # Get active window geometry
        WINDOW_GEOM=$(hyprctl activewindow -j | jq -r '.at[0],.at[1],.size[0],.size[1]' | xargs printf "%d,%d %dx%d")
        
        if [ "$COPY_TO_CLIPBOARD" = "true" ]; then
            grim -g "$WINDOW_GEOM" - | wl-copy
            notify "Window screenshot copied to clipboard"
        else
            FILE="$SCREENSHOT_DIR/Screenshot_${TIMESTAMP}.png"
            grim -g "$WINDOW_GEOM" "$FILE"
            notify "Screenshot saved to $FILE"
        fi
        ;;
    
    "delay")
        # Delayed screenshot (5 seconds)
        notify "Taking screenshot in 5 seconds..."
        sleep 5
        
        if [ "$COPY_TO_CLIPBOARD" = "true" ]; then
            grim - | wl-copy
            notify "Screenshot copied to clipboard"
        else
            FILE="$SCREENSHOT_DIR/Screenshot_${TIMESTAMP}.png"
            grim "$FILE"
            notify "Screenshot saved to $FILE"
        fi
        ;;
    
    *)
        echo "Usage: $0 [full|region|window|delay] [true|false]"
        echo "  First argument: screenshot mode"
        echo "  Second argument: copy to clipboard (default: true)"
        exit 1
        ;;
esac

# Play screenshot sound if available
if [ -f "/usr/share/sounds/freedesktop/stereo/camera-shutter.oga" ]; then
    paplay /usr/share/sounds/freedesktop/stereo/camera-shutter.oga &
fi