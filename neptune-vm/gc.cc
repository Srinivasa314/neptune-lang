#include "neptune-vm.h"

namespace neptune_vm {
template <typename T> T *GC::manage(T *t) {
  static_assert(std::is_base_of<Object, T>::value,
                "T must be a descendant of Object");
  bytes_allocated += sizeof(T);
  auto o = reinterpret_cast<Object *>(t);
  o->type = T::type;
  o->is_dark = false;
  o->next = first_obj;
  first_obj = o;
  return t;
}

template <typename T> T *GC::make_constant(T *t) {
  static_assert(std::is_base_of<Object, T>::value,
                "T must be a descendant of Object");
  manage(t);
  constants.push_back(t);
  return t;
}
} // namespace neptune_vm
