#!/bin/bash
# HorizonOS OSTree Deployment Script

set -e

# System configuration
echo 'compiled-system' > /etc/hostname
ln -sf /usr/share/zoneinfo/UTC /etc/localtime
echo 'LANG=en_US.UTF-8' > /etc/locale.conf

# Run other configuration scripts
./install-packages.sh
./configure-services.sh
./create-users.sh
