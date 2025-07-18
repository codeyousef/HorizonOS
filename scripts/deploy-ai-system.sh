#!/bin/bash
# HorizonOS AI System Deployment Script
# This script deploys the complete AI integration system

set -euo pipefail

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo -e "${2:-}[$(date '+%Y-%m-%d %H:%M:%S')] $1${NC}"
}

# Error handler
error_exit() {
    log "Error: $1" "$RED"
    exit 1
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        error_exit "This script must be run as root"
    fi
}

# Parse command line arguments
DEPLOY_MODE="production"
SKIP_BUILD=false
SKIP_TESTS=false
PULL_MODELS=true

while [[ $# -gt 0 ]]; do
    case $1 in
        --dev|--development)
            DEPLOY_MODE="development"
            shift
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --skip-tests)
            SKIP_TESTS=true
            shift
            ;;
        --no-models)
            PULL_MODELS=false
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --dev, --development  Deploy in development mode"
            echo "  --skip-build         Skip building from source"
            echo "  --skip-tests         Skip running tests"
            echo "  --no-models          Don't pull AI models"
            echo "  --help, -h           Show this help message"
            exit 0
            ;;
        *)
            error_exit "Unknown option: $1"
            ;;
    esac
done

log "Starting HorizonOS AI System deployment (mode: $DEPLOY_MODE)" "$BLUE"

# Step 1: Create system user
create_system_user() {
    log "Creating system user..." "$BLUE"
    
    if ! id "horizonos-ai" &>/dev/null; then
        useradd -r -s /bin/false -d /var/lib/horizonos-ai -m horizonos-ai
        log "Created horizonos-ai user" "$GREEN"
    else
        log "User horizonos-ai already exists" "$YELLOW"
    fi
    
    # Add user to necessary groups
    usermod -a -G docker,audio,video,input horizonos-ai || true
}

# Step 2: Build the project
build_project() {
    if [[ "$SKIP_BUILD" == "true" ]]; then
        log "Skipping build step" "$YELLOW"
        return
    fi
    
    log "Building HorizonOS AI components..." "$BLUE"
    
    cd "$PROJECT_ROOT"
    
    # Build Rust components
    log "Building Rust components..."
    cd src/desktop/graph-ai
    cargo build --release || error_exit "Failed to build Rust components"
    
    # Build Kotlin DSL
    log "Building Kotlin DSL..."
    cd "$PROJECT_ROOT/src/kotlin-config"
    ./gradlew build || error_exit "Failed to build Kotlin DSL"
    
    log "Build completed successfully" "$GREEN"
}

# Step 3: Run tests
run_tests() {
    if [[ "$SKIP_TESTS" == "true" ]]; then
        log "Skipping tests" "$YELLOW"
        return
    fi
    
    log "Running tests..." "$BLUE"
    
    cd "$PROJECT_ROOT"
    
    # Run Rust tests
    log "Running Rust tests..."
    cd src/desktop/graph-ai
    cargo test || error_exit "Rust tests failed"
    
    # Run integration tests if in development mode
    if [[ "$DEPLOY_MODE" == "development" ]]; then
        log "Running integration tests..."
        cd "$PROJECT_ROOT"
        cargo test --test '*' --features integration-tests || log "Some integration tests failed" "$YELLOW"
    fi
    
    log "Tests completed" "$GREEN"
}

# Step 4: Install binaries
install_binaries() {
    log "Installing binaries..." "$BLUE"
    
    # Install main service binary
    install -m 755 -o root -g root \
        "$PROJECT_ROOT/target/release/horizonos-ai-service" \
        /usr/bin/horizonos-ai-service
    
    # Install monitor binary
    install -m 755 -o root -g root \
        "$PROJECT_ROOT/target/release/horizonos-ai-monitor" \
        /usr/bin/horizonos-ai-monitor
    
    # Install agents binary
    install -m 755 -o root -g root \
        "$PROJECT_ROOT/target/release/horizonos-ai-agents" \
        /usr/bin/horizonos-ai-agents
    
    # Install check script
    install -m 755 -o root -g root \
        "$SCRIPT_DIR/systemd/horizonos-ai-check" \
        /usr/bin/horizonos-ai-check
    
    log "Binaries installed" "$GREEN"
}

# Step 5: Install systemd services
install_systemd_services() {
    log "Installing systemd services..." "$BLUE"
    
    # Copy service files
    cp "$SCRIPT_DIR/systemd/"*.service /etc/systemd/system/
    cp "$SCRIPT_DIR/systemd/"*.target /etc/systemd/system/
    
    # Set correct permissions
    chmod 644 /etc/systemd/system/horizonos-ai*.{service,target}
    
    # Reload systemd
    systemctl daemon-reload
    
    log "Systemd services installed" "$GREEN"
}

# Step 6: Deploy Docker containers
deploy_containers() {
    log "Deploying Docker containers..." "$BLUE"
    
    cd "$PROJECT_ROOT"
    
    if [[ "$DEPLOY_MODE" == "production" ]]; then
        # Production deployment
        docker-compose -f docker-compose.yml up -d
    else
        # Development deployment with all services
        docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d
    fi
    
    # Wait for containers to be ready
    log "Waiting for containers to be ready..."
    sleep 10
    
    # Check container health
    docker-compose ps
    
    log "Docker containers deployed" "$GREEN"
}

