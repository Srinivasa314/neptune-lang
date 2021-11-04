#pragma once
#include "rust/cxx.h"
#include <memory>
#include <sstream>
#include <string>
#include <tsl/robin_map.h>
#include <tsl/robin_set.h>

constexpr size_t MAX_FRAMES = 1024;
constexpr size_t STACK_SIZE = 128 * 1024;
constexpr unsigned int HEAP_GROWTH_FACTOR = 2;
constexpr size_t INITIAL_HEAP_SIZE = 10 * 1024 * 1024;
constexpr bool STRESS_GC = false;
constexpr bool DEBUG_GC = false;

namespace neptune_vm {
namespace native_builtins {
bool disassemble(FunctionContext ctx, void *data);
} // namespace native_builtins
struct Frame {
  Value *bp;
  Function *f;
  const uint8_t *ip;
};

class VM {
  std::unique_ptr<Value[]> stack;
  std::unique_ptr<Frame[]> frames;
  size_t num_frames;
  UpValue *open_upvalues;
  std::vector<Object *> temp_roots;
  tsl::robin_map<std::string, Module *, StringHasher, StringEquality> modules;
  mutable std::vector<Value> module_variables;
  size_t bytes_allocated;
  // Linked list of all objects
  Object *first_obj;
  size_t threshhold;
  tsl::robin_set<Symbol *, StringHasher, StringEquality> symbols;
  Handle<Object> *handles;
  Value *stack_top;
  std::vector<Object *> greyobjects;
  std::string stack_trace;
  bool is_running;
  std::ostringstream panic_message;
  NativeFunction *last_native_function;

public:
  Value return_value;
  Value to_string(Value val);
  VMStatus run(Function *f);
  bool add_module_variable(StringSlice module, StringSlice name,
                           bool mutable_) const;
  ModuleVariable get_module_variable(StringSlice module_name,
                                     StringSlice name) const;
  FunctionInfoWriter new_function_info(StringSlice module, StringSlice name,
                                       uint8_t arity) const;
  template <typename O> O *manage(O *t);
  template <typename O> Handle<O> *make_handle(O *object);
  template <typename O> void release(Handle<O> *handle);
  Symbol *intern(StringSlice s);
  void release(Object *o);
  void collect();
  void blacken(Object *o);
  void grey(Object *o);
  void close(Value *last);
  std::string generate_stack_trace();
  const uint8_t *panic(const uint8_t *ip, Value v);
  const uint8_t *panic(const uint8_t *ip);
  bool declare_native_function(StringSlice module, StringSlice name,
                               uint8_t arity, uint16_t extra_slots,
                               NativeFunctionCallback *callback, Data *data,
                               FreeDataCallback *free_data) const;
  void declare_native_builtins();
  Value make_function(Value *bp, Value constant);
  rust::String get_stack_trace() const { return rust::String(stack_trace); }
  rust::String get_result() const {
    std::ostringstream os;
    os << return_value;
    return rust::String(os.str());
  }
  bool module_exists(StringSlice module_name) const;
  void create_module(StringSlice module_name) const;
  void create_module_with_prelude(StringSlice module_name) const;
  VM()
      : stack(new Value[STACK_SIZE]), frames(new Frame[MAX_FRAMES]),
        num_frames(0), open_upvalues(nullptr), bytes_allocated(0),
        first_obj(nullptr), threshhold(INITIAL_HEAP_SIZE), handles(nullptr),
        stack_top(stack.get()), is_running(false), return_value(Value::null()) {
    create_module(StringSlice("prelude"));
    declare_native_builtins();
  }
  ~VM();
};

std::unique_ptr<VM> new_vm();
} // namespace neptune_vm
