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
String::operator StringSlice() const { return StringSlice{data, len}; }
Symbol::operator StringSlice() const { return StringSlice{data, len}; }
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

uint32_t StringHasher::operator()(StringSlice s) const {
  // FNV-1a hash. http://www.isthe.com/chongo/tech/comp/fnv/
  uint32_t hash = 2166136261U;
  constexpr uint32_t prime = 16777619U;

  for (size_t i = 0; i < s.len; i++) {
    hash ^= static_cast<uint32_t>(s.data[i]);
    hash *= prime;
  }

  return hash;
}

uint32_t StringHasher::operator()(Symbol *sym) const { return sym->hash; }

const char *Object::type_string() const {
  // todo change this when more types are added
  switch (type) {
  case Type::String:
    return "string";
  case Type::Symbol:
    return "symbol";
  case Type::Array:
    return "array";
  case Type::Map:
    return "map";
  default:
    unreachable();
  }
}

static std::string escaped_string(neptune_vm::StringSlice s) {
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
      str += *c;
    }
  }
  str += '\"';
  return str;
}

std::ostream &operator<<(std::ostream &os, Object &o) {
  // todo change this when more types are added
  switch (o.type) {
  case Type::String:
    return os << escaped_string(static_cast<StringSlice>(*o.as<String>()));
  case Type::Symbol: {
    os << '@';
    auto s = static_cast<StringSlice>(*o.as<Symbol>());
    return os.write(s.data, static_cast<std::streamsize>(s.len));
  }
  case Type::Array:
    return os << "[ Array @ " << static_cast<void *>(&o) << " ]";
  case Type::Map:
    return os << "[ Map @ " << static_cast<void *>(&o) << " ]";
  default:
    unreachable();
  }
}
Array::Array(size_t size) : inner(std::vector<Value>(size, Value::null())) {}

Map::Map(size_t size) { inner.reserve(size); }
} // namespace neptune_vm
