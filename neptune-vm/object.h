#pragma once
#include <cstddef>
#include <cstdint>

namespace neptune_vm {
struct StringSlice;
enum class Type : uint8_t { String, Symbol };

class Object {
  Type type;
  bool is_dark;
  Object *next; // part of intrusive linked list contained in GC
  friend class GC;
};

class String : public Object {
  size_t len;
  char data[];

public:
  static constexpr Type type = Type::String;
  static String *from_string_slice(StringSlice s);
};

class Symbol : public Object {
  size_t len;
  char data[];

public:
  static constexpr Type type = Type::Symbol;
  static Symbol *from_string_slice(StringSlice s);
};
} // namespace neptune_vm
