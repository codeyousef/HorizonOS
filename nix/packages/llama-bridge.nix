{ lib, stdenv, cmake, ninja, llama-cpp }:
assert llama-cpp.version == "9190";
stdenv.mkDerivation {
  pname = "minnerite-llama-bridge";
  version = "1-b9190";
  src = ../../native/llama-bridge;
  nativeBuildInputs = [ cmake ninja ];
  preConfigure = ''
    revisionPrefix=$(cat ${llama-cpp.src}/COMMIT)
    test "''${#revisionPrefix}" -ge 7
    case b64739ea393b3c9d07cc9907e0a611f707838051 in
      "$revisionPrefix"*) ;;
      *) echo "Pinned runtime revision mismatch" >&2; exit 1 ;;
    esac
  '';
  cmakeFlags = [
    "-DLLAMA_SOURCE=${llama-cpp.src}"
    "-DGGML_NATIVE=OFF" "-DGGML_CPU=ON"
    "-DGGML_BACKEND_DL=ON" "-DGGML_CPU_ALL_VARIANTS=ON"
    "-DGGML_CUDA=OFF" "-DGGML_HIP=OFF" "-DGGML_VULKAN=OFF"
    "-DGGML_METAL=OFF" "-DGGML_OPENCL=OFF" "-DGGML_RPC=OFF" "-DGGML_BLAS=OFF"
    "-DLLAMA_BUILD_NUMBER=9190"
    "-DLLAMA_BUILD_COMMIT=b64739ea393b3c9d07cc9907e0a611f707838051"
  ];
  postInstall = ''
    test -x "$out/bin/llama-quantize"
    install -Dm644 ${llama-cpp.src}/LICENSE "$out/share/licenses/llama.cpp/LICENSE"
    bin/aios-bridge-contract --installed-backend
  '';
  doCheck = true;
  checkPhase = "ctest --output-on-failure";
  meta = { description = "Minnerite bounded CPU inference ABI"; license = lib.licenses.mit; platforms = [ "x86_64-linux" ]; };
}
