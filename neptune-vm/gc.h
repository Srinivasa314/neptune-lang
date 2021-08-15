#pragma once
#include "object.h"
#include <tsl/robin_map.h>
namespace neptune_vm {
template <typename T> class Handle {
  friend class GC;
  Handle *previous;
  Handle *next;

public:
  T object;
  Handle(Handle *previous, T object, Handle *next)
      : previous(previous), next(next), object(object) {}
  ~Handle() {
    if (this->previous != nullptr)
      this->previous->next = this->next;
    if (this->next != nullptr)
      this->next->previous = this->previous;
  }
};
class GC {
  size_t bytes_allocated;
  std::vector<Object *> constants; // these live as long the GC
  // Linked list of all objects
  Object *first_obj;
  size_t threshhold;
  tsl::robin_map<Symbol *, bool> symbol_table;
  Handle<Object *> *handles;

public:
  // safety:must not be null
  template <typename T> T *manage(T *t);
  template <typename T> T *make_constant(T *t);
  template <typename T> Handle<T> *make_handle(T object) {
    if (this->handles == nullptr)
      return reinterpret_cast<Handle<T> *>(
          this->handles = new Handle(static_cast<Handle<Object *> *>(nullptr),
                                     static_cast<Object *>(object),
                                     static_cast<Handle<Object *> *>(nullptr)));
    else
      return reinterpret_cast<Handle<T> *>(
          this->handles->next =
              new Handle(this->handles->next, static_cast<Object *>(object),
                         static_cast<Handle<Object *> *>(nullptr)));
  }
};
} // namespace neptune_vm
