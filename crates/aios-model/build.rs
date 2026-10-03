fn main() {
    println!("cargo:rerun-if-env-changed=AIOS_LLAMA_BRIDGE");
    let root = std::env::var("AIOS_LLAMA_BRIDGE").expect("Build aios-model inside the pinned Nix environment with AIOS_LLAMA_BRIDGE");
    assert!(root.starts_with("/nix/store/"), "native bridge must be a pinned store output");
    let source = std::env::var("AIOS_LLAMA_SOURCE").expect("Pinned converter source is required");
    assert!(source.starts_with("/nix/store/"), "converter must be a pinned store source");
    println!("cargo:rerun-if-env-changed=AIOS_LLAMA_SOURCE");
    println!("cargo:rustc-env=AIOS_PINNED_QUANTIZER={root}/bin/llama-quantize");
    println!("cargo:rustc-env=AIOS_PINNED_CONVERTER={source}/convert_hf_to_gguf.py");
    println!("cargo:rustc-link-search=native={root}/lib");
    println!("cargo:rustc-link-lib=dylib=aios-llama-bridge");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{root}/lib");
}
