#pragma once
#include "object.h"
namespace neptune_vm {
template <typename O> class Handle {
  Handle *previous;
  Handle *next;
  friend class VM;
public:
  O *object;
  Handle(Handle *previous_, O *object_, Handle *next_)
      : previous(previous_), next(next_), object(object_) {}
};
} // namespace neptune_vm
