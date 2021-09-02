#include "neptune-vm.h"
#include <string>

#ifdef __GNUC__ // gcc or clang
[[noreturn]] void unreachable() { __builtin_unreachable(); }
#elif defined(_MSC_VER) // MSVC
[[noreturn]] void unreachable() { __assume(false); }
#else
[[noreturn]] void unreachable() { abort(); }
#endif

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
