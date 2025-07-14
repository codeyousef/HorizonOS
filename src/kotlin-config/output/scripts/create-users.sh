#!/bin/bash
# HorizonOS User Management Script

# Create user: admin
useradd -m -u 1000 -s /usr/bin/fish -G wheel,users admin

