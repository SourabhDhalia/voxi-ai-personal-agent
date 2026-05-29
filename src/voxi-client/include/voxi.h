/*
 * Copyright (c) 2026 Samsung Electronics Co., Ltd All Rights Reserved
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

/**
 * @file voxi.h
 * @brief Voxi Agent C API
 *
 * Provides C-compatible interface to the Voxi AI agent system.
 * Internal implementation is in Rust; this header exposes the FFI boundary.
 *
 * @section Usage
 * @code
 * #include <voxi/voxi.h>
 *
 * voxi_h agent;
 * int ret = voxi_create(&agent);
 * if (ret != VOXI_ERROR_NONE) { ... }
 *
 * ret = voxi_initialize(agent);
 * char *response = voxi_process_prompt(agent, "default", "Hello!");
 * printf("Response: %s\n", response);
 * voxi_free_string(response);
 *
 * voxi_destroy(agent);
 * @endcode
 */

#ifndef __VOXI_H__
#define __VOXI_H__

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Error codes returned by Voxi API functions.
 */
typedef enum {
    VOXI_ERROR_NONE = 0,              /**< Success */
    VOXI_ERROR_INVALID_PARAMETER = -1, /**< Invalid parameter */
    VOXI_ERROR_OUT_OF_MEMORY = -2,     /**< Memory allocation failure */
    VOXI_ERROR_NOT_INITIALIZED = -3,   /**< Agent not initialized */
    VOXI_ERROR_ALREADY_INITIALIZED = -4, /**< Agent already initialized */
    VOXI_ERROR_IO = -5,                /**< I/O error */
    VOXI_ERROR_LLM_FAILED = -6,       /**< LLM backend failure */
    VOXI_ERROR_TOOL_FAILED = -7,      /**< Tool execution failure */
    VOXI_ERROR_NOT_SUPPORTED = -8,    /**< Operation not supported */
} voxi_error_e;

/**
 * @brief Opaque handle to a Voxi agent instance.
 */
typedef struct voxi_s *voxi_h;

/**
 * @brief Callback for asynchronous prompt processing.
 *
 * @param[in] response  The complete response text (UTF-8, null-terminated)
 * @param[in] error     Error code (VOXI_ERROR_NONE on success)
 * @param[in] user_data User data passed to voxi_process_prompt_async()
 */
typedef void (*voxi_response_cb)(const char *response,
                                 int error,
                                 void *user_data);

/**
 * @brief Callback for streaming chunks during prompt processing.
 *
 * @param[in] chunk     A partial response chunk (UTF-8, null-terminated)
 * @param[in] user_data User data passed to voxi_process_prompt_async()
 */
typedef void (*voxi_stream_cb)(const char *chunk,
                               void *user_data);

/* ═══════════════════════════════════════════
 *  Lifecycle
 * ═══════════════════════════════════════════ */

/**
 * @brief Create a new Voxi agent instance.
 *
 * @param[out] handle  Pointer to receive the agent handle
 * @return VOXI_ERROR_NONE on success
 */
int voxi_create(voxi_h *handle);

/**
 * @brief Initialize the agent (loads config, LLM backends, tools).
 *
 * Must be called after voxi_create() and before any other operations.
 *
 * @param[in] handle  Agent handle
 * @return VOXI_ERROR_NONE on success
 */
int voxi_initialize(voxi_h handle);

/**
 * @brief Destroy the agent and release all resources.
 *
 * @param[in] handle  Agent handle (becomes invalid after this call)
 */
void voxi_destroy(voxi_h handle);

/* ═══════════════════════════════════════════
 *  Prompt Processing
 * ═══════════════════════════════════════════ */

/**
 * @brief Process a prompt synchronously.
 *
 * @param[in] handle      Agent handle
 * @param[in] session_id  Session identifier (UTF-8)
 * @param[in] prompt      User prompt text (UTF-8)
 * @return Heap-allocated response string (caller must free with voxi_free_string()),
 *         or NULL on error (check voxi_last_error())
 */
