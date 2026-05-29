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

#ifndef API_VOXI_ERROR_H_
#define API_VOXI_ERROR_H_

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Enumeration for Voxi error codes.
 */
typedef enum {
  VOXI_ERROR_NONE = 0,                   /**< Successful */
  VOXI_ERROR_INVALID_PARAMETER = -1,     /**< Invalid parameter */
  VOXI_ERROR_OUT_OF_MEMORY = -2,         /**< Out of memory */
  VOXI_ERROR_CONNECTION_REFUSED = -3,     /**< Connection refused */
  VOXI_ERROR_IO_ERROR = -4,              /**< I/O error */
  VOXI_ERROR_NOT_SUPPORTED = -5,         /**< Not supported */
  VOXI_ERROR_COMMUNICATION_FAILED = -6,  /**< Communication failed */
} voxi_error_e;

#ifdef __cplusplus
}
#endif

#endif  /* API_VOXI_ERROR_H_ */
