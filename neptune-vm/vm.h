#pragma once
#include <memory>
#include <string>
#include <tsl/robin_set.h>

namespace neptune_vm {
struct Frame {
  Value *bp; // base pointer which points to the base of the stack
};

enum class VMStatus : uint8_t { Success, Error };

class VMResult {
  VMStatus status;
  std::string result;

public:
  VMResult(VMStatus status_, const std::string &result_)
      : status(status_), result(result_) {}
  VMStatus get_status() const { return status; }
  StringSlice get_result() const {
    return StringSlice{result.data(), result.size()};
  }
};

class VM {
  std::vector<Value> stack;
  std::vector<Frame> frames;
  mutable std::vector<Value> globals;
  mutable std::vector<std::string> global_names;
  size_t bytes_allocated;
  // Linked list of all objects
  Object *first_obj;
  size_t threshhold;
  tsl::robin_set<Symbol *, StringHasher, StringEquality> symbols;
  Handle<Object> *handles;
  Value *stack_top;

public:
  Value to_string(Value val);
  VMResult run(FunctionInfo *f);
  void add_global(StringSlice name) const;
  FunctionInfoWriter new_function_info() const;
  template <typename O> O *manage(O *t);
  template <typename O> Handle<O> *make_handle(O *object);
  template <typename O> void release(Handle<O> *handle);
  Symbol *intern(StringSlice s);
  void release(Object *o);
  VM()
      : stack(1024 * 1024, Value::null()), frames(1024), bytes_allocated(0),
        first_obj(nullptr), threshhold(10 * 1024 * 1024), handles(nullptr) {}
  ~VM();
};

std::unique_ptr<VM> new_vm();
} // namespace neptune_vm
