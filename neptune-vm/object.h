#pragma once
#include "neptune-vm.h"
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <ostream>

namespace neptune_vm {
struct StringSlice {
  const char *data;
  size_t len;
};
enum class Type : uint8_t { String, Symbol };

class Object {
  Type type;
  bool is_dark;
  Object *next; // part of intrusive linked list contained in GC
  friend class VM;

public:
  template <typename O> bool is() const;
  template <typename O> O *as();
  const char *type_string() const;
  friend std::ostream &operator<<(std::ostream &os, Object &o);
};

class String : public Object {
  size_t len;
  char data[];

public:
  static constexpr Type type = Type::String;
  static String *from_string_slice(StringSlice s);
  explicit operator StringSlice();
};

class Symbol : public Object {
  size_t len;
  char data[];
  friend class VM;

public:
  static constexpr Type type = Type::Symbol;
  explicit operator StringSlice();
};

struct StringEquality {
  using is_transparent = void;
  bool operator()(Symbol *const sym, StringSlice s) const {
    return StringEquality{}(static_cast<StringSlice>(*sym), s);
  }
  bool operator()(StringSlice s, Symbol *const sym) const {
    return StringEquality{}(s, static_cast<StringSlice>(*sym));
  }
  bool operator()(Symbol *sym1, Symbol *sym2) const {
    return StringEquality{}(static_cast<StringSlice>(*sym1),
                            static_cast<StringSlice>(*sym2));
  }
  bool operator()(StringSlice s1, StringSlice s2) const {
    return s1.len == s2.len && memcmp(s1.data, s2.data, s1.len) == 0;
  }
  bool operator()(String *s1, String *s2) const {
    return StringEquality{}(static_cast<StringSlice>(*s1),
                            static_cast<StringSlice>(*s2));
  }
};

struct StringHasher {
  size_t operator()(StringSlice s) const;
  size_t operator()(Symbol *sym) const;
};
} // namespace neptune_vm
