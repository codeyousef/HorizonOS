#!/bin/bash
# HorizonOS AI Development Environment Setup Script
# This script sets up a complete development environment for the AI system

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

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..." "$BLUE"
    
    local missing=()
    
    # Check for required commands
    local commands=("docker" "docker-compose" "cargo" "rustc" "java" "gradle" "jq" "curl")
    
    for cmd in "${commands[@]}"; do
        if ! command -v "$cmd" &> /dev/null; then
            missing+=("$cmd")
        fi
    done
    
    if [[ ${#missing[@]} -gt 0 ]]; then
        log "Missing required commands: ${missing[*]}" "$RED"
        log "Please install the missing dependencies and try again" "$YELLOW"
        exit 1
    fi
    
    # Check Docker daemon
    if ! docker info &> /dev/null; then
        error_exit "Docker daemon is not running. Please start Docker and try again."
    fi
    
    # Check Rust version
    local rust_version=$(rustc --version | cut -d' ' -f2)
    log "Found Rust version: $rust_version" "$GREEN"
    
    # Check Java version
    local java_version=$(java -version 2>&1 | head -n1 | cut -d'"' -f2)
    log "Found Java version: $java_version" "$GREEN"
    
    log "All prerequisites satisfied" "$GREEN"
}

# Setup Rust dependencies
setup_rust_deps() {
    log "Setting up Rust dependencies..." "$BLUE"
    
    cd "$PROJECT_ROOT/src/desktop/graph-ai"
    
    # Install additional Rust tools
    log "Installing Rust tools..."
    cargo install cargo-watch cargo-expand cargo-outdated || true
    
    # Download dependencies
    log "Downloading Rust dependencies..."
    cargo fetch
    
    log "Rust dependencies ready" "$GREEN"
}

# Setup Kotlin DSL
setup_kotlin_dsl() {
    log "Setting up Kotlin DSL..." "$BLUE"
    
    cd "$PROJECT_ROOT/src/kotlin-config"
    
    # Make gradlew executable
    chmod +x gradlew
    
    # Download dependencies
    log "Downloading Gradle dependencies..."
    ./gradlew dependencies --quiet || error_exit "Failed to download Gradle dependencies"
    
    # Build DSL
    log "Building Kotlin DSL..."
    ./gradlew build || error_exit "Failed to build Kotlin DSL"
    
    log "Kotlin DSL ready" "$GREEN"
}

# Start Docker containers
start_containers() {
    log "Starting Docker containers..." "$BLUE"
    
    cd "$PROJECT_ROOT"
    
    # Create docker-compose.dev.yml if it doesn't exist
    if [[ ! -f docker-compose.dev.yml ]]; then
        log "Creating development compose file..."
        cat > docker-compose.dev.yml <<'EOF'
version: '3.8'

# Development overrides for docker-compose.yml
services:
  ollama:
    environment:
      - OLLAMA_DEBUG=1
    volumes:
      - ./models:/root/.ollama/models
  
  timescaledb:
    ports:
      - "5432:5432"
  
  n8n:
    environment:
      - N8N_BASIC_AUTH_ACTIVE=false
  
  grafana:
    environment:
      - GF_AUTH_ANONYMOUS_ENABLED=true
      - GF_AUTH_ANONYMOUS_ORG_ROLE=Admin
  
  # Additional development services
  pgadmin:
    image: dpage/pgadmin4:latest
    container_name: horizonos-pgadmin
    environment:
      - PGADMIN_DEFAULT_EMAIL=admin@horizonos.local
      - PGADMIN_DEFAULT_PASSWORD=admin
    ports:
      - "5050:80"
    depends_on:
      - timescaledb
    networks:
      - horizonos-ai
EOF
    fi
    
    # Start containers
    log "Starting containers with docker-compose..."
    docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d
    
    # Wait for containers
    log "Waiting for containers to be ready..."
    sleep 15
    
    # Show container status
    docker-compose ps
    
    log "Docker containers started" "$GREEN"
}

# Initialize development database
init_dev_database() {
    log "Initializing development database..." "$BLUE"
    
    # Wait for TimescaleDB
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
    
    # Run init script
    docker exec -i horizonos-timescaledb psql -U postgres horizonos < "$SCRIPT_DIR/sql/init-timescaledb.sql" || \
        log "Database might already be initialized" "$YELLOW"
    
    # Create test data
    log "Creating test data..."
    docker exec -i horizonos-timescaledb psql -U postgres horizonos <<'EOF'
-- Insert test user actions
INSERT INTO user_actions (time, user_id, action_type, target, context, duration_ms, success)
SELECT 
    NOW() - (interval '1 hour' * generate_series(1, 100)),
    'dev-user',
    CASE (random() * 3)::int
        WHEN 0 THEN 'app_launch'
        WHEN 1 THEN 'file_open'
        ELSE 'web_visit'
    END,
    CASE (random() * 3)::int
        WHEN 0 THEN 'firefox'
        WHEN 1 THEN 'vscode'
        ELSE 'terminal'
    END,
    '{"dev": true}'::jsonb,
    (random() * 1000)::int,
    true
FROM generate_series(1, 100);

-- Refresh continuous aggregates
CALL refresh_continuous_aggregate('hourly_action_summary', NOW() - interval '1 day', NOW());
CALL refresh_continuous_aggregate('daily_pattern_summary', NOW() - interval '7 days', NOW());
EOF
    
    log "Development database initialized" "$GREEN"
}

# Pull development AI models
pull_dev_models() {
    log "Pulling development AI models..." "$BLUE"
    
    # Smaller models for development
    local dev_models=("llama3.2:latest" "tinyllama:latest")
    
    for model in "${dev_models[@]}"; do
        log "Pulling $model..."
        docker exec horizonos-ollama ollama pull "$model" || \
            log "Failed to pull $model" "$YELLOW"
    done
    
    log "Development models ready" "$GREEN"
}

# Create development configs
create_dev_configs() {
    log "Creating development configurations..." "$BLUE"
    
    mkdir -p "$PROJECT_ROOT/config/dev"
    
    # Create development AI config
    cat > "$PROJECT_ROOT/config/dev/ai-config.toml" <<'EOF'
# Development AI Configuration
[ai]
enabled = true
ollama_endpoint = "http://localhost:11434"
default_model = "tinyllama:latest"

[hardware]
optimization = "AUTO"

[privacy]
local_only = true
telemetry_enabled = false
encrypt_storage = false  # Disabled for development

[learning]
enabled = true
min_confidence = 0.5  # Lower threshold for testing
min_occurrences = 2   # Lower for testing

[monitoring]
sample_interval = 2   # More frequent for testing
idle_threshold = 60   # Shorter idle time
EOF
    
    # Create VS Code settings
    cat > "$PROJECT_ROOT/.vscode/settings.json" <<'EOF'
{
    "rust-analyzer.cargo.features": ["all"],
    "rust-analyzer.checkOnSave.command": "clippy",
    "editor.formatOnSave": true,
    "[rust]": {
        "editor.defaultFormatter": "rust-lang.rust-analyzer"
    },
    "[kotlin]": {
        "editor.defaultFormatter": "fwcd.kotlin"
    }
}
EOF
    
    # Create VS Code launch configurations
    cat > "$PROJECT_ROOT/.vscode/launch.json" <<'EOF'
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug AI Service",
            "cargo": {
                "args": ["build", "--bin=horizonos-ai-service"],
                "filter": {
                    "name": "horizonos-ai-service",
                    "kind": "bin"
                }
            },
            "args": [],
            "cwd": "${workspaceFolder}/src/desktop/graph-ai"
        },
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug AI Monitor",
            "cargo": {
                "args": ["build", "--bin=horizonos-ai-monitor"],
                "filter": {
                    "name": "horizonos-ai-monitor",
                    "kind": "bin"
                }
            },
            "args": [],
            "cwd": "${workspaceFolder}/src/desktop/graph-ai"
        }
    ]
}
EOF
    
    log "Development configurations created" "$GREEN"
}

