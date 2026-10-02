#include "bridge.h"
#include "llama.h"
#include "chat.h"
#include "ggml-backend.h"
#include <atomic>
#include <cstring>
#include <cstdlib>
#include <memory>
#include <mutex>
#include <string>
#include <vector>

struct aios_cancel { std::shared_ptr<std::atomic_bool> value; };
struct aios_model {
    llama_model * value = nullptr;
    common_chat_templates_ptr templates;
    ~aios_model() { templates.reset(); if (value) llama_model_free(value); }
};
struct aios_context {
    aios_model * model = nullptr;
    llama_context * value = nullptr;
    llama_sampler * sampler = nullptr;
    std::shared_ptr<std::atomic_bool> cancelled;
    bool ready = false;
    bool started = false;
    uint32_t used = 0;
    uint32_t capacity = 0;
    ~aios_context() { if (sampler) llama_sampler_free(sampler); if (value) { llama_memory_clear(llama_get_memory(value),true);llama_free(value); } }
};
static std::once_flag initialization;
static bool cpu_only = false;
static void quiet_log(ggml_log_level, const char *, void *) {}
static void initialize() {
    std::call_once(initialization, [] {
        // Ordinary inference logs never contain model metadata or prompt text.
        llama_log_set(quiet_log, nullptr);
        // Upstream also consults this variable when an explicit directory is
        // supplied. Reject it before any external backend can be dlopened.
        if (std::getenv("GGML_BACKEND_PATH") != nullptr) return;
        ggml_backend_load_all_from_path(AIOS_BACKEND_DIR);
        llama_backend_init();
        cpu_only = ggml_backend_dev_count() > 0;
        for (size_t i=0; i<ggml_backend_dev_count(); ++i)
            cpu_only = cpu_only && ggml_backend_dev_type(ggml_backend_dev_get(i)) == GGML_BACKEND_DEVICE_TYPE_CPU;
    });
}
static bool abort_decode(void * value) {
    return static_cast<std::atomic_bool *>(value)->load(std::memory_order_acquire);
}
static bool text_bound(const char * text, size_t maximum) {
    return text && strnlen(text, maximum+1) <= maximum;
}
static int copy(const std::string & value, char * out, size_t capacity, size_t * written) {
    if (!out || !written) return AIOS_INVALID;
    *written = 0;
    if (value.size() > capacity) return AIOS_LIMIT;
    std::memcpy(out, value.data(), value.size()); *written = value.size(); return AIOS_OK;
}
extern "C" {
uint32_t aios_abi_version() { return 1; }
const char * aios_runtime_revision() { return "b64739ea393b3c9d07cc9907e0a611f707838051"; }
int aios_cpu_backend_check() { try { initialize(); return cpu_only ? AIOS_OK : AIOS_BACKEND; } catch (...) { return AIOS_BACKEND; } }
aios_cancel * aios_cancel_new() {
    try { return new aios_cancel{std::make_shared<std::atomic_bool>(false)}; } catch (...) { return nullptr; }
}
void aios_cancel_set(aios_cancel * token) { if (token) token->value->store(true, std::memory_order_release); }
void aios_cancel_free(aios_cancel * token) { delete token; }
int aios_model_open(const char * path, aios_model ** out) {
    if (!out) return AIOS_INVALID; *out = nullptr;
    if (!text_bound(path,4096) || std::strncmp(path,"/proc/self/fd/",14) != 0) return AIOS_INVALID;
    try {
        initialize(); if (!cpu_only) return AIOS_BACKEND;
        auto model = std::make_unique<aios_model>();
        auto params = llama_model_default_params();
        ggml_backend_dev_t devices[] = {nullptr};
        params.devices = devices; params.n_gpu_layers = 0;
        params.use_mmap = true; params.use_mlock = false; params.check_tensors = true;
        model->value = llama_model_load_from_file(path,params);
        if (!model->value) return AIOS_LOAD;
        const char * templ = llama_model_chat_template(model->value,nullptr);
        if (!text_bound(templ,32768) || !*templ) return AIOS_TEMPLATE;
        model->templates = common_chat_templates_init(model->value,"");
        if (!model->templates || !common_chat_templates_support_enable_thinking(model->templates.get())) return AIOS_TEMPLATE;
        *out = model.release(); return AIOS_OK;
    } catch (...) { return AIOS_LOAD; }
}
void aios_model_free(aios_model * model) { delete model; }
int aios_model_template(aios_model * model,char * out,size_t capacity,size_t * written) {
    if (!model) return AIOS_INVALID;
    try { return copy(common_chat_templates_source(model->templates.get()),out,capacity,written); }
    catch (...) { return AIOS_TEMPLATE; }
}
int aios_chat_format(aios_model * model,const char * system,const char * user,char * out,size_t capacity,size_t * written) {
    if (!model || !text_bound(system,16384) || !text_bound(user,65536)) return AIOS_INVALID;
    try {
        common_chat_templates_inputs inputs;
        common_chat_msg s,u; s.role="system";s.content=system;u.role="user";u.content=user;
        inputs.messages={s,u}; inputs.enable_thinking=false; inputs.use_jinja=true;
        inputs.add_generation_prompt=true; inputs.force_pure_content=true;
        auto rendered=common_chat_templates_apply(model->templates.get(),inputs);
        return copy(rendered.prompt,out,capacity,written);
    } catch (...) { return AIOS_TEMPLATE; }
}
int aios_context_new(aios_model * model,uint32_t tokens,uint32_t threads,aios_cancel * cancel,aios_context ** out) {
    if (!out) return AIOS_INVALID; *out=nullptr;
    if (!model || !cancel || tokens<512 || tokens>8192 || threads<1 || threads>4) return AIOS_LIMIT;
    try {
        auto context=std::make_unique<aios_context>(); context->model=model;context->cancelled=cancel->value;context->capacity=tokens;
        auto params=llama_context_default_params();
        params.n_ctx=tokens;params.n_batch=256;params.n_ubatch=256;params.n_seq_max=1;
        params.n_threads=threads;params.n_threads_batch=threads;
        params.offload_kqv=false;params.op_offload=false;
        params.abort_callback=abort_decode;params.abort_callback_data=context->cancelled.get();
        context->value=llama_init_from_model(model->value,params);
        if (!context->value) return AIOS_LOAD;
        *out=context.release();return AIOS_OK;
    } catch (...) { return AIOS_LOAD; }
}
void aios_context_free(aios_context * context) { delete context; }
int aios_context_prompt(aios_context * ctx,const char * prompt,const char * grammar,uint32_t maximum,uint32_t * input_tokens) {
    if (!ctx || !input_tokens || ctx->started || !text_bound(prompt,131072) || !text_bound(grammar,32768)) return AIOS_INVALID;
    *input_tokens=0;
    if (maximum<1 || maximum>6144 || !*grammar) return AIOS_LIMIT;
    // A failed prefill is never resumed with partial KV state or a new sampler.
    ctx->started = true;
    if (ctx->cancelled->load()) return AIOS_CANCELLED;
    try {
        auto vocab=llama_model_get_vocab(ctx->model->value);
        std::vector<llama_token> tokens(maximum+1);
        int count=llama_tokenize(vocab,prompt,std::strlen(prompt),tokens.data(),tokens.size(),true,true);
        if (count<=0 || uint32_t(count)>maximum || uint32_t(count)>=ctx->capacity) return AIOS_LIMIT;
        std::unique_ptr<llama_sampler, decltype(&llama_sampler_free)> constraint(llama_sampler_init_grammar(vocab,grammar,"root"),llama_sampler_free);
        if (!constraint) return AIOS_INVALID;
        ctx->sampler=llama_sampler_chain_init(llama_sampler_chain_default_params());
        if (!ctx->sampler) return AIOS_LOAD;
        llama_sampler_chain_add(ctx->sampler,constraint.release());
        llama_sampler_chain_add(ctx->sampler,llama_sampler_init_greedy());
        for (int offset=0;offset<count;offset+=256) {
            if (ctx->cancelled->load()) return AIOS_CANCELLED;
            int batch_size=std::min(256,count-offset);
            auto batch=llama_batch_get_one(tokens.data()+offset,batch_size);
            if (llama_decode(ctx->value,batch)!=0) return ctx->cancelled->load()?AIOS_CANCELLED:AIOS_DECODE;
        }
        ctx->ready=true;ctx->used=count;*input_tokens=count;return AIOS_OK;
    } catch (...) { return AIOS_DECODE; }
}
int aios_context_next(aios_context * ctx,char * out,size_t capacity,size_t * written) {
    if (!ctx || !out || !written || !ctx->ready) return AIOS_INVALID;
    *written=0;
    if (ctx->cancelled->load()) return AIOS_CANCELLED;
    if (ctx->used>=ctx->capacity) return AIOS_LIMIT;
    try {
        auto vocab=llama_model_get_vocab(ctx->model->value);
        auto token=llama_sampler_sample(ctx->sampler,ctx->value,-1);
        if (llama_vocab_is_eog(vocab,token)) { ctx->ready=false;return AIOS_END; }
        if (capacity>16384) return AIOS_LIMIT;
        int count=llama_token_to_piece(vocab,token,out,capacity,0,false);
        if (count<0 || size_t(count)>capacity) return AIOS_LIMIT;
        auto batch=llama_batch_get_one(&token,1);
        if (llama_decode(ctx->value,batch)!=0) return ctx->cancelled->load()?AIOS_CANCELLED:AIOS_DECODE;
        ++ctx->used;*written=count;return AIOS_OK;
    } catch (...) { return AIOS_DECODE; }
}
}
