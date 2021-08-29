#pragma once
#include "neptune-vm.h"
#include <cstddef>
#include <cstdint>
#include <cstring>
namespace neptune_vm {
struct StringSlice {
  char *data;
  size_t len;
};
enum class Type : uint8_t { String, Symbol };

class Object {
  Type type;
  bool is_dark;
  Object *next; // part of intrusive linked list contained in GC
  friend class VM;

public:
  template <typename O> bool is();
  template <typename O> O *as();
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
  friend struct StringEquality;
  friend struct StringHasher;
  friend class VM;

public:
  static constexpr Type type = Type::Symbol;
};

struct StringEquality {
  using is_transparent = void;
  bool operator()(Symbol *const sym, StringSlice s) const {
    return sym->len == s.len && memcmp(sym->data, s.data, s.len) == 0;
  }
  bool operator()(StringSlice s, Symbol *const sym) const {
    return sym->len == s.len && memcmp(sym->data, s.data, s.len) == 0;
  }
  bool operator()(Symbol *const sym1, Symbol *const sym2) const {
    return sym1->len == sym2->len &&
           memcmp(sym1->data, sym2->data, sym1->len) == 0;
  }
};

struct StringHasher {
  size_t operator()(StringSlice s) const;
  size_t operator()(Symbol *sym) const;
};
} // namespace neptune_vm
