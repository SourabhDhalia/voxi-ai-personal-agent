#ifndef VOXI_LLM_BACKEND_H_
#define VOXI_LLM_BACKEND_H_

#include <stdbool.h>
#include <voxi_error.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief The LLM response handle.
 */
typedef void* voxi_llm_response_h;

/**
 * @brief The LLM messages list handle.
 */
typedef void* voxi_llm_messages_h;

/**
 * @brief The LLM message handle.
 */
typedef void* voxi_llm_message_h;

/**
 * @brief The LLM tools list handle.
 */
typedef void* voxi_llm_tools_h;

/**
 * @brief The LLM tool handle.
 */
typedef void* voxi_llm_tool_h;

/**
 * @brief The LLM tool call handle.
 */
typedef void* voxi_llm_tool_call_h;

/**
 * @brief Callback for streaming chunks.
 * @param[in] chunk The text chunk.
 * @param[in] user_data User data context passed from the caller.
 */
typedef void (*voxi_llm_backend_chunk_cb)(const char* chunk,
                                          void* user_data);

/**
 * @brief Callback for iterating over tool calls in a response or message.
 * @param[in] tool_call The tool call handle.
 * @param[in] user_data User data.
 * @return @c true to continue, @c false to stop.
 */
typedef bool (*voxi_llm_tool_call_cb)(voxi_llm_tool_call_h tool_call,
                                      void* user_data);

/**
 * @brief Callback for iterating over messages.
 * @param[in] message The message handle.
 * @param[in] user_data User data.
 * @return @c true to continue, @c false to stop.
 */
typedef bool (*voxi_llm_message_cb)(voxi_llm_message_h message,
                                    void* user_data);

/**
 * @brief Callback for iterating over tools.
 * @param[in] tool The tool handle.
 * @param[in] user_data User data.
 * @return @c true to continue, @c false to stop.
 */
typedef bool (*voxi_llm_tool_cb)(voxi_llm_tool_h tool,
                                 void* user_data);

