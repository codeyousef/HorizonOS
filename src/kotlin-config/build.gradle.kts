import org.jetbrains.kotlin.gradle.tasks.KotlinCompile

plugins {
    kotlin("jvm") version "1.9.22"
    kotlin("plugin.serialization") version "1.9.22"
    application
}

group = "org.horizonos"
version = "0.1.0-dev"

repositories {
    mavenCentral()
}

dependencies {
    // Kotlin standard library
    implementation(kotlin("stdlib"))

    // Kotlin scripting support for DSL
    implementation("org.jetbrains.kotlin:kotlin-scripting-common")
    implementation("org.jetbrains.kotlin:kotlin-scripting-jvm")
    implementation("org.jetbrains.kotlin:kotlin-scripting-jvm-host")

    // Serialization for config output
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.2")

    // Coroutines for async operations
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.3")

    // CLI argument parsing
    implementation("com.github.ajalt.clikt:clikt:4.2.1")

    // File operations
    implementation("com.squareup.okio:okio:3.6.0")

    // Testing
    testImplementation(kotlin("test"))
    testImplementation("io.kotest:kotest-runner-junit5:5.8.0")
    testImplementation("io.kotest:kotest-assertions-core:5.8.0")
    testImplementation("io.mockk:mockk:1.13.8")
}

tasks.test {
    useJUnitPlatform()
}

// Use current Java version for both Kotlin and Java compilation
tasks.withType<KotlinCompile> {
    kotlinOptions {
        jvmTarget = "21"  // Compatible with Java 24
        freeCompilerArgs = listOf(
            "-Xjsr305=strict",
            "-opt-in=kotlin.RequiresOptIn"
        )
    }
}

tasks.withType<JavaCompile> {
    targetCompatibility = "21"
    sourceCompatibility = "21"
}

application {
    mainClass.set("org.horizonos.config.MainKt")
}

// Task to compile a HorizonOS configuration
tasks.register<JavaExec>("compileConfig") {
    group = "horizonos"
    description = "Compile a HorizonOS configuration file"

    classpath = sourceSets["main"].runtimeClasspath
    mainClass.set("org.horizonos.config.CompilerKt")

    // Pass through command line arguments
    args = project.findProperty("configFile")?.toString()?.let { listOf(it) } ?: emptyList()
}
