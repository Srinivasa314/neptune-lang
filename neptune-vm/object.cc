#include "neptune-vm.h"
#include <cstring>
#include <new>

namespace neptune_vm {
template <> size_t size(String *s) { return sizeof(String) + s->len; }
template <> size_t size(Symbol *s) { return sizeof(Symbol) + s->len; }
template <> size_t size(Function *f) {
  return sizeof(Function) + f->num_upvalues * sizeof(UpValue *);
}

String::operator StringSlice() const { return StringSlice{data, len}; }
Symbol::operator StringSlice() const { return StringSlice{data, len}; }
template <typename O> bool Object::is() const { return type == O::type; }
template <typename O> O *Object::as() {
  assert(is<O>());
  return reinterpret_cast<O *>(this);
}

String *VM::concat(String *s1, String *s2) {
  String *p = static_cast<String *>(alloc(sizeof(String) + s1->len + s2->len));
  if (p == nullptr) {
    throw std::bad_alloc();
  }
  memcpy(p->data, s1->data, s1->len);
  memcpy(p->data + s1->len, s2->data, s2->len);
  p->len = s1->len + s2->len;
  return manage(p);
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
uint32_t StringHasher::operator()(String *s) const {
  return StringHasher{}(static_cast<StringSlice>(*s));
}

const char *Object::type_string() const {
  // todo change this when more types are added
  switch (type) {
  case Type::Class:
    return "Class";
  case Type::String:
    return "String";
  case Type::Symbol:
    return "Symbol";
  case Type::Array:
    return "Array";
  case Type::Map:
    return "Map";
  case Type::Function:
    return "Function";
  case Type::NativeFunction:
    return "Function";
  case Type::Module:
    return "Module";
  case Type::Task:
    return "Task";
  case Type::Instance:
    return const_cast<Object *>(this)->as<Instance>()->class_->name.c_str();
  case Type::FunctionInfo:
    return "<internal type FunctionInfo>";
  case Type::UpValue:
    return "<internal type UpValue>";
  case Type::Range:
    return "Range";
  case Type::ArrayIterator:
    return "ArrayIterator";
  case Type::MapIterator:
    return "MapIterator";
  case Type::StringIterator:
    return "StringIterator";
  case Type::Channel:
    return "Channel";
  case Type::Resource:
    return "Resource";
  default:
    unreachable();
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
        ++it;
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
        ++it;
        for (auto p = it; p != o.end(); ++p) {
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
    if (obj->as<Task>()->name != nullptr)
      vf.os << "<Task " << escaped_string(*obj->as<Task>()->name) << '>';
    else
      vf.os << "<Task>";
    break;
  case Type::Instance: {
    if (obj->as<Instance>()->class_->name != "Object")
      vf.os << obj->as<Instance>()->class_->name << " ";
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
        ++it;
        for (auto p = it; p != o.end(); ++p) {
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
  }
  case Type::Range:
    vf.os << obj->as<Range>()->start << ".." << obj->as<Range>()->end;
    break;
  case Type::ArrayIterator:
    vf.os << "<ArrayIterator>";
    break;
  case Type::MapIterator:
    vf.os << "<MapIterator>";
    break;
  case Type::StringIterator:
    vf.os << "<StringIterator>";
    break;
  case Type::Channel:
    vf.os << "<Channel>";
    break;
  case Type::Resource:
    vf.os << "<Resource>";
    break;
  default:
    unreachable();
  }
}
Array::Array(size_t size) : inner(vector<Value>(size, Value::null())) {}

Array::Array(size_t size, Value v) : inner(vector<Value>(size, v)) {}

Map::Map(uint32_t size) : inner(size) {}
Object *Class::find_method(Symbol *method) {
  auto class_ = this;
  while (class_ != nullptr) {
    auto iter = class_->methods.find(method);
    if (iter != class_->methods.end()) {
      return iter->second;
    }
    class_ = class_->super;
  }
  return nullptr;
}
Instance::Instance(size_t size) : properties(static_cast<uint32_t>(size)) {}

std::ostream &operator<<(std::ostream &os, StringSlice s) {
  os.write(s.data, std::streamsize(s.len));
  return os;
}

MapIterator::MapIterator(Map *map) {
  this->map = map;
  auto iter = map->inner.begin();
  if (iter == map->inner.end())
    last_key = Value(nullptr);
  else
    last_key = iter->first;
}

void Class::copy_methods(Class &other) {
  for (auto &method : other.methods) {
    methods.insert(method);
  }
}

// Boyer-Moore-Horspool algorithm
size_t String::find(String *haystack, String *needle, size_t start) {
  if (needle->len == 0)
    return start;

  if (start + needle->len > haystack->len)
    return haystack->len;

  size_t skip[UINT8_MAX];

  for (size_t i = 0; i < UINT8_MAX; i++)
    skip[i] = needle->len;

  for (size_t i = 0; i < needle->len - 1; i++)
    skip[static_cast<uint8_t>(needle->data[i])] = needle->len - 1 - i;

  char last = needle->data[needle->len - 1], c;

  for (size_t i = start; i <= haystack->len - needle->len;
       i += skip[static_cast<uint8_t>(c)]) {
    c = haystack->data[i + needle->len - 1];
    if (last == c &&
        memcmp(haystack->data + i, needle->data, needle->len - 1) == 0) {
      return i;
    }
  }

  return haystack->len;
}

String *String::replace(VM *vm, String *from, String *to) {
  if (from->len == 0)
    return this;
  std::string result;
  size_t offset = 0, pos;
  while (1) {
    pos = String::find(this, from, offset);
    result.insert(result.end(), data + offset, data + pos);
    if (pos == len)
      break;
    offset = pos + from->len;
    result.insert(result.end(), to->data, to->data + to->len);
  }
  return vm->allocate<String>(result);
}

bool ValueEmpty::is_empty(Value v) { return v.is_empty(); }
Value ValueEmpty::empty() { return Value(nullptr); }

} // namespace neptune_vm
