#include "neptune-vm.h"
#include <cstring>
#include <new>

namespace neptune_vm {
template <> size_t size(String *s) { return sizeof(String) + s->len; }
template <> size_t size(Symbol *s) { return sizeof(Symbol) + s->len; }
template <> size_t size(Function *f) {
  return sizeof(Function) + f->num_upvalues * sizeof(UpValue *);
}

String *String::from(StringSlice s) {
  String *p = static_cast<String *>(malloc(sizeof(String) + s.len));
  if (p == nullptr) {
    throw std::bad_alloc();
  }
  memcpy(p->data, s.data, s.len);
  p->len = s.len;
  return p;
}
String *String::from(const std::string &s) {
  return from(StringSlice{s.data(), s.length()});
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

String::operator rust::String() const { return rust::String(data, len); }

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

uint32_t StringHasher::operator()(const std::string &s) const {
  return StringHasher{}(StringSlice(s.data(), s.length()));
}

const char *Object::type_string() const {
  // todo change this when more types are added
  switch (type) {
  case Type::FunctionInfo:
    return "<internal type FunctionInfo>";
  case Type::UpValue:
    return "<internal type UpValue>";
  default:
    return class_->name.c_str();
  }
}

static std::string escaped_string(neptune_vm::StringSlice s) {
  std::string str = "'";
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
    case '\'':
      str += "\\'";
      break;
    case '\0':
      str += "\\0";
      break;
    default:
      str += *c;
    }
  }
  str += '\'';
  return str;
}

void operator<<(ValueFormatter vf, Object *obj) {
  constexpr uint32_t MAX_DEPTH = 10;
  // todo change this when more types are added
  switch (obj->type) {
  case Type::String:
    vf.os << escaped_string(*obj->as<String>());
    break;
  case Type::Symbol: {
    vf.os << '@';
    auto s = static_cast<StringSlice>(*obj->as<Symbol>());
    vf.os << s;
    break;
  }
  case Type::Array: {
    if (vf.depth > MAX_DEPTH) {
      vf.os << "[ ... ]";
    } else {
      auto new_vf = vf.inc_depth();
      auto &o = obj->as<Array>()->inner;
      auto it = o.begin();
      if (it != o.end()) {
        vf.os << "[ ";

        new_vf << *it;
        it++;
        for (auto v = it; v != o.end(); v++) {
          new_vf.os << ", ";
          new_vf << *v;
        }
        vf.os << " ]";
      } else {
        vf.os << "[]";
      }
    }
    break;
  }
  case Type::Map: {
    if (vf.depth > MAX_DEPTH) {
      vf.os << "Map { ... }";
    } else {
      auto new_vf = vf.inc_depth();
      auto &o = obj->as<Map>()->inner;
      auto it = o.begin();
      if (it != o.end()) {
        vf.os << "Map { ";
        new_vf << it->first;
        new_vf.os << ": ";
        new_vf << it->second;
        it++;
        for (auto p = it; p != o.end(); p++) {
          new_vf.os << ", ";
          new_vf << p->first;
          new_vf.os << ": ";
          new_vf << p->second;
        }
        vf.os << " }";
      } else {
        vf.os << "Map {}";
      }
    }
    break;
  }
  case Type::FunctionInfo:
    vf.os << "<FunctionInfo for " << obj->as<FunctionInfo>()->name << '>';
    break;
  case Type::Function:
    vf.os << "<Function " << obj->as<Function>()->function_info->name << '>';
    break;
  case Type::UpValue:
    vf.os << "<UpValue>";
    break;
  case Type::NativeFunction:
    vf.os << "<Function " << obj->as<NativeFunction>()->name << '>';
    break;
  case Type::Module:
    vf.os << "<Module " << obj->as<Module>()->name << '>';
    break;
  case Type::Class:
    vf.os << "<Class " << obj->as<Class>()->name << '>';
    break;
  case Type::Task:
    vf.os << "<Task>";
    break;
  case Type::Instance: {
    if (obj->class_->name != "Object")
      vf.os << obj->class_->name << " ";
    if (vf.depth > MAX_DEPTH) {
      vf.os << "{ ... }";
    } else {
      auto new_vf = vf.inc_depth();
      auto &o = obj->as<Instance>()->properties;
      auto it = o.begin();
      if (it != o.end()) {
        vf.os << "{ ";
        new_vf.os << static_cast<StringSlice>(*it->first);
        new_vf.os << ": ";
        new_vf << it->second;
        it++;
        for (auto p = it; p != o.end(); p++) {
          new_vf.os << ", ";
          new_vf.os << static_cast<StringSlice>(*p->first);
          new_vf.os << ": ";
          new_vf << p->second;
        }
        vf.os << " }";
      } else {
        vf.os << "{}";
      }
    }
    break;

    break;
  }
  default:
    unreachable();
  }
}
Array::Array(size_t size) : inner(std::vector<Value>(size, Value::null())) {}

Map::Map(size_t size) { inner.reserve(size); }
Object *Class::find_method(Symbol *method) {
  auto class_ = this;
  while (class_ != nullptr) {
    if (class_->methods.find(method) != class_->methods.end()) {
      return class_->methods[method];
    }
    class_ = class_->super;
  }
  return nullptr;
}
Instance::Instance(size_t size) { properties.reserve(size); }

std::ostream &operator<<(std::ostream &os, StringSlice s) {
  os.write(s.data, std::streamsize(s.len));
  return os;
}

} // namespace neptune_vm
