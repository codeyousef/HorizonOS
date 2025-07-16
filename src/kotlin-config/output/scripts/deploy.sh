#!/bin/bash
# HorizonOS Deployment Script
# Generated from Kotlin DSL configuration

set -euo pipefail

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Starting HorizonOS deployment...${NC}"

# Run all configuration scripts
./system-config.sh
./package-manager.sh
./service-manager.sh
./user-manager.sh
./repository-config.sh

echo -e "${GREEN}Deployment completed successfully!${NC}"
