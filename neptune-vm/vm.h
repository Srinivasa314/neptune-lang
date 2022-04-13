#pragma once
#include "hash_table.h"
#include "native_function.h"
#include "util.h"
#include <deque>
#include <functional>
#include <memory>
#include <random>
#include <sstream>
#include <string>

constexpr size_t INITIAL_FRAMES = 4;
constexpr unsigned int HEAP_GROWTH_FACTOR = 2;
constexpr size_t INITIAL_HEAP_SIZE = 10 * 1024 * 1024;
constexpr bool STRESS_GC = false;
constexpr bool DEBUG_GC = false;

namespace neptune_vm {
struct Frame {
  Value *bp;
  Function *f;
  const uint8_t *ip;
};

class Task;

class Channel : public Object {
public:
  std::deque<Value> queue;
  std::deque<Task *> wait_list;
  void send(Value v, VM *vm);
  static constexpr Type type = Type::Channel;
};

class Task : public Object {
public:
  VMStatus status;
  Value uncaught_exception;
  bool waiting_for_rust_future;
  std::unique_ptr<Value[]> stack;
  UpValue *open_upvalues;
  size_t stack_size;
  Value *stack_top;
  vector<Frame> frames;
  vector<Channel *> monitors;
  String *name;
  HashSet<Task *, PointerHash<Task>, std::equal_to<Task *>, NullptrEmpty<Task>>
      links;

  static constexpr Type type = Type::Task;
  void close(Value *last);
  Value *grow_stack(Value *bp, size_t extra_needed);
  explicit Task(Function *f);
};

struct TaskQueueEntry {
  Task *task;
  Value accumulator;
  bool uncaught_exception;
};

class TaskHandle {
  Handle<Task> *handle;
  VM *vm;

public:
  TaskHandle(VM *vm, Task *task);
  void release();
  VMStatus resume(EFuncCallback *callback, Data *data);
};

class Resource : public Object {
  Data *data;
  FreeDataCallback *free_data;

public:
  Resource(Data *data, FreeDataCallback *free_data)
      : data(data), free_data(free_data) {}
  void close() {
    if (data != nullptr) {
      free_data(data);
      data = nullptr;
    }
  }
  static constexpr Type type = Type::Resource;
  friend class EFuncContext;
};

class VM {
private:
  rust::Box<UserData> user_data;
  HashMap<String *, Module *, StringHasher, StringEquality,
          NullptrEmpty<String>>
      modules;
  mutable vector<Value> module_variables;
  size_t bytes_allocated;
  // Linked list of all objects
  Object *first_obj;
  size_t threshhold;
  HashSet<Symbol *, StringHasher, StringEquality, NullptrEmpty<Symbol>> symbols;
  Handle<Object> *handles;
  vector<Object *> greyobjects;
  std::ostringstream throw_message;
  NativeFunction *last_native_function;
  template <typename O> O *manage(O *object);

public:
  bool is_running;
  Task *current_task;
  Task *main_task;
  BuiltinClasses builtin_classes;
  BuiltinSymbols builtin_symbols;
  vector<Value> temp_roots;
  SymbolMap<EFunc> efuncs;
  Value return_value;
  std::mt19937_64 rng;
  std::deque<TaskQueueEntry> tasks_queue;
  Value to_string(Value val);
  void run(TaskQueueEntry entry);
  VMStatus run();
  bool add_module_variable(StringSlice module, StringSlice name, bool mutable_,
                           bool exported) const;
  ModuleVariable get_module_variable(StringSlice module_name,
                                     StringSlice name) const;
  FunctionInfoWriter new_function_info(StringSlice module, StringSlice name,
                                       uint8_t arity) const;
  template <typename O, typename... Args> O *allocate(Args... args);
  template <typename O> Handle<O> *make_handle(O *object);
  template <typename O> void release(Handle<O> *handle);
  Symbol *intern(StringSlice s);
  void release(Object *o);
  void collect();
  void trace(Object *o);
  void mark(Object *o);
  std::string generate_stack_trace(bool include_native_function,
                                   uint32_t depth);
  const uint8_t *throw_(Value v);
  const uint8_t *throw_(const uint8_t *ip, const char *type);
  bool declare_native_function(std::string module, std::string name,
                               bool exported, uint8_t arity,
                               NativeFunctionCallback *callback) const;
  void declare_native_builtins();
  Function *make_function(Value *bp, FunctionInfo *function_info);
  rust::String get_result() const {
    auto vm = const_cast<VM *>(this);
    auto s = vm->report_error(return_value);
    vm->return_value = Value::null();
    return rust::String(std::move(s));
  }
  bool module_exists(StringSlice module_name) const;
  void create_module(StringSlice module_name) const;
  void create_module_with_prelude(StringSlice module_name) const;
  bool create_efunc(StringSlice name, EFuncCallback *callback, Data *data,
                    FreeDataCallback *free_data) const;
  Module *get_module(StringSlice module_name) const;
  Class *get_class(Value v) const;
  String *concat(String *s1, String *s2);
  Value create_error(StringSlice type, StringSlice message);
  Value create_error(StringSlice module, StringSlice type, StringSlice message);
  std::string report_error(Value error);
  void kill(Task *task, Value uncaught_exception);
  rust::String kill_main_task(StringSlice error, StringSlice message) const;
  TaskHandle get_current_task() const {
    return TaskHandle(const_cast<VM *>(this), current_task);
  }
  const UserData &get_user_data() const { return *user_data; }
  VM(rust::Box<UserData> user_data);
  ~VM();
};

std::unique_ptr<VM> new_vm(rust::Box<UserData> user_data);
template <> String *VM::allocate<String, StringSlice>(StringSlice s);

template <> String *VM::allocate<String, std::string>(std::string s);
template <> String *VM::allocate<String, const char *>(const char *s);
} // namespace neptune_vm
