#pragma once
#include "rust/cxx.h"
#include <memory>
#include <sstream>
#include <string>
#include <tsl/robin_map.h>
#include <tsl/robin_set.h>

constexpr size_t INITIAL_FRAMES = 4;
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

class Task : public Object {
  std::unique_ptr<Value[]> stack;
  size_t stack_size;
  Value *stack_top;
  UpValue *open_upvalues;

public:
  std::vector<Frame> frames;
  static constexpr Type type = Type::Task;
  friend class VM;
  void close(Value *last);
  Value *grow_stack(Value *bp, size_t extra_needed);
  Task(size_t stack_size_)
      : stack(std::unique_ptr<Value[]>(new Value[stack_size_ / sizeof(Value)])),
        stack_size(stack_size_), stack_top(stack.get()), open_upvalues(NULL) {}
};

class VM {
public:
  Task *current_task;

private:
  std::vector<Object *> temp_roots;
  tsl::robin_map<std::string, Module *, StringHasher, StringEquality> modules;
  mutable std::vector<Value> module_variables;
  size_t bytes_allocated;
  // Linked list of all objects
  Object *first_obj;
  size_t threshhold;
  tsl::robin_set<Symbol *, StringHasher, StringEquality> symbols;
  Handle<Object> *handles;
  std::vector<Object *> greyobjects;
  std::string stack_trace;
  bool is_running;
  std::ostringstream panic_message;
  NativeFunction *last_native_function;

public:
  Value return_value;
  Value to_string(Value val);
  VMStatus run(Task *task, Function *f);
  bool add_module_variable(StringSlice module, StringSlice name, bool mutable_,
                           bool exported) const;
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
  std::string generate_stack_trace();
  const uint8_t *panic(const uint8_t *ip, Value v);
  const uint8_t *panic(const uint8_t *ip);
  bool declare_native_function(StringSlice module, StringSlice name,
                               bool exported, uint8_t arity,
                               uint16_t extra_slots,
                               NativeFunctionCallback *callback, Data *data,
                               FreeDataCallback *free_data) const;
  void declare_native_builtins();
  Function *make_function(Value *bp, FunctionInfo *function_info);
  rust::String get_stack_trace() const { return rust::String(stack_trace); }
  rust::String get_result() const {
    std::ostringstream os;
    os << return_value;
    return rust::String(os.str());
  }
  bool module_exists(StringSlice module_name) const;
  void create_module(StringSlice module_name) const;
  void create_module_with_prelude(StringSlice module_name) const;
  Module *get_module(StringSlice module_name) const;
  VM()
      : bytes_allocated(0), first_obj(nullptr), threshhold(INITIAL_HEAP_SIZE),
        handles(nullptr), is_running(false), last_native_function(nullptr),
        return_value(Value::null()),current_task(nullptr) {
    create_module(StringSlice("<prelude>"));
    declare_native_builtins();
  }
  ~VM();
};

std::unique_ptr<VM> new_vm();
std::unique_ptr<VM> new_vm();
} // namespace neptune_vm