# Setup development tools
setup_dev_tools() {
    log "Setting up development tools..." "$BLUE"
    
    # Create useful aliases
    cat > "$PROJECT_ROOT/.dev-aliases" <<'EOF'
# HorizonOS AI Development Aliases
alias ai-logs='docker-compose logs -f ollama'
alias ai-db='docker exec -it horizonos-timescaledb psql -U postgres horizonos'
alias ai-test='cd $PROJECT_ROOT/src/desktop/graph-ai && cargo test'
alias ai-run='cd $PROJECT_ROOT/src/desktop/graph-ai && cargo run'
alias ai-watch='cd $PROJECT_ROOT/src/desktop/graph-ai && cargo watch -x test'
alias ai-bench='cd $PROJECT_ROOT && cargo bench'
alias ai-grafana='xdg-open http://localhost:3000'
alias ai-n8n='xdg-open http://localhost:5678'
alias ai-pgadmin='xdg-open http://localhost:5050'
EOF
    
    log "Development tools ready" "$GREEN"
    log "Source .dev-aliases to use development shortcuts" "$YELLOW"
}

# Print development info
print_dev_info() {
    log "=== Development Environment Ready ===" "$GREEN"
    echo ""
    log "Service URLs:" "$BLUE"
    log "  - Ollama API: http://localhost:11434"
    log "  - TimescaleDB: postgresql://postgres:password@localhost:5432/horizonos"
    log "  - n8n Workflows: http://localhost:5678"
    log "  - Grafana Dashboard: http://localhost:3000 (admin/admin)"
    log "  - PgAdmin: http://localhost:5050 (admin@horizonos.local/admin)"
    log "  - Jupyter Notebook: http://localhost:8888"
    log "  - MinIO Console: http://localhost:9001 (minioadmin/minioadmin)"
    echo ""
    log "Quick Commands:" "$BLUE"
    log "  - Run tests: cargo test"
    log "  - Run benchmarks: cargo bench"
    log "  - Watch tests: cargo watch -x test"
    log "  - View logs: docker-compose logs -f"
    log "  - Access DB: docker exec -it horizonos-timescaledb psql -U postgres horizonos"
    echo ""
    log "Development configs in: $PROJECT_ROOT/config/dev/" "$YELLOW"
}

# Main setup flow
main() {
    log "=== HorizonOS AI Development Setup ===" "$BLUE"
    
    check_prerequisites
    setup_rust_deps
    setup_kotlin_dsl
    start_containers
    init_dev_database
    pull_dev_models
    create_dev_configs
    setup_dev_tools
    
    print_dev_info
}

# Run main function
main "$@"