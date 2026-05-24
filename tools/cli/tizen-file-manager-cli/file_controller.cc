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

#include "file_controller.hh"

#include <dirent.h>
#include <sys/stat.h>
#include <unistd.h>

#include <cerrno>
#include <cstring>
#include <fstream>
#include <sstream>

namespace {

std::string JsonEscape(const std::string& s) {
  std::string out;
  out.reserve(s.size());
  for (char c : s) {
    switch (c) {
      case '"':  out += "\\\""; break;
      case '\\': out += "\\\\"; break;
      case '\n': out += "\\n";  break;
      case '\r': out += "\\r";  break;
      case '\t': out += "\\t";  break;
      default:   out += c;      break;
    }
  }

  return out;
}

std::string DecodeEscapedText(const std::string& s) {
  std::string out;
  out.reserve(s.size());

  for (size_t i = 0; i < s.size(); ++i) {
    char c = s[i];
    if (c != '\\' || i + 1 >= s.size()) {
      out += c;
      continue;
    }

    char next = s[++i];
    switch (next) {
      case 'n':
        out += '\n';
        break;
      case 'r':
        out += '\r';
        break;
      case 't':
        out += '\t';
        break;
      case '\\':
        out += '\\';
        break;
      case '"':
        out += '"';
        break;
      default:
        out += '\\';
        out += next;
        break;
    }
  }

  return out;
}

std::string ErrorJson(const std::string& msg) {
  return "{\"error\": \"" + JsonEscape(msg) + "\"}";
}

std::string SuccessJson(const std::string& msg) {
  return "{\"status\": \"success\", \"message\": \""
         + JsonEscape(msg) + "\"}";
}

}  // namespace

