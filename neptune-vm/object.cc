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

template <typename O> bool Object::is() { return type == O::type; }
template <typename O> O *Object::as() {
  assert(is<O>());
  return reinterpret_cast<O *>(this);
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
    hash ^= s.data[i];
    hash *= prime;
  }

  return hash;
}

size_t StringHasher::operator()(Symbol *sym) const {
  return StringHasher{}(StringSlice{sym->data, sym->len});
}

} // namespace neptune_vm
