#pragma once

namespace neptune_vm {
template <typename O> class Handle {
  Handle *previous;
  Handle *next;
  friend class VM;

public:
  O *object;
  Handle(Handle *previous, O *object, Handle *next)
      : previous(previous), next(next), object(object) {}
};
} // namespace neptune_vm
