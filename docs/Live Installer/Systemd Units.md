```
# /etc/systemd/system/horizonos-update.service
[Unit]
Description=HorizonOS Automatic Update Service
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/horizonos-autoupdate timer
StandardOutput=journal
StandardError=journal
# Run as root to manage OSTree
User=root
# Restart on failure with delay
Restart=on-failure
RestartSec=1h

[Install]
WantedBy=multi-user.target

---

# /etc/systemd/system/horizonos-update.timer
[Unit]
Description=HorizonOS Automatic Update Timer
After=network-online.target
Wants=network-online.target

[Timer]
# Run daily at 2 AM
OnCalendar=daily
OnCalendar=*-*-* 02:00:00
# Run on boot if missed
Persistent=true
# Randomize by up to 1 hour to avoid thundering herd
RandomizedDelaySec=1h

[Install]
WantedBy=timers.target

---

# /etc/systemd/system/horizonos-update-notify.service
[Unit]
Description=HorizonOS Update Notification Service
After=graphical-session.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/horizonos-update-notify
# Run as the logged-in user
User=%i
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
```