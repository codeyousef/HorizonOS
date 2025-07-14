#!/bin/bash
# HorizonOS Package Installation Script
# Generated from Kotlin DSL configuration

set -e

echo 'Installing packages...'
pacman -S --needed --noconfirm \
    base \
    linux \
    btrfs-progs \
    networkmanager

