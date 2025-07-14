package org.horizonos.config

fun main(args: Array<String>) {
    println("HorizonOS Configuration System")
    println("Usage:")
    println("  compile <config-file>  - Compile a configuration file")

    if (args.isNotEmpty() && args[0] == "compile") {
        CompileCommand().main(args.drop(1).toTypedArray())
    } else {
        println("Unknown command. Use 'compile' to compile configuration files.")
    }
}
