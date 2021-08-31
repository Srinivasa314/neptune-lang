#pragma once
#include "object.h"

#ifdef __GNUC__ // gcc or clang
[[noreturn]] static void unreachable() { __builtin_unreachable(); }
#elif defined(_MSC_VER) // MSVC
[[noreturn]] static void unreachable() { __assume(false); }
#else
[[noreturn]] static void unreachable() { abort(); }
#endif

template <typename T> static T read(const uint8_t *&bytecode) {
  T ret;
  memcpy(&ret, bytecode, sizeof(T));
  bytecode++;
  return ret;
}

#define TODO()                                                                 \
  std::cout << "TODO at: " << __FILE__ << " : " << __LINE__ << std::endl;      \
  exit(1)

#define READ(type) read<type>(ip)

std::string escaped_string(neptune_vm::StringSlice s) {
  std::string str = "\"";
  for (auto c = s.data; c != s.data + s.len; c++) {
    switch (*c) {
    case '\n':
      str += "\\n";
      break;
    case '\r':
      str += "\\r";
      break;
    case '\t':
      str += "\\t";
      break;
    case '\\':
      str += "\\\\";
      break;
    case '"':
      str += "\\\"";
      break;
    case '\0':
      str += "\\0";
      break;
    default:
      str += c;
    }
  }
  str += '\"';
  return str;
}
