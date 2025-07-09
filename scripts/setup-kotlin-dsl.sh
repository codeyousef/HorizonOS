#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KOTLIN_DIR="$PROJECT_ROOT/src/kotlin-config"

echo "=== Setting up HorizonOS Kotlin DSL ==="

# Create directory structure
echo "Creating directory structure..."
mkdir -p "$KOTLIN_DIR"/{src/main/kotlin/org/horizonos/config/{dsl,generators},examples,gradle/wrapper}

# Initialize Gradle wrapper
echo "Initializing Gradle..."
cd "$KOTLIN_DIR"

# Download gradle wrapper
if [ ! -f "gradlew" ]; then
    gradle wrapper --gradle-version=8.5
fi

# Create settings.gradle.kts
cat > settings.gradle.kts << 'EOF'
rootProject.name = "horizonos-kotlin-config"
EOF

# Create gradle.properties
cat > gradle.properties << 'EOF'
kotlin.code.style=official
kotlin.incremental=true
org.gradle.daemon=true
org.gradle.parallel=true
EOF

# Copy the DSL core file
echo "Setting up source files..."
mkdir -p src/main/kotlin/org/horizonos/config/dsl

# Create a simple test
mkdir -p src/test/kotlin/org/horizonos/config
cat > src/test/kotlin/org/horizonos/config/DslTest.kt << 'EOF'
package org.horizonos.config

import io.kotest.core.spec.style.StringSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.shouldNotBe
import org.horizonos.config.dsl.*

class DslTest : StringSpec({
    "should create basic configuration" {
        val config = horizonOS {
            hostname = "test-system"
            timezone = "UTC"
            
            packages {
                install("base", "linux")
            }
            
            services {
                enable("NetworkManager")
            }
        }
        
        config.system.hostname shouldBe "test-system"
        config.packages.size shouldBe 2
        config.services.size shouldBe 1
    }
    
    "should handle package groups" {
        val config = horizonOS {
            packages {
                group("development") {
                    install("git", "vim", "gcc")
                }
            }
        }
        
        config.packages.size shouldBe 3
        config.packages.all { it.group == "development" } shouldBe true
    }
    
    "should configure users with groups" {
        val config = horizonOS {
            users {
                user("testuser") {
                    uid = 1000
                    shell = "/usr/bin/fish"
                    groups("wheel", "docker")
                }
            }
        }
        
        val user = config.users.first()
        user.name shouldBe "testuser"
        user.groups shouldBe listOf("wheel", "docker")
    }
})
EOF

# Create a CLI wrapper
cat > horizonos-config << 'EOF'
#!/bin/bash
cd "$(dirname "$0")"
./gradlew run --args="$@"
EOF
chmod +x horizonos-config

# Build the project
echo "Building project..."
./gradlew build

echo ""
echo "✓ Kotlin DSL setup complete!"
echo ""
echo "Project location: $KOTLIN_DIR"
echo ""
echo "Available commands:"
echo "  cd $KOTLIN_DIR"
echo "  ./gradlew test              # Run tests"
echo "  ./gradlew run               # Run the compiler"
echo "  ./gradlew compileConfig -PconfigFile=examples/desktop.horizonos.kts"
echo ""
echo "Next steps:"
echo "1. Add the DSL core implementation to src/main/kotlin/org/horizonos/config/dsl/Core.kt"
echo "2. Add the compiler to src/main/kotlin/org/horizonos/config/Compiler.kt"
echo "3. Create example configs in the examples/ directory"
echo "4. Run tests with: ./gradlew test"