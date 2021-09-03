#pragma once
#include "object.h"

[[noreturn]] void unreachable();

template <typename T> static T read(const uint8_t *&bytecode) {
  T ret;
  memcpy(&ret, bytecode, sizeof(T));
  bytecode++;
  return ret;
}

#define TODO()                                                                 \
  std::cout << "TODO at: " << __FILE__ << ":" << __LINE__ << std::endl;        \
  exit(1)

#define READ(type) read<type>(ip)

std::string escaped_string(neptune_vm::StringSlice s);
