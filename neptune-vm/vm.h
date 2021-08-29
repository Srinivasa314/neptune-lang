#pragma once
#include <memory>
#include <string>
#include <tsl/robin_set.h>

namespace neptune_vm {
struct Frame {
  Value *bp; // base pointer which points to the base of the stack
};

struct Global {
  std::string name;
  Value value;
};

class VM {
  std::vector<Value> stack;
  std::vector<Frame> frames;
  mutable std::vector<Global> globals;
  mutable size_t bytes_allocated;
  // Linked list of all objects
  mutable Object *first_obj;
  size_t threshhold;
  mutable tsl::robin_set<Symbol *, StringHasher, StringEquality> symbols;
  mutable Handle<Object> *handles;

public:
  // SAFETY:must not be null bytecode must be valid
  void run(FunctionInfo *f) const;
  void add_global(StringSlice name) const;
  FunctionInfoWriter new_function_info() const;
  template <typename O> O *manage(O *t) const;
  template <typename O> Handle<O> *make_handle(O *object) const;
  template <typename O> void release(Handle<O> *handle) const;
  Symbol *intern(StringSlice s) const;
  void release(Object *o) const;
  VM()
      : stack(1024 * 1024, Value::null()), frames(1024), bytes_allocated(0),
        first_obj(nullptr), threshhold(10 * 1024 * 1024), handles(nullptr) {}
  ~VM();
};

std::unique_ptr<VM> new_vm();
} // namespace neptune_vm