# Step 7: Initialize database
initialize_database() {
    log "Initializing database..." "$BLUE"
    
    # Wait for TimescaleDB to be ready
    local retries=30
    while ((retries > 0)); do
        if docker exec horizonos-timescaledb pg_isready -U postgres >/dev/null 2>&1; then
            break
        fi
        sleep 1
        ((retries--))
    done
    
    if ((retries == 0)); then
        error_exit "TimescaleDB failed to start"
    fi
    
    # Run initialization script
    docker exec -i horizonos-timescaledb psql -U postgres horizonos < "$SCRIPT_DIR/sql/init-timescaledb.sql" || \
        log "Database might already be initialized" "$YELLOW"
    
    log "Database initialized" "$GREEN"
}

# Step 8: Pull AI models
pull_ai_models() {
    if [[ "$PULL_MODELS" == "false" ]]; then
        log "Skipping AI model download" "$YELLOW"
        return
    fi
    
    log "Pulling AI models..." "$BLUE"
    
    # Default models for different hardware profiles
    local models=("llama3.2:latest" "codellama:7b" "mistral:7b-instruct")
    
    for model in "${models[@]}"; do
        log "Pulling $model..."
        docker exec horizonos-ollama ollama pull "$model" || \
            log "Failed to pull $model" "$YELLOW"
    done
    
    log "AI models pulled" "$GREEN"
}

# Step 9: Configure the system
configure_system() {
    log "Configuring system..." "$BLUE"
    
    # Create configuration directories
    mkdir -p /etc/horizonos/ai
    
    # Generate default configuration from Kotlin DSL
    if [[ -f "$PROJECT_ROOT/src/kotlin-config/examples/ai-settings.horizonos.kts" ]]; then
        cd "$PROJECT_ROOT/src/kotlin-config"
        ./gradlew run --args="examples/ai-settings.horizonos.kts" > /etc/horizonos/ai/config.toml || \
            log "Failed to generate config from DSL" "$YELLOW"
    fi
    
    # Set permissions
    chown -R horizonos-ai:horizonos-ai /etc/horizonos/ai
    chmod -R 640 /etc/horizonos/ai/*
    
    log "System configured" "$GREEN"
}

# Step 10: Start services
start_services() {
    log "Starting services..." "$BLUE"
    
    # Enable services
    systemctl enable horizonos-ai.target
    
    # Start services
    systemctl start horizonos-ai.target
    
    # Wait a moment
    sleep 5
    
    # Check status
    systemctl status horizonos-ai.service --no-pager || true
    systemctl status horizonos-ai-monitor.service --no-pager || true
    systemctl status horizonos-ai-agents.service --no-pager || true
    
    log "Services started" "$GREEN"
}

# Step 11: Verify deployment
verify_deployment() {
    log "Verifying deployment..." "$BLUE"
    
    local errors=0
    
    # Check if services are running
    if ! systemctl is-active --quiet horizonos-ai.service; then
        log "Main AI service is not running" "$RED"
        ((errors++))
    fi
    
    # Check if Ollama is accessible
    if ! curl -sf http://localhost:11434/api/tags >/dev/null; then
        log "Ollama is not accessible" "$RED"
        ((errors++))
    fi
    
    # Check if TimescaleDB is accessible
    if ! docker exec horizonos-timescaledb pg_isready -U postgres >/dev/null 2>&1; then
        log "TimescaleDB is not accessible" "$RED"
        ((errors++))
    fi
    
    if ((errors > 0)); then
        log "Deployment verification failed with $errors errors" "$RED"
        log "Check logs with: journalctl -u horizonos-ai.service" "$YELLOW"
    else
        log "Deployment verified successfully" "$GREEN"
    fi
}

# Main deployment flow
main() {
    check_root
    
    log "=== Phase 1: System Preparation ===" "$BLUE"
    create_system_user
    
    log "=== Phase 2: Build and Test ===" "$BLUE"
    build_project
    run_tests
    
    log "=== Phase 3: Installation ===" "$BLUE"
    install_binaries
    install_systemd_services
    
    log "=== Phase 4: Container Deployment ===" "$BLUE"
    deploy_containers
    initialize_database
    pull_ai_models
    
    log "=== Phase 5: Configuration ===" "$BLUE"
    configure_system
    
    log "=== Phase 6: Service Startup ===" "$BLUE"
    start_services
    
    log "=== Phase 7: Verification ===" "$BLUE"
    verify_deployment
    
    log "=== Deployment Complete ===" "$GREEN"
    log ""
    log "Next steps:" "$BLUE"
    log "1. Check service status: systemctl status horizonos-ai.target"
    log "2. View logs: journalctl -u horizonos-ai.service -f"
    log "3. Configure AI settings: /etc/horizonos/ai/config.toml"
    log "4. Access monitoring dashboard: http://localhost:3000 (admin/admin)"
}

# Run main function
main "$@"