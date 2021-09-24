#pragma once
#include <memory>
#include <string>
#include <tsl/robin_set.h>

constexpr size_t MAX_FRAMES = 1024;
constexpr size_t STACK_SIZE = 128 * 1024;
constexpr unsigned int HEAP_GROWTH_FACTOR = 2;
constexpr size_t INITIAL_HEAP_SIZE = 10 * 1024 * 1024;
constexpr bool STRESS_GC = false;
constexpr bool DEBUG_GC = false;

namespace neptune_vm {
struct Frame {
  Value *bp;
  FunctionInfo *f;
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
  std::unique_ptr<Value[]> stack;
  std::unique_ptr<Frame[]> frames;
  size_t num_frames;
  mutable std::vector<Value> globals;
  mutable std::vector<std::string> global_names;
  size_t bytes_allocated;
  // Linked list of all objects
  Object *first_obj;
  size_t threshhold;
  tsl::robin_set<Symbol *, StringHasher, StringEquality> symbols;
  Handle<Object> *handles;
  Value *stack_top;
  std::vector<Object *> greyobjects;

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
  void collect();
  void blacken(Object *o);
  void grey(Object *o);
  VM()
      : stack(new Value[STACK_SIZE]), frames(new Frame[MAX_FRAMES]),
        num_frames(0), bytes_allocated(0), first_obj(nullptr),
        threshhold(INITIAL_HEAP_SIZE), handles(nullptr),
        stack_top(stack.get()) {}
  ~VM();
};

std::unique_ptr<VM> new_vm();
} // namespace neptune_vm
