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

#ifndef VOXI_CURL_H_
#define VOXI_CURL_H_

#include <voxi_error.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief The Voxi Curl handle.
 */
typedef void* voxi_curl_h;

/**
 * @brief Called when a chunk of data is received via Curl.
 * @param[in] chunk The data chunk.
 * @param[in] user_data The user data passed from the request function.
 */
typedef void (*voxi_curl_chunk_cb)(const char* chunk, void* user_data);

/**
 * @brief Creates a Voxi Curl handle.
 * @param[out] curl The Voxi Curl handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 * @retval #VOXI_ERROR_IO_ERROR Internal error
 */
int voxi_curl_create(voxi_curl_h* curl);

/**
 * @brief Destroys the Voxi Curl handle.
 * @param[in] curl The Voxi Curl handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_curl_destroy(voxi_curl_h curl);

/**
 * @brief Sets the URL for the Curl request.
 * @param[in] curl The Voxi Curl handle.
 * @param[in] url The URL string to request.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 * @retval #VOXI_ERROR_IO_ERROR Internal error
 */
int voxi_curl_set_url(voxi_curl_h curl, const char* url);

/**
 * @brief Adds an HTTP header to the Curl request.
 * @param[in] curl The Voxi Curl handle.
 * @param[in] header The complete HTTP header line to append.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_curl_add_header(voxi_curl_h curl, const char* header);

/**
 * @brief Sets the POST data for the Curl request.
 * @param[in] curl The Voxi Curl handle.
 * @param[in] data The string data to post.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 * @retval #VOXI_ERROR_IO_ERROR Internal error
 */
int voxi_curl_set_post_data(voxi_curl_h curl, const char* data);

/**
 * @brief Sets the method of the Curl request to GET.
 * @param[in] curl The Voxi Curl handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 * @retval #VOXI_ERROR_IO_ERROR Internal error
 */
int voxi_curl_set_method_get(voxi_curl_h curl);

/**
 * @brief Sets timeout limits on a Curl request.
 * @param[in] curl The Voxi Curl handle.
 * @param[in] connect_timeout Timeout in seconds for the connection phase.
 * @param[in] request_timeout Total timeout in seconds for the request.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_curl_set_timeout(voxi_curl_h curl, long connect_timeout, long request_timeout);

/**
 * @brief Sets the write callback to process received chunks.
 * @param[in] curl The Voxi Curl handle.
 * @param[in] callback Callback function pointer.
 * @param[in] user_data User data passed to the callback.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_curl_set_write_callback(
    voxi_curl_h curl, voxi_curl_chunk_cb callback, void* user_data);

/**
 * @brief Performs the actual Curl request.
 * @param[in] curl The Voxi Curl handle.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 * @retval #VOXI_ERROR_IO_ERROR Internal error
 */
int voxi_curl_perform(voxi_curl_h curl);

/**
 * @brief Gets the HTTP response code from the completed Curl request.
 * @param[in] curl The Voxi Curl handle.
 * @param[out] code Pointer to store the HTTP response code.
 * @return @c 0 on success, otherwise a negative error value.
 * @retval #VOXI_ERROR_NONE Successful
 * @retval #VOXI_ERROR_INVALID_PARAMETER Invalid parameter
 */
int voxi_curl_get_response_code(voxi_curl_h curl, long* code);

/**
 * @brief Gets a human-readable error message if a Curl request failed.
 * @param[in] curl The Voxi Curl handle.
 * @return The error message string, or "Unknown or no error".
 */
const char* voxi_curl_get_error_message(voxi_curl_h curl);

#ifdef __cplusplus
}
#endif

#endif  // VOXI_CURL_H_
