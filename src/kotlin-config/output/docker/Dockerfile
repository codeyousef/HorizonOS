# HorizonOS Configuration Container
# Generated from Kotlin DSL

FROM archlinux:latest

# Install base packages
RUN pacman -Syu --noconfirm
RUN pacman -S --needed --noconfirm base-devel git

# Copy configuration files
COPY scripts/ /opt/horizonos/scripts/
COPY json/ /opt/horizonos/config/

# Set working directory
WORKDIR /opt/horizonos

# Run configuration
CMD ["./scripts/deploy.sh"]
