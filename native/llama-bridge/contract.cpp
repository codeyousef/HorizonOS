#include "bridge.h"
#include <cassert>
#include <cstdlib>
#include <cstring>
int main(int argc,char ** argv) {
    assert(aios_abi_version()==1);
    assert(aios_model_open(nullptr,nullptr)==AIOS_INVALID);
    aios_model * model=nullptr;
    assert(aios_model_open("/etc/passwd",&model)==AIOS_INVALID && !model);
    aios_context * context=nullptr;
    assert(aios_context_new(nullptr,8193,4,nullptr,&context)==AIOS_LIMIT && !context);
    auto cancel=aios_cancel_new();assert(cancel);assert(!aios_cancelled(cancel));aios_cancel_set(cancel);assert(aios_cancelled(cancel));
    assert(aios_model_open_cancelable("/proc/self/fd/0",cancel,&model)==AIOS_CANCELLED && !model);aios_cancel_free(cancel);
    if (argc>1 && std::strcmp(argv[1],"--reject-backend-override")==0) {
        assert(setenv("GGML_BACKEND_PATH","/untrusted/backend.so",1)==0);
        assert(aios_cpu_backend_check()==AIOS_BACKEND);
    } else if (argc>1) assert(aios_cpu_backend_check()==AIOS_OK);
}