/**
 * @brief Creates a tool call handle.
 * @param[out] tool_call The tool call handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tool_call_create(voxi_llm_tool_call_h* tool_call);

/**
 * @brief Destroys a tool call handle.
 * @param[in] tool_call The tool call handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tool_call_destroy(voxi_llm_tool_call_h tool_call);

/**
 * @brief Sets the ID of a tool call.
 * @param[in] tool_call The tool call handle.
 * @param[in] id The ID string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tool_call_set_id(voxi_llm_tool_call_h tool_call,
                              const char* id);

/**
 * @brief Gets the ID of a tool call.
 * @param[in] tool_call The tool call handle.
 * @param[out] id The ID string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tool_call_get_id(voxi_llm_tool_call_h tool_call,
                              char** id);

/**
 * @brief Sets the name of a tool call.
 * @param[in] tool_call The tool call handle.
 * @param[in] name The name string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tool_call_set_name(voxi_llm_tool_call_h tool_call,
                                const char* name);

/**
 * @brief Gets the name of a tool call.
 * @param[in] tool_call The tool call handle.
 * @param[out] name The name string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tool_call_get_name(voxi_llm_tool_call_h tool_call,
                                char** name);

/**
 * @brief Sets the arguments JSON of a tool call.
 * @param[in] tool_call The tool call handle.
 * @param[in] args_json The arguments JSON string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tool_call_set_args_json(voxi_llm_tool_call_h tool_call,
                                     const char* args_json);

/**
 * @brief Gets the arguments JSON of a tool call.
 * @param[in] tool_call The tool call handle.
 * @param[out] args_json The arguments JSON string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tool_call_get_args_json(voxi_llm_tool_call_h tool_call,
                                     char** args_json);

/**
 * @brief Creates a message handle.
 * @param[out] message The message handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_message_create(voxi_llm_message_h* message);

/**
 * @brief Destroys a message handle.
 * @param[in] message The message handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_message_destroy(voxi_llm_message_h message);

/**
 * @brief Sets the role of a message.
 * @param[in] message The message handle.
 * @param[in] role The role string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_message_set_role(voxi_llm_message_h message,
                              const char* role);

/**
 * @brief Gets the role of a message.
 * @param[in] message The message handle.
 * @param[out] role The role string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_message_get_role(voxi_llm_message_h message,
                              char** role);

/**
 * @brief Sets the text of a message.
 * @param[in] message The message handle.
 * @param[in] text The text string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_message_set_text(voxi_llm_message_h message,
                              const char* text);

/**
 * @brief Gets the text of a message.
 * @param[in] message The message handle.
 * @param[out] text The text string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_message_get_text(voxi_llm_message_h message,
                              char** text);

/**
 * @brief Adds a tool call to a message.
 * @param[in] message The message handle.
 * @param[in] tool_call The tool call handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_message_add_tool_call(voxi_llm_message_h message,
                                   voxi_llm_tool_call_h tool_call);

/**
 * @brief Iterates over tool calls of a message.
 * @param[in] message The message handle.
 * @param[in] callback The callback function.
 * @param[in] user_data User data for the callback.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_message_foreach_tool_calls(
    voxi_llm_message_h message, voxi_llm_tool_call_cb callback,
    void* user_data);

/**
 * @brief Sets the tool name of a message (for tool results).
 * @param[in] message The message handle.
 * @param[in] tool_name The tool name string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_message_set_tool_name(voxi_llm_message_h message,
                                   const char* tool_name);

/**
 * @brief Gets the tool name of a message.
 * @param[in] message The message handle.
 * @param[out] tool_name The tool name string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_message_get_tool_name(voxi_llm_message_h message,
                                   char** tool_name);

/**
 * @brief Sets the tool call ID of a message.
 * @param[in] message The message handle.
 * @param[in] tool_call_id The tool call ID string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_message_set_tool_call_id(voxi_llm_message_h message,
                                      const char* tool_call_id);

/**
 * @brief Gets the tool call ID of a message.
 * @param[in] message The message handle.
 * @param[out] tool_call_id The tool call ID string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_message_get_tool_call_id(voxi_llm_message_h message,
                                      char** tool_call_id);

/**
 * @brief Sets the tool result JSON of a message.
 * @param[in] message The message handle.
 * @param[in] tool_result_json The JSON string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_message_set_tool_result_json(voxi_llm_message_h message,
                                          const char* tool_result_json);

/**
 * @brief Gets the tool result JSON of a message.
 * @param[in] message The message handle.
 * @param[out] tool_result_json The JSON string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_message_get_tool_result_json(voxi_llm_message_h message,
                                          char** tool_result_json);

/**
 * @brief Creates a messages list handle.
 * @param[out] messages The messages list handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_messages_create(voxi_llm_messages_h* messages);

/**
 * @brief Destroys a messages list handle.
 * @param[in] messages The messages list handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_messages_destroy(voxi_llm_messages_h messages);

/**
 * @brief Adds a message to the messages list.
 * @param[in] messages The messages list handle.
 * @param[in] message The message handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_messages_add(voxi_llm_messages_h messages,
                          voxi_llm_message_h message);

/**
 * @brief Iterates over messages in a list.
 * @param[in] messages The messages list handle.
 * @param[in] callback The callback function.
 * @param[in] user_data User data for the callback.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_messages_foreach(voxi_llm_messages_h messages,
                              voxi_llm_message_cb callback,
                              void* user_data);

/**
 * @brief Creates a tool handle.
 * @param[out] tool The tool handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tool_create(voxi_llm_tool_h* tool);

/**
 * @brief Destroys a tool handle.
 * @param[in] tool The tool handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tool_destroy(voxi_llm_tool_h tool);

/**
 * @brief Sets the name of a tool.
 * @param[in] tool The tool handle.
 * @param[in] name The name string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tool_set_name(voxi_llm_tool_h tool, const char* name);

/**
 * @brief Gets the name of a tool.
 * @param[in] tool The tool handle.
 * @param[out] name The name string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tool_get_name(voxi_llm_tool_h tool, char** name);

/**
 * @brief Sets the description of a tool.
 * @param[in] tool The tool handle.
 * @param[in] description The description string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tool_set_description(voxi_llm_tool_h tool,
                                  const char* description);

/**
 * @brief Gets the description of a tool.
 * @param[in] tool The tool handle.
 * @param[out] description The description string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tool_get_description(voxi_llm_tool_h tool,
                                  char** description);

/**
 * @brief Sets the parameters JSON of a tool.
 * @param[in] tool The tool handle.
 * @param[in] parameters_json The parameters JSON string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tool_set_parameters_json(voxi_llm_tool_h tool,
                                      const char* parameters_json);

/**
 * @brief Gets the parameters JSON of a tool.
 * @param[in] tool The tool handle.
 * @param[out] parameters_json The parameters JSON string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tool_get_parameters_json(voxi_llm_tool_h tool,
                                      char** parameters_json);

/**
 * @brief Creates a tools list handle.
 * @param[out] tools The tools list handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tools_create(voxi_llm_tools_h* tools);

/**
 * @brief Destroys a tools list handle.
 * @param[in] tools The tools list handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tools_destroy(voxi_llm_tools_h tools);

/**
 * @brief Adds a tool to the tools list.
 * @param[in] tools The tools list handle.
 * @param[in] tool The tool handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tools_add(voxi_llm_tools_h tools,
                       voxi_llm_tool_h tool);

/**
 * @brief Iterates over tools in a list.
 * @param[in] tools The tools list handle.
 * @param[in] callback The callback function.
 * @param[in] user_data User data for the callback.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_tools_foreach(voxi_llm_tools_h tools,
                           voxi_llm_tool_cb callback,
                           void* user_data);

/**
 * @brief Creates a response handle.
 * @param[out] response The response handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_create(voxi_llm_response_h* response);

/**
 * @brief Destroys a response handle.
 * @param[in] response The response handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_destroy(voxi_llm_response_h response);

/**
 * @brief Sets the success flag of a response.
 * @param[in] response The response handle.
 * @param[in] success The success flag.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_set_success(voxi_llm_response_h response,
                                  bool success);

/**
 * @brief Checks if a response is a success.
 * @param[in] response The response handle.
 * @param[out] success The success flag.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_is_success(voxi_llm_response_h response,
                                 bool* success);

/**
 * @brief Sets the text of a response.
 * @param[in] response The response handle.
 * @param[in] text The text string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_set_text(voxi_llm_response_h response,
                               const char* text);

/**
 * @brief Gets the text of a response.
 * @param[in] response The response handle.
 * @param[out] text The text string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_get_text(voxi_llm_response_h response,
                               char** text);

/**
 * @brief Sets the error message of a response.
 * @param[in] response The response handle.
 * @param[in] error_message The error message string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_set_error_message(voxi_llm_response_h response,
                                        const char* error_message);

/**
 * @brief Gets the error message of a response.
 * @param[in] response The response handle.
 * @param[out] error_message The error message string.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_get_error_message(voxi_llm_response_h response,
                                        char** error_message);

/**
 * @brief Adds a tool call to a response.
 * @param[in] response The response handle.
 * @param[in] tool_call The tool call handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_add_llm_tool_call(
    voxi_llm_response_h response, voxi_llm_tool_call_h tool_call);

/**
 * @brief Iterates over tool calls of a response.
 * @param[in] response The response handle.
 * @param[in] callback The callback function.
 * @param[in] user_data User data for the callback.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_foreach_llm_tool_calls(
    voxi_llm_response_h response, voxi_llm_tool_call_cb callback,
    void* user_data);

/**
 * @brief Sets the prompt tokens of a response.
 * @param[in] response The response handle.
 * @param[in] prompt_tokens The prompt tokens count.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_set_prompt_tokens(voxi_llm_response_h response,
                                        int prompt_tokens);

/**
 * @brief Gets the prompt tokens of a response.
 * @param[in] response The response handle.
 * @param[out] prompt_tokens The prompt tokens count.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_get_prompt_tokens(voxi_llm_response_h response,
                                        int* prompt_tokens);

/**
 * @brief Sets the completion tokens of a response.
 * @param[in] response The response handle.
 * @param[in] completion_tokens The completion tokens count.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_set_completion_tokens(
    voxi_llm_response_h response, int completion_tokens);

/**
 * @brief Gets the completion tokens of a response.
 * @param[in] response The response handle.
 * @param[out] completion_tokens The completion tokens count.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_get_completion_tokens(
    voxi_llm_response_h response, int* completion_tokens);

/**
 * @brief Sets the total tokens of a response.
 * @param[in] response The response handle.
 * @param[in] total_tokens The total tokens count.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_set_total_tokens(voxi_llm_response_h response,
                                       int total_tokens);

/**
 * @brief Gets the total tokens of a response.
 * @param[in] response The response handle.
 * @param[out] total_tokens The total tokens count.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_get_total_tokens(voxi_llm_response_h response,
                                       int* total_tokens);

/**
 * @brief Sets the HTTP status of a response.
 * @param[in] response The response handle.
 * @param[in] http_status The HTTP status code.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_set_http_status(voxi_llm_response_h response,
                                      int http_status);

/**
 * @brief Gets the HTTP status of a response.
 * @param[in] response The response handle.
 * @param[out] http_status The HTTP status code.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_llm_response_get_http_status(voxi_llm_response_h response,
                                      int* http_status);

/**
 * @brief Initializes the backend plugin.
 * @param[in] config_json_str The configuration JSON string.
 * @return @c true on success, @c false otherwise.
 */
bool VOXI_LLM_BACKEND_INITIALIZE(const char* config_json_str);

/**
 * @brief Gets the backend plugin name.
 * @return The name string.
 */
const char* VOXI_LLM_BACKEND_GET_NAME(void);

/**
 * @brief Performs a chat request with the backend plugin.
 * @param[in] messages The messages list handle.
 * @param[in] tools The tools list handle.
 * @param[in] on_chunk The callback for text chunks.
 * @param[in] user_data User data for the callback.
 * @param[in] system_prompt The system prompt string.
 * @return The response handle, or @c NULL on error.
 */
voxi_llm_response_h VOXI_LLM_BACKEND_CHAT(
    voxi_llm_messages_h messages, voxi_llm_tools_h tools,
    voxi_llm_backend_chunk_cb on_chunk, void* user_data,
    const char* system_prompt);

/**
 * @brief Shuts down the backend plugin.
 */
void VOXI_LLM_BACKEND_SHUTDOWN(void);

#ifdef __cplusplus
}
#endif

#endif  // VOXI_LLM_BACKEND_H_