char *voxi_process_prompt(voxi_h handle,
                          const char *session_id,
                          const char *prompt);

/**
 * @brief Process a prompt asynchronously.
 *
 * @param[in] handle      Agent handle
 * @param[in] session_id  Session identifier (UTF-8)
 * @param[in] prompt      User prompt text (UTF-8)
 * @param[in] callback    Completion callback
 * @param[in] user_data   User data for callback
 * @return VOXI_ERROR_NONE on success (callback will be invoked)
 */
int voxi_process_prompt_async(voxi_h handle,
                              const char *session_id,
                              const char *prompt,
                              voxi_response_cb callback,
                              void *user_data);

/* ═══════════════════════════════════════════
 *  Session Management
 * ═══════════════════════════════════════════ */

/**
 * @brief Clear a session's conversation history.
 *
 * @param[in] handle      Agent handle
 * @param[in] session_id  Session identifier
 * @return VOXI_ERROR_NONE on success
 */
int voxi_clear_session(voxi_h handle, const char *session_id);

/* ═══════════════════════════════════════════
 *  Monitoring
 * ═══════════════════════════════════════════ */

/**
 * @brief Get agent status as JSON.
 *
 * @param[in] handle  Agent handle
 * @return JSON string (caller must free with voxi_free_string()),
 *         or NULL on error
 */
char *voxi_get_status(voxi_h handle);

/**
 * @brief Get system metrics as JSON (memory, CPU, uptime, counters).
 *
 * @param[in] handle  Agent handle
 * @return JSON string (caller must free with voxi_free_string()),
 *         or NULL on error
 */
char *voxi_get_metrics(voxi_h handle);

/* ═══════════════════════════════════════════
 *  Tools & Skills
 * ═══════════════════════════════════════════ */

/**
 * @brief Get available tools as JSON array.
 *
 * @param[in] handle  Agent handle
 * @return JSON string (caller must free with voxi_free_string()),
 *         or NULL on error
 */
char *voxi_get_tools(voxi_h handle);

/**
 * @brief Execute a tool directly by name.
 *
 * @param[in] handle     Agent handle
 * @param[in] tool_name  Tool name (UTF-8)
 * @param[in] args_json  Tool arguments as JSON string (UTF-8)
 * @return JSON result string (caller must free with voxi_free_string()),
 *         or NULL on error
 */
char *voxi_execute_tool(voxi_h handle,
                        const char *tool_name,
                        const char *args_json);

/**
 * @brief Force reload of skill manifests.
 *
 * @param[in] handle  Agent handle
 * @return VOXI_ERROR_NONE on success
 */
int voxi_reload_skills(voxi_h handle);

/* ═══════════════════════════════════════════
 *  Web Dashboard
 * ═══════════════════════════════════════════ */

/**
 * @brief Start the web dashboard on the specified port.
 *
 * @param[in] handle  Agent handle
 * @param[in] port    TCP port to listen on (e.g. 9090)
 * @return VOXI_ERROR_NONE on success
 */
int voxi_start_dashboard(voxi_h handle, uint16_t port);

/**
 * @brief Stop the web dashboard.
 *
 * @param[in] handle  Agent handle
 * @return VOXI_ERROR_NONE on success
 */
int voxi_stop_dashboard(voxi_h handle);

/* ═══════════════════════════════════════════
 *  Utility
 * ═══════════════════════════════════════════ */

/**
 * @brief Free a string returned by Voxi API functions.
 *
 * @param[in] str  String to free (NULL is safe)
 */
void voxi_free_string(char *str);

/**
 * @brief Get the last error message (thread-local).
 *
 * @return Error message string (static, do NOT free), or NULL if no error
 */
const char *voxi_last_error(void);

#ifdef __cplusplus
}
#endif

#endif /* __VOXI_H__ */
