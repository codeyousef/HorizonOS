#pragma once
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
// ABI v1. Owned opaque objects; free only after all calls have finished.
// Context calls are serialized by Rust. Only cancellation is concurrent.
typedef struct aios_model aios_model;
typedef struct aios_context aios_context;
typedef struct aios_cancel aios_cancel;
enum aios_result { AIOS_OK=0, AIOS_INVALID=1, AIOS_LIMIT=2, AIOS_LOAD=3,
    AIOS_DECODE=4, AIOS_CANCELLED=5, AIOS_END=6, AIOS_BACKEND=7, AIOS_TEMPLATE=8 };
uint32_t aios_abi_version(void);
const char * aios_runtime_revision(void);
int aios_cpu_backend_check(void);
aios_cancel * aios_cancel_new(void);
void aios_cancel_set(aios_cancel *);
uint32_t aios_cancelled(aios_cancel *);
void aios_cancel_free(aios_cancel *);
int aios_model_open(const char * verified_descriptor_path, aios_model ** out);
int aios_model_open_cancelable(const char * verified_descriptor_path, aios_cancel *, aios_model ** out);
void aios_model_free(aios_model *);
int aios_model_template(aios_model *, char * out, size_t capacity, size_t * written);
int aios_chat_format(aios_model *, const char * system, const char * user,
    char * out, size_t capacity, size_t * written);
int aios_context_new(aios_model *, uint32_t tokens, uint32_t threads,
    aios_cancel *, aios_context ** out);
void aios_context_free(aios_context *);
int aios_context_prompt(aios_context *, const char * prompt, const char * grammar,
    uint32_t max_input_tokens, uint32_t * input_tokens);
// Returns one token's bytes into a caller-owned buffer, or END/CANCELLED.
int aios_context_next(aios_context *, char * out, size_t capacity, size_t * written);
#ifdef __cplusplus
}
#endif
