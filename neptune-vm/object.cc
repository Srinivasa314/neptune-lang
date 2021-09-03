#include "neptune-vm.h"
#include <cstring>
#include <new>

namespace neptune_vm {
String *String::from_string_slice(StringSlice s) {
  String *p = static_cast<String *>(malloc(sizeof(String) + s.len));
  if (p == nullptr) {
    throw std::bad_alloc();
  }
  memcpy(p->data, s.data, s.len);
  p->len = s.len;
  return p;
}
String::operator StringSlice() { return StringSlice{data, len}; }
Symbol::operator StringSlice() { return StringSlice{data, len}; }
template <typename O> bool Object::is() const { return type == O::type; }
template <typename O> O *Object::as() {
  assert(is<O>());
  return reinterpret_cast<O *>(this);
}

String *String::concat(String *s) {
  String *p = static_cast<String *>(malloc(sizeof(String) + len + s->len));
  if (p == nullptr) {
    throw std::bad_alloc();
  }
  memcpy(p->data, data, len);
  memcpy(p->data + len, s->data, s->len);
  p->len = len + s->len;
  return p;
}

size_t StringHasher::operator()(StringSlice s) const {
// FNV-1a hash. http://www.isthe.com/chongo/tech/comp/fnv/
#if SIZE_MAX == 4294967295
  size_t hash = 2166136261U;
  size_t prime = 16777619U;
#else
  size_t hash = 14695981039346656037U;
  size_t prime = 1099511628211U;
#endif

  for (size_t i = 0; i < s.len; i++) {
    hash ^= static_cast<size_t>(s.data[i]);
    hash *= prime;
  }

  return hash;
}

size_t StringHasher::operator()(Symbol *sym) const {
  return StringHasher{}(static_cast<StringSlice>(*sym));
}

const char *Object::type_string() const {
  // todo change this when more types are added
  switch (type) {
  case Type::String:
    return "string";
  case Type::Symbol:
    return "symbol";
  case Type::Array:
    return "array";
  default:
    unreachable();
  }
}
std::ostream &operator<<(std::ostream &os, Object &o) {
  // todo change this when more types are added
  switch (o.type) {
  case Type::String:
    return os << escaped_string(static_cast<StringSlice>(*o.as<String>()));
  case Type::Symbol: {
    os << '@';
    auto s = static_cast<StringSlice>(*o.as<Symbol>());
    return os.write(s.data, s.len);
  }
  case Type::Array: {
    return os << "Array@" << (void *)&o;
  }
  default:
    unreachable();
  }
}
Array::Array(size_t size) : inner(std::vector<Value>(size, Value::null())) {}

} // namespace neptune_vm
