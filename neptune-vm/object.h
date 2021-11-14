#pragma once
#include "rust/cxx.h"
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
  explicit StringSlice(const char *data_, size_t len_)
      : data(data_), len(len_) {}
  explicit StringSlice(const char *cstring)
      : data(cstring), len(strlen(cstring)) {}
};

std::ostream &operator<<(std::ostream &os, StringSlice s);

enum class Type : uint8_t {
  String,
  Symbol,
  Array,
  Map,
  FunctionInfo,
  Function,
  UpValue,
  NativeFunction,
  Module,
  Class,
  Task,
  Instance
};
class Class;
class Object {
  bool is_dark;
  Object *next; // part of intrusive linked list contained in GC
  friend class VM;

public:
  Type type;
  Class *class_;
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
  operator rust::String() const;
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
  bool operator()(const std::string &s1, const std::string &s2) const {
    return s1 == s2;
  }
  bool operator()(const std::string &s1, StringSlice s2) const {
    return StringEquality{}(StringSlice(s1.data(), s1.size()), s2);
  }
  bool operator()(StringSlice s1, const std::string &s2) const {
    return StringEquality{}(s2, s1);
  }
};

struct StringHasher {
  uint32_t operator()(StringSlice s) const;
  uint32_t operator()(Symbol *sym) const;
  uint32_t operator()(const std::string &s) const;
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

template <typename T>
using SymbolMap = tsl::robin_map<Symbol *, T, StringHasher, StringEquality>;

class Map : public Object {
public:
  Map() = default;
  explicit Map(size_t size);
  ValueMap<Value> inner;
  static constexpr Type type = Type::Map;
};

struct ModuleVariable {
  uint32_t position;
  bool mutable_;
  bool exported;
};

class Module : public Object {
  SymbolMap<ModuleVariable> module_variables;

public:
  std::string name;
  Module(std::string name_) : name(name_) {}
  static constexpr Type type = Type::Module;
  friend class VM;
};

class Class : public Object {
public:
  SymbolMap<Object *> methods;
  std::string name;
  Class *super;
  static constexpr Type type = Type::Class;
  friend class VM;
};

class Instance : public Object {
public:
  SymbolMap<Value> properties;
  Instance() = default;
  explicit Instance(size_t size);
  static constexpr Type type = Type::Instance;
  friend class VM;
};
struct BuiltinClasses {
  Class *Object, *Class_, *Int, *Float, *Bool, *Null, *String, *Symbol, *Array,
      *Map, *Function, *Module, *Task;
};
} // namespace neptune_vm