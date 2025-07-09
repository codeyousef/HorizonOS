#!/bin/bash
# Test HorizonOS Kotlin DSL

cd src/kotlin-config

echo "=== Testing HorizonOS Kotlin DSL ==="
echo ""

# Download gradle wrapper jar if needed
if [ ! -f "gradle/wrapper/gradle-wrapper.jar" ]; then
    echo "Downloading Gradle wrapper..."
    mkdir -p gradle/wrapper
    curl -sL https://github.com/gradle/gradle/raw/v8.5.0/gradle/wrapper/gradle-wrapper.jar \
         -o gradle/wrapper/gradle-wrapper.jar
fi

echo "Building Kotlin DSL project..."
./gradlew build

echo ""
echo "Compiling example configuration..."
./gradlew compileConfig -PconfigFile=examples/desktop.horizonos.kts

echo ""
echo "Generated files:"
if [ -d "output" ]; then
    find output -type f | sort
else
    echo "No output directory found"
fi