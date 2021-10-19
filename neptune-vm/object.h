#pragma once
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <ostream>
#include <tsl/robin_map.h>
#include <vector>

#define UNUSED(x) (void)(x)

namespace neptune_vm {
template <typename T> size_t size(T *t) {
  UNUSED(t);
  return sizeof(T);
}
struct StringSlice {
  const char *data;
  size_t len;
};
enum class Type : uint8_t {
  String,
  Symbol,
  Array,
  Map,
  FunctionInfo,
  Function,
  UpValue
};

class Object {
  bool is_dark;
  Object *next; // part of intrusive linked list contained in GC
  friend class VM;

public:
  Type type;
  template <typename O> bool is() const;
  template <typename O> O *as();
  const char *type_string() const;
  friend std::ostream &operator<<(std::ostream &os, Object &o);
};

class String : public Object {
  size_t len;
  char data[];
  template <typename T> friend size_t size(T *t);

public:
  static constexpr Type type = Type::String;
  static String *from(StringSlice s);
  static String *from(const std::string &s);
  operator StringSlice() const;
  String *concat(String *s);
};

template <> size_t size(String *s);

class Symbol : public Object {
  size_t len;
  uint32_t hash;
  char data[];
  friend class VM;
  friend struct StringHasher;
  template <typename T> friend size_t size(T *t);

public:
  static constexpr Type type = Type::Symbol;
  operator StringSlice() const;
};

template <> size_t size(Symbol *s);

struct StringEquality {
  using is_transparent = void;
  bool operator()(Symbol *sym, StringSlice s) const {
    return StringEquality{}(static_cast<StringSlice>(*sym), s);
  }
  bool operator()(StringSlice s, Symbol *sym) const {
    return StringEquality{}(s, static_cast<StringSlice>(*sym));
  }
  bool operator()(Symbol *sym1, Symbol *sym2) const { return sym1 == sym2; }
  bool operator()(StringSlice s1, StringSlice s2) const {
    return s1.len == s2.len && memcmp(s1.data, s2.data, s1.len) == 0;
  }
  bool operator()(String *s1, String *s2) const {
    return StringEquality{}(static_cast<StringSlice>(*s1),
                            static_cast<StringSlice>(*s2));
  }
};

struct StringHasher {
  uint32_t operator()(StringSlice s) const;
  uint32_t operator()(Symbol *sym) const;
};

class Array : public Object {
public:
  Array() = default;
  explicit Array(size_t size);
  std::vector<Value> inner;
  static constexpr Type type = Type::Array;
};

template <typename T>
using ValueMap = tsl::robin_map<Value, T, ValueHasher, ValueStrictEquality,
                                std::allocator<std::pair<Value, Value>>, true>;

class Map : public Object {
public:
  Map() = default;
  explicit Map(size_t size);
  ValueMap<Value> inner;
  static constexpr Type type = Type::Map;
};
} // namespace neptune_vm