namespace tizenclaw {
namespace cli {

std::string FileController::Read(
    const std::string& path) {
  std::ifstream f(path, std::ios::binary);
  if (!f.is_open()) {
    return ErrorJson(
        "Cannot open: " + std::string(strerror(errno)));
  }

  std::ostringstream ss;
  ss << f.rdbuf();
  std::string content = ss.str();

  return "{\"path\": \"" + JsonEscape(path)
       + "\", \"size\": "
       + std::to_string(content.size())
       + ", \"content\": \""
       + JsonEscape(content) + "\"}";
}

std::string FileController::Write(
    const std::string& path,
    const std::string& content) {
  const std::string decoded = DecodeEscapedText(content);
  std::ofstream f(path,
      std::ios::binary | std::ios::trunc);
  if (!f.is_open()) {
    return ErrorJson(
        "Cannot write: "
        + std::string(strerror(errno)));
  }

  f << decoded;
  f.close();

  return "{\"status\": \"success\", \"path\": \""
       + JsonEscape(path) + "\", \"bytes_written\": "
       + std::to_string(decoded.size()) + "}";
}

std::string FileController::Append(
    const std::string& path,
    const std::string& content) {
  const std::string decoded = DecodeEscapedText(content);
  std::ofstream f(path,
      std::ios::binary | std::ios::app);
  if (!f.is_open()) {
    return ErrorJson(
        "Cannot append: "
        + std::string(strerror(errno)));
  }

  f << decoded;
  f.close();

  return "{\"status\": \"success\", \"path\": \""
       + JsonEscape(path)
       + "\", \"bytes_appended\": "
       + std::to_string(decoded.size()) + "}";
}

std::string FileController::Remove(
    const std::string& path) {
  if (::remove(path.c_str()) != 0) {
    return ErrorJson(
        "Cannot remove: "
        + std::string(strerror(errno)));
  }

  return SuccessJson("Removed " + path);
}

std::string FileController::Mkdir(
    const std::string& path) {
  if (::mkdir(path.c_str(), 0755) != 0) {
    if (errno != EEXIST) {
      return ErrorJson(
          "Cannot mkdir: "
          + std::string(strerror(errno)));
    }
  }

  return SuccessJson("Directory created: " + path);
}

std::string FileController::List(
    const std::string& path) {
  DIR* dir = opendir(path.c_str());
  if (!dir) {
    return ErrorJson(
        "Cannot open dir: "
        + std::string(strerror(errno)));
  }

  std::ostringstream ss;
  ss << "{\"path\": \"" << JsonEscape(path)
     << "\", \"entries\": [";

  struct dirent* entry;
  bool first = true;
  int count = 0;

  while ((entry = readdir(dir)) != nullptr) {
    std::string name = entry->d_name;
    if (name == "." || name == "..")
      continue;

    if (!first) ss << ", ";

    first = false;

    std::string full =
        path + "/" + name;
    struct stat st;
    std::string type = "unknown";

    if (::stat(full.c_str(), &st) == 0) {
      if (S_ISDIR(st.st_mode))
        type = "directory";
      else if (S_ISREG(st.st_mode))
        type = "file";
      else if (S_ISLNK(st.st_mode))
        type = "symlink";
    }

    ss << "{\"name\": \"" << JsonEscape(name)
       << "\", \"type\": \"" << type << "\"";

    if (type == "file") {
      ss << ", \"size\": " << st.st_size;
    }

    ss << "}";
    ++count;
  }

  closedir(dir);
  ss << "], \"count\": " << count << "}";

  return ss.str();
}

std::string FileController::Stat(
    const std::string& path) {
  struct stat st;
  if (::stat(path.c_str(), &st) != 0) {
    return ErrorJson(
        "Cannot stat: "
        + std::string(strerror(errno)));
  }

  std::string type = "unknown";
  if (S_ISDIR(st.st_mode)) type = "directory";
  else if (S_ISREG(st.st_mode)) type = "file";
  else if (S_ISLNK(st.st_mode)) type = "symlink";

  std::ostringstream ss;
  ss << "{\"path\": \"" << JsonEscape(path)
     << "\", \"type\": \"" << type
     << "\", \"size\": " << st.st_size
     << ", \"mode\": " << (st.st_mode & 0777)
     << ", \"uid\": " << st.st_uid
     << ", \"gid\": " << st.st_gid
     << ", \"mtime\": " << st.st_mtime
     << "}";

  return ss.str();
}

std::string FileController::Copy(
    const std::string& src,
    const std::string& dst) {
  std::ifstream in(src, std::ios::binary);
  if (!in.is_open()) {
    return ErrorJson(
        "Cannot open source: "
        + std::string(strerror(errno)));
  }

  std::ofstream out(dst,
      std::ios::binary | std::ios::trunc);
  if (!out.is_open()) {
    return ErrorJson(
        "Cannot open dest: "
        + std::string(strerror(errno)));
  }

  out << in.rdbuf();
  in.close();
  out.close();

  return SuccessJson(
      "Copied " + src + " -> " + dst);
}

std::string FileController::Move(
    const std::string& src,
    const std::string& dst) {
  if (::rename(src.c_str(), dst.c_str()) != 0) {
    return ErrorJson(
        "Cannot move: "
        + std::string(strerror(errno)));
  }

  return SuccessJson(
      "Moved " + src + " -> " + dst);
}

std::string FileController::Download(
    const std::string& url,
    const std::string& dest) {
  if (url.empty())
    return ErrorJson("URL is required");
  if (dest.empty())
    return ErrorJson("Destination is required");

  // Use curl to download
  std::string cmd = "curl -fsSL -o '" + dest +
                    "' '" + url + "' 2>&1";
  FILE* pipe = popen(cmd.c_str(), "r");
  if (!pipe)
    return ErrorJson("Failed to execute curl");

  std::string output;
  char buf[1024];
  while (fgets(buf, sizeof(buf), pipe))
    output += buf;
  int status = pclose(pipe);

  if (status != 0) {
    while (!output.empty() &&
           output.back() == '\n')
      output.pop_back();
    return ErrorJson(
        "Download failed: " + output);
  }

  // Verify file exists and get size
  struct stat st;
  if (::stat(dest.c_str(), &st) != 0)
    return ErrorJson("Downloaded file not found");

  return "{\"status\": \"success\", \"url\": \"" +
         JsonEscape(url) +
         "\", \"file_path\": \"" +
         JsonEscape(dest) +
         "\", \"size_bytes\": " +
         std::to_string(st.st_size) + "}";
}

}  // namespace cli
}  // namespace tizenclaw
