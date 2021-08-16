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

Symbol *Symbol::from_string_slice(StringSlice s) {
  Symbol *p = static_cast<Symbol *>(malloc(sizeof(Symbol) + s.len));
  if (p == nullptr) {
    throw std::bad_alloc();
  }
  memcpy(p->data, s.data, s.len);
  p->len = s.len;
  return p;
}
} // namespace neptune_vm
