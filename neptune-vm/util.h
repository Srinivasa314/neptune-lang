#pragma once
#include <string>

#ifdef __GNUC__ // gcc or clang
#define unreachable() __builtin_unreachable()
#elif defined(_MSC_VER) // MSVC
#define unreachable() __assume(false)
#else
#define unreachable() abort()
#endif

template <typename T> static T read(const uint8_t *&bytecode) {
  T ret;
  memcpy(&ret, bytecode, sizeof(T));
  bytecode+=sizeof(T);
  return ret;
}

#define TODO()                                                                 \
  std::cout << "TODO at: " << __FILE__ << ":" << __LINE__ << std::endl;        \
  exit(1)

#define READ(type) read<type>(ip)
