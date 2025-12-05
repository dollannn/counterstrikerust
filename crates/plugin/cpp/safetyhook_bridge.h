#pragma once

#include <cstdint>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque handle types for Rust
typedef struct SafetyHookInlineHandle* InlineHookHandle;
typedef struct SafetyHookMidHandle* MidHookHandle;

// Error codes matching Rust HookError
typedef enum HookResult {
    HOOK_SUCCESS = 0,
    HOOK_ERROR_ALLOCATION = 1,
    HOOK_ERROR_DECODE = 2,
    HOOK_ERROR_UNPROTECT = 3,
    HOOK_ERROR_NOT_ENOUGH_SPACE = 4,
    HOOK_ERROR_UNSUPPORTED = 5,
    HOOK_ERROR_IP_RELATIVE = 6,
    HOOK_ERROR_INVALID = 7,
} HookResult;

// MidHook context matching CS2Rust's MidHookContext layout exactly
// Layout: xmm[16], rflags, r15-r8, rdi, rsi, rbp, rdx, rcx, rbx, rax, rsp
typedef struct RustMidHookContext {
    uint8_t xmm[256];     // 16 XMM registers * 16 bytes each
    uint64_t rflags;
    uint64_t r15;
    uint64_t r14;
    uint64_t r13;
    uint64_t r12;
    uint64_t r11;
    uint64_t r10;
    uint64_t r9;
    uint64_t r8;
    uint64_t rdi;
    uint64_t rsi;
    uint64_t rbp;         // Note: different position than SafetyHook
    uint64_t rdx;
    uint64_t rcx;
    uint64_t rbx;
    uint64_t rax;
    uint64_t rsp;
} RustMidHookContext;

// Callback type for mid hooks (matches Rust side)
typedef void (*MidHookCallback)(RustMidHookContext* ctx, void* user_data);

// === Inline Hook API ===

// Create an inline hook. Returns trampoline pointer (original function).
HookResult safetyhook_create_inline(
    void* target,
    void* destination,
    InlineHookHandle* out_handle,
    void** out_trampoline
);

// Enable a previously disabled inline hook
HookResult safetyhook_enable_inline(InlineHookHandle handle);

// Disable an inline hook (can be re-enabled)
HookResult safetyhook_disable_inline(InlineHookHandle handle);

// Destroy an inline hook and free resources
void safetyhook_destroy_inline(InlineHookHandle handle);

// Check if an inline hook is currently enabled
bool safetyhook_is_inline_enabled(InlineHookHandle handle);

// Get the trampoline address for an inline hook
void* safetyhook_get_inline_trampoline(InlineHookHandle handle);

// === Mid Hook API ===

// Create a mid-function hook with full register context
HookResult safetyhook_create_mid(
    void* target,
    MidHookCallback callback,
    void* user_data,
    MidHookHandle* out_handle
);

// Enable a previously disabled mid hook
HookResult safetyhook_enable_mid(MidHookHandle handle);

// Disable a mid hook (can be re-enabled)
HookResult safetyhook_disable_mid(MidHookHandle handle);

// Destroy a mid hook and free resources
void safetyhook_destroy_mid(MidHookHandle handle);

// Check if a mid hook is currently enabled
bool safetyhook_is_mid_enabled(MidHookHandle handle);

#ifdef __cplusplus
}
#endif
