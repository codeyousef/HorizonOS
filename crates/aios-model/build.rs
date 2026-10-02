fn main() {
    println!("cargo:rerun-if-env-changed=AIOS_LLAMA_BRIDGE");
    let root = std::env::var("AIOS_LLAMA_BRIDGE").expect("Build aios-model inside the pinned Nix environment with AIOS_LLAMA_BRIDGE");
    assert!(root.starts_with("/nix/store/"), "native bridge must be a pinned store output");
    println!("cargo:rustc-link-search=native={root}/lib");
    println!("cargo:rustc-link-lib=dylib=aios-llama-bridge");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{root}/lib");
}
