#!/bin/bash
# HorizonOS Power Menu using wofi
# Provides shutdown, reboot, logout, suspend, and lock options

# Define options
options="Lock Screen\nLogout\nSuspend\nReboot\nShutdown\nCancel"

# Show menu and get selection
selected=$(echo -e "$options" | wofi --dmenu --prompt="Power Menu" --cache-file=/dev/null)

# Execute based on selection
case "$selected" in
    "Lock Screen")
        if command -v swaylock &> /dev/null; then
            swaylock
        else
            notify-send "Error" "swaylock not found"
        fi
        ;;
    "Logout")
        # For Hyprland
        if pgrep -x "Hyprland" > /dev/null; then
            hyprctl dispatch exit
        else
            # Generic logout
            loginctl terminate-session $XDG_SESSION_ID
        fi
        ;;
    "Suspend")
        systemctl suspend
        ;;
    "Reboot")
        # Confirm reboot
        confirm=$(echo -e "Yes\nNo" | wofi --dmenu --prompt="Reboot system?" --cache-file=/dev/null)
        if [ "$confirm" = "Yes" ]; then
            systemctl reboot
        fi
        ;;
    "Shutdown")
        # Confirm shutdown
        confirm=$(echo -e "Yes\nNo" | wofi --dmenu --prompt="Shutdown system?" --cache-file=/dev/null)
        if [ "$confirm" = "Yes" ]; then
            systemctl poweroff
        fi
        ;;
    "Cancel"|"")
        exit 0
        ;;
    *)
        notify-send "Power Menu" "Invalid option selected"
        ;;
esac