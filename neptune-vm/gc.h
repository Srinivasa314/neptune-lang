#pragma once
#include <tsl/robin_map.h>

namespace neptune_vm {
class GC {
  size_t bytes_allocated;
  std::vector<Object *> constants; // these live as long the GC
  // Linked list of all objects
  Object *first_obj;
  size_t threshhold;
  tsl::robin_map<Symbol *, bool> symbol_table;

public:
  // safety:must not be null
  template <typename T> T *manage(T *t);
  template <typename T> T *make_constant(T *t);
};
} // namespace neptune_vm
