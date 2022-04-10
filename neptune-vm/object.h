#pragma once
#include "hash_table.h"
#include "rust/cxx.h"
#include "util.h"
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <ostream>
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
  explicit StringSlice(const char *data, size_t len) : data(data), len(len) {}
  StringSlice(const char *cstring) : data(cstring), len(strlen(cstring)) {}
  StringSlice(const std::string &s) : data(s.data()), len(s.size()) {}
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
  Instance,
  Range,
  ArrayIterator,
  MapIterator,
  StringIterator,
  Channel
};
class Class;
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
  void *operator new(size_t size) { return alloc(size); }
  void operator delete(void *p) { dealloc(p); }
};

class VM;
class String : public Object {
  size_t len;
  char data[];
  template <typename T> friend size_t size(T *t);

public:
  String() = delete;
  size_t get_len() { return len; }
  static size_t find(String *haystack, String *needle, size_t start);
  String *replace(VM *vm, String *from, String *to);
  static constexpr Type type = Type::String;
  operator StringSlice() const;
  operator rust::String() const;
  friend class VM;
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
  Symbol() = delete;
  static constexpr Type type = Type::Symbol;
  operator StringSlice() const;
};

template <> size_t size(Symbol *s);

struct StringEquality {
  using is_transparent = void;
  bool operator()(Symbol *sym, StringSlice s) const {
    return sym != nullptr &&
           StringEquality{}(static_cast<StringSlice>(*sym), s);
  }
  bool operator()(StringSlice s, Symbol *sym) const {
    return sym != nullptr &&
           StringEquality{}(s, static_cast<StringSlice>(*sym));
  }
  bool operator()(Symbol *sym1, Symbol *sym2) const { return sym1 == sym2; }
  bool operator()(StringSlice s1, StringSlice s2) const {
    return s1.len == s2.len && memcmp(s1.data, s2.data, s1.len) == 0;
  }
  bool operator()(String *s1, String *s2) const {
    if (s1 == nullptr || s2 == nullptr)
      return s1 == s2;
    return StringEquality{}(static_cast<StringSlice>(*s1),
                            static_cast<StringSlice>(*s2));
  }
  bool operator()(String *s1, StringSlice s2) const {
    return s1 != nullptr && StringEquality{}(static_cast<StringSlice>(*s1), s2);
  }
  bool operator()(StringSlice s1, String *s2) const {
    return s2 != nullptr && StringEquality{}(s1, static_cast<StringSlice>(*s2));
  }
};

struct StringHasher {
  uint32_t operator()(StringSlice s) const;
  uint32_t operator()(Symbol *sym) const;
  uint32_t operator()(String *s) const;
};

class Array : public Object {
public:
  Array() = default;
  explicit Array(size_t size);
  Array(size_t size, Value v);
  vector<Value> inner;
  static constexpr Type type = Type::Array;
};

struct ValueEmpty {
  bool is_empty(Value v);
  Value empty();
};

template <typename T>
using ValueMap =
    HashMap<Value, T, ValueHasher, ValueStrictEquality, ValueEmpty>;

template <typename T>
using SymbolMap =
    HashMap<Symbol *, T, StringHasher, StringEquality, NullptrEmpty<Symbol>>;

class Map : public Object {
public:
  Map() = default;
  explicit Map(uint32_t size);
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
  explicit Module(const std::string &name) : name(name) {}
  static constexpr Type type = Type::Module;
  friend class VM;
};
class FunctionInfoWriter;
class VM;
class Function;
class Class : public Object {
  SymbolMap<Object *> methods;

public:
  bool is_native = false;
  std::string name;
  Class *super;
  Object *find_method(Symbol *method);
  void copy_methods(Class &other);
  static constexpr Type type = Type::Class;
  friend class VM;
  friend class FunctionInfoWriter;
};

class Instance : public Object {
public:
  Class *class_;
  SymbolMap<Value> properties;
  Instance() = default;
  explicit Instance(size_t size);
  static constexpr Type type = Type::Instance;
  friend class VM;
};

class Range : public Object {
public:
  int32_t start;
  int32_t end;
  Range(int32_t start, int32_t end) : start(start), end(end) {}
  static constexpr Type type = Type::Range;
};

class ArrayIterator : public Object {
public:
  Array *array;
  size_t position;
  explicit ArrayIterator(Array *array) : array(array), position(0) {}
  static constexpr Type type = Type::ArrayIterator;
};

class MapIterator : public Object {
public:
  Map *map;
  Value last_key;
  explicit MapIterator(Map *map);
  static constexpr Type type = Type::MapIterator;
};

class StringIterator : public Object {
public:
  String *string;
  size_t position;
  explicit StringIterator(String *string) : string(string), position(0) {}
  static constexpr Type type = Type::StringIterator;
};

struct BuiltinClasses {
  Class *Object, *Class_, *Int, *Float, *Bool, *Null, *String, *Symbol, *Array,
      *Map, *Function, *Module, *Task, *Range, *ArrayIterator, *MapIterator,
      *StringIterator, *Channel;
  BuiltinClasses() {
    Object = Class_ = Int = Float = Bool = Null = String = Symbol = Array =
        Map = Function = Module = Task = Range = ArrayIterator = MapIterator =
            StringIterator = Channel = nullptr;
  }
};
struct BuiltinSymbols {
  Symbol *construct, *message, *stack, *task, *finished, *running, *killed;
  BuiltinSymbols() {
    construct = message = stack = task = finished = running = killed = nullptr;
  }
};
} // namespace neptune_vm
