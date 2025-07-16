#!/bin/bash
# System Configuration

echo 'Configuring system...'

# Set hostname
hostnamectl set-hostname 'horizonos-minimal'

# Set timezone
timedatectl set-timezone 'UTC'

# Set locale
echo 'LANG=en_US.UTF-8' > /etc/locale.conf
locale-gen

echo 'System configuration completed.'
