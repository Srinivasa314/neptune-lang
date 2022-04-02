#include "checked_arithmetic.cc"
#include "neptune-vm.h"
#include <cstring>
#include <mimalloc.h>

#if defined(__GNUC__) || defined(__clang__)
#define COMPUTED_GOTO
#endif

constexpr uint32_t WIDE_OFFSET =
    static_cast<uint32_t>(neptune_vm::Op::Throw) + 1;
constexpr uint32_t EXTRAWIDE_OFFSET = 2 * WIDE_OFFSET;

#define WIDE(x) (static_cast<uint32_t>(x) + WIDE_OFFSET)
#define EXTRAWIDE(x) (static_cast<uint32_t>(x) + EXTRAWIDE_OFFSET)

#ifdef COMPUTED_GOTO

#define HANDLER(x) x##_handler
#define WIDE_HANDLER(x) x##_wide_handler
#define EXTRAWIDE_HANDLER(x) x##_extrawide_handler

#define DISPATCH() goto *dispatch_table[READ(uint8_t)]
#define DISPATCH_WIDE() goto *dispatch_table[WIDE(READ(uint8_t))]
#define DISPATCH_EXTRAWIDE() goto *dispatch_table[EXTRAWIDE(READ(uint8_t))]
#define INTERPRET_LOOP DISPATCH();

#else

#define HANDLER(x) case static_cast<uint32_t>(Op::x)
#define WIDE_HANDLER(x) case WIDE(Op::x)
#define EXTRAWIDE_HANDLER(x) case EXTRAWIDE(Op::x)
#define INTERPRET_LOOP                                                         \
  uint32_t __op;                                                               \
  DISPATCH();                                                                  \
  loop:                                                                        \
  switch (__op)

#define DISPATCH()                                                             \
  __op = READ(uint8_t);                                                        \
  goto loop

#define DISPATCH_WIDE()                                                        \
  __op = WIDE(READ(uint8_t));                                                  \
  goto loop

#define DISPATCH_EXTRAWIDE()                                                   \
  __op = EXTRAWIDE(READ(uint8_t));                                             \
  goto loop

#endif

#define READ(type) read<type>(ip)
#define CLOSE(n) task->close(&bp[n])

namespace neptune_vm {

VM::VM(Data *user_data, FreeDataCallback *free_user_data)
    : user_data(user_data), free_user_data(free_user_data), bytes_allocated(0),
      first_obj(nullptr), threshhold(INITIAL_HEAP_SIZE), handles(nullptr),
      is_running(false), last_native_function(nullptr), current_task(nullptr),
      return_value(Value::null()), rng(std::random_device()()) {
  builtin_symbols.construct = intern("construct");
  builtin_symbols.message = intern("message");
  builtin_symbols.stack = intern("stack");
  builtin_symbols.task = intern("task");
  builtin_symbols.finished = intern("finished");
  builtin_symbols.running = intern("running");
  builtin_symbols.killed = intern("killed");
  create_module("<prelude>");
  declare_native_builtins();
}

#define THROW(type, fmt)                                                       \
  do {                                                                         \
    throw_message << fmt;                                                      \
    if ((ip = throw_(ip, type)) != nullptr) {                                  \
      bp = task->frames.back().bp;                                             \
      auto f = task->frames.back().f;                                          \
      constants = f->function_info->constants.data();                          \
      DISPATCH();                                                              \
    } else {                                                                   \
      goto throw_end;                                                          \
    }                                                                          \
  } while (0)

VMStatus VM::run() {
  // return if main task finished/rust needs to resume/error
  if (is_running)
    throw std::runtime_error("Cannot call run() while VM is already running");

  while (!tasks_queue.empty()) {
    TaskQueueEntry entry = tasks_queue.front();
    tasks_queue.pop_front();
    if (entry.task->status == VMStatus::Error)
      continue;
    run(entry);
    if (main_task->status == VMStatus::Error) {
      main_task = nullptr;
      is_running = false;
      return VMStatus::Error;
    } else if (main_task->status == VMStatus::Success) {
      for (auto link : main_task->links) {
        if (link->status == VMStatus::Suspend)
          kill(link, Value::null());
      }
      main_task = nullptr;
      is_running = false;
      return VMStatus::Success;
    }
  }
  is_running = false;
  return VMStatus::Suspend;
}

void VM::run(TaskQueueEntry entry) {
#ifdef COMPUTED_GOTO
  static void *dispatch_table[] = {
#define OP(x) &&HANDLER(x),
      OPS
#undef OP
#define OP(x) &&WIDE_HANDLER(x),
          OPS
#undef OP
#define OP(x) &&EXTRAWIDE_HANDLER(x),
              OPS
#undef OP
  };
#endif

  Value accumulator = entry.accumulator;
  Task *task = entry.task;
  current_task = task;
  auto frame = task->frames.back();
  const uint8_t *ip = frame.ip;
  Value *bp = frame.bp;
  Value *constants = frame.f->function_info->constants.data();
  if (entry.uncaught_exception) {
    if ((ip = throw_(accumulator)) != nullptr) {
      bp = task->frames.back().bp;
      auto f = task->frames.back().f;
      constants = f->function_info->constants.data();
      DISPATCH();
    } else {
      goto throw_end;
    }
  }

  INTERPRET_LOOP {
    HANDLER(Wide) : DISPATCH_WIDE();
    HANDLER(ExtraWide) : DISPATCH_EXTRAWIDE();
    WIDE_HANDLER(Wide)
        : WIDE_HANDLER(ExtraWide)
        : EXTRAWIDE_HANDLER(Wide)
        : EXTRAWIDE_HANDLER(ExtraWide) : unreachable();

#define handler(op, impl)                                                      \
  HANDLER(op) : impl DISPATCH();                                               \
  WIDE_HANDLER(op) : unreachable();                                            \
  EXTRAWIDE_HANDLER(op) : unreachable()
#include "handlers.h"
#undef handler

#define utype uint8_t
#define itype int8_t
#define handler(op, impl) HANDLER(op) : impl DISPATCH()
#include "wide_handlers.h"
#undef utype
#undef itype
#undef handler

#define utype uint16_t
#define itype int16_t
#define handler(op, impl) WIDE_HANDLER(op) : impl DISPATCH()
#include "wide_handlers.h"
#undef utype
#undef itype
#undef handler

#define utype uint32_t
#define itype int32_t
#define handler(op, impl) EXTRAWIDE_HANDLER(op) : impl DISPATCH()
#include "wide_handlers.h"
#undef utype
#undef itype
#undef handler

// Doubles performance for MSVC!!
#ifndef COMPUTED_GOTO
  default:
    unreachable();
#endif
  }
throw_end:
  kill(current_task, accumulator);
  if (current_task != main_task)
    return_value = Value::null();
  current_task = nullptr;
  return;
end:
  for (auto chan : current_task->monitors)
    chan->send(Value(task), this);
  current_task = nullptr;
}
#undef READ
#undef THROW

void Task::close(Value *last) {
  while (open_upvalues != nullptr && open_upvalues->location >= last) {
    open_upvalues->closed = *open_upvalues->location;
    open_upvalues->location = &open_upvalues->closed;
    open_upvalues = open_upvalues->next;
  }
}

bool VM::add_module_variable(StringSlice module, StringSlice name,
                             bool mutable_, bool exported) const {
  auto module_iter = modules.find(module);
  if (module_iter == modules.end())
    return false;
  else {
    auto module = module_iter->second;
    if (!module->module_variables.insert(
            {const_cast<VM *>(this)->intern(name),
             ModuleVariable{static_cast<uint32_t>(module_variables.size()),
                            mutable_, exported}}))
      return false;
    module_variables.push_back(Value::null());
    return true;
  }
}

ModuleVariable VM::get_module_variable(StringSlice module_name,
                                       StringSlice name) const {
  auto module_iter = modules.find(module_name);
  if (module_iter == modules.end())
    throw std::runtime_error("No such module");
  auto module = module_iter->second;
  auto it = module->module_variables.find(name);
  if (it == module->module_variables.end())
    throw std::runtime_error("No such module variable");
  return it->second;
}
std::unique_ptr<VM> new_vm(Data *user_data, FreeDataCallback *free_user_data) { return std::unique_ptr<VM>{new VM(user_data,free_user_data)}; }

FunctionInfoWriter VM::new_function_info(StringSlice module, StringSlice name,
                                         uint8_t arity) const {
  auto this_ = const_cast<VM *>(this);
  auto function_info = this_->allocate<FunctionInfo>(module, name, arity);
  return FunctionInfoWriter(this_->make_handle(function_info), this);
}

template <typename O, typename... Args> O *VM::allocate(Args... args) {
  return manage(new O(args...));
}

template <> String *VM::allocate<String, StringSlice>(StringSlice s) {
  String *p = static_cast<String *>(mi_malloc(sizeof(String) + s.len));
  if (p == nullptr) {
    throw std::bad_alloc();
  }
  memcpy(p->data, s.data, s.len);
  p->len = s.len;
  return manage(p);
}

template <> String *VM::allocate<String, std::string>(std::string s) {
  return VM::allocate<String>(StringSlice{s.data(), s.length()});
}

template <> String *VM::allocate<String, const char *>(const char *s) {
  return VM::allocate<String>(StringSlice(s));
}

template <typename O> O *VM::manage(O *t) {
  if (STRESS_GC || bytes_allocated > threshhold)
    collect();
  static_assert(std::is_base_of<Object, O>::value,
                "O must be a descendant of Object");
  bytes_allocated += size(t);
  auto o = reinterpret_cast<Object *>(t);
  o->type = O::type;
  o->is_dark = false;
  o->next = first_obj;
  first_obj = o;
  return t;
}

template <typename O> Handle<O> *VM::make_handle(O *object) {
  static_assert(std::is_base_of<Object, O>::value,
                "O must be a descendant of Object");
  if (handles == nullptr)
    return reinterpret_cast<Handle<O> *>(
        handles = new Handle<Object>(nullptr, static_cast<Object *>(object),
                                     nullptr));
  else {
    handles->previous =
        new Handle<Object>(nullptr, static_cast<Object *>(object), handles);
    handles = handles->previous;
    return reinterpret_cast<Handle<O> *>(handles);
  }
}

template <typename O> void VM::release(Handle<O> *handle) {
  if (handle->previous != nullptr)
    handle->previous->next = handle->next;
  else
    handles = reinterpret_cast<Handle<Object> *>(handle->next);
  if (handle->next != nullptr)
    handle->next->previous = handle->previous;
  delete handle;
}

Symbol *VM::intern(StringSlice s) {
  auto reused_sym = symbols.find(s);
  if (reused_sym == symbols.end()) {
    Symbol *sym = static_cast<Symbol *>(mi_malloc(sizeof(Symbol) + s.len));
    if (sym == nullptr) {
      throw std::bad_alloc();
    }
    memcpy(sym->data, s.data, s.len);
    sym->len = s.len;
    sym->hash = StringHasher{}(*sym);
    manage(sym);
    symbols.insert(sym);
    return sym;
  } else {
    return *reused_sym;
  }
}

void VM::release(Object *o) {
  if (DEBUG_GC)
    std::cout << "Freeing: " << *o << std::endl;
  // todo change this when more types are added
  switch (o->type) {
  case Type::String:
    mi_free(o);
    break;
  case Type::Symbol:
    symbols.erase(o->as<Symbol>());
    mi_free(o);
    break;
  case Type::Array:
    delete o->as<Array>();
    break;
  case Type::Map:
    delete o->as<Map>();
    break;
  case Type::FunctionInfo:
    delete o->as<FunctionInfo>();
    break;
  case Type::Function:
    mi_free(o);
    break;
  case Type::UpValue:
    delete o->as<UpValue>();
    break;
  case Type::NativeFunction: {
    auto n = o->as<NativeFunction>();
    delete n;
  } break;
  case Type::Module: {
    delete o->as<Module>();
    break;
  }
  case Type::Class: {
    delete o->as<Class>();
    break;
  }
  case Type::Task: {
    delete o->as<Task>();
    break;
  }
  case Type::Instance: {
    delete o->as<Instance>();
    break;
  }
  case Type::Range: {
    delete o->as<Range>();
    break;
  }
  case Type::ArrayIterator: {
    delete o->as<ArrayIterator>();
    break;
  }
  case Type::MapIterator: {
    delete o->as<MapIterator>();
    break;
  }
  case Type::StringIterator: {
    delete o->as<StringIterator>();
    break;
  }
  case Type::Channel: {
    delete o->as<Channel>();
    break;
  }
  default:
    unreachable();
  }
}

VM::~VM() {
  if (DEBUG_GC)
    std::cout << "VM destructor:" << std::endl;
  while (handles != nullptr) {
    auto old = handles;
    handles = handles->next;
    delete old;
  }

  while (first_obj != nullptr) {
    auto old = first_obj;
    first_obj = first_obj->next;
    release(old);
  }

  for (auto efunc : efuncs)
    efunc.second.free_data(efunc.second.data);
}

Value VM::to_string(Value val) {
  if (val.is_int()) {
    char buffer[12];
    size_t len = static_cast<size_t>(sprintf(buffer, "%d", val.as_int()));
    return Value(allocate<String>(StringSlice{buffer, len}));
  } else if (val.is_float()) {
    auto f = val.as_float();
    if (std::isnan(f)) {
      const char *result = std::signbit(f) ? "-NaN" : "NaN";
      return Value(allocate<String>(StringSlice(result)));
    } else {
      char buffer[24];
      size_t len = static_cast<size_t>(sprintf(buffer, "%.14g", f));
      if (strspn(buffer, "0123456789-") == len) {
        buffer[len] = '.';
        buffer[len + 1] = '0';
        len += 2;
      }
      return Value(allocate<String>(StringSlice{buffer, len}));
    }
  } else if (val.is_object()) {
    if (val.as_object()->is<String>()) {
      return val;
    } else if (val.as_object()->is<Symbol>()) {
      return Value(
          allocate<String>(StringSlice(*val.as_object()->as<Symbol>())));
    } else {
      std::ostringstream os;
      os << val;
      auto s = os.str();
      return Value(allocate<String>(StringSlice{s.data(), s.length()}));
    }
  } else if (val.is_true()) {
    return Value(allocate<String>("true"));
  } else if (val.is_false()) {
    return Value(allocate<String>("false"));
  } else if (val.is_null()) {
    return Value(allocate<String>("null"));
  } else {
    unreachable();
  }
}

void VM::collect() {
  if (DEBUG_GC)
    std::cout << "Starting GC\nBytes allocated before: " << bytes_allocated
              << std::endl;
  bytes_allocated = 0;

  // Mark roots
  {
    mark(builtin_classes.Object);
    mark(builtin_classes.Class_);
    mark(builtin_classes.Int);
    mark(builtin_classes.Float);
    mark(builtin_classes.Bool);
    mark(builtin_classes.Null);
    mark(builtin_classes.String);
    mark(builtin_classes.Symbol);
    mark(builtin_classes.Array);
    mark(builtin_classes.Map);
    mark(builtin_classes.Function);
    mark(builtin_classes.Module);
    mark(builtin_classes.Task);
    mark(builtin_classes.Range);
    mark(builtin_classes.ArrayIterator);
    mark(builtin_classes.MapIterator);
    mark(builtin_classes.StringIterator);
    mark(builtin_classes.Channel);
    mark(builtin_symbols.construct);
    mark(builtin_symbols.message);
    mark(builtin_symbols.stack);
    mark(builtin_symbols.task);
    mark(builtin_symbols.killed);
  }

  auto current_handle = handles;
  while (current_handle != nullptr) {
    mark(current_handle->object);
    current_handle = current_handle->next;
  }
  for (auto root : temp_roots)
    if (root.is_object())
      mark(root.as_object());
  for (auto v : module_variables) {
    if (v.is_object())
      mark(v.as_object());
  }
  for (auto module : modules) {
    mark(module.first);
    mark(module.second);
  }
  if (return_value.is_object())
    mark(return_value.as_object());
  mark(last_native_function);
  mark(current_task);
  mark(main_task);
  for (auto efunc : efuncs)
    mark(efunc.first);
  for (auto entry : tasks_queue) {
    mark(entry.task);
    if (entry.accumulator.is_object())
      mark(entry.accumulator.as_object());
  }
  while (!greyobjects.empty()) {
    Object *o = greyobjects.back();
    greyobjects.pop_back();
    trace(o);
  }

  threshhold = bytes_allocated * HEAP_GROWTH_FACTOR;
  // Sweep white objects
  Object **obj = &first_obj;
  while (*obj != nullptr) {
    if (!((*obj)->is_dark)) {
      auto to_free = *obj;
      *obj = to_free->next;
      release(to_free);
    } else {
      (*obj)->is_dark = false;
      obj = &(*obj)->next;
    }
  }
  if (DEBUG_GC)
    std::cout << "Bytes allocated after: " << bytes_allocated << std::endl;
}

void VM::mark(Object *o) {
  if (o != nullptr) {
    if (o->is_dark)
      return;
    o->is_dark = true;
    greyobjects.push_back(o);
  }
}

void VM::trace(Object *o) {
  switch (o->type) {
  case Type::Array:
    for (auto v : o->as<Array>()->inner) {
      if (v.is_object())
        mark(v.as_object());
    }
    bytes_allocated += sizeof(Array);
    break;
  case Type::Map:
    for (auto pair : o->as<Map>()->inner) {
      if (pair.first.is_object())
        mark(pair.first.as_object());
      if (pair.second.is_object())
        mark(pair.second.as_object());
    }
    bytes_allocated += sizeof(Map);
    break;
  case Type::FunctionInfo:
    for (auto constant : o->as<FunctionInfo>()->constants) {
      if (constant.is_object())
        mark(constant.as_object());
    }
    bytes_allocated += sizeof(FunctionInfo);
    break;
  case Type::String:
    bytes_allocated += size(o->as<String>());
    break;
  case Type::Symbol:
    bytes_allocated += size(o->as<Symbol>());
    break;
  case Type::Function:
    bytes_allocated += size(o->as<Function>());
    mark(o->as<Function>()->function_info);
    for (size_t i = 0; i < o->as<Function>()->num_upvalues; i++) {
      mark(o->as<Function>()->upvalues[i]);
    }
    break;
  case Type::UpValue:
    bytes_allocated += sizeof(UpValue);
    if (o->as<UpValue>()->closed.is_object())
      mark(o->as<UpValue>()->closed.as_object());
    break;
  case Type::NativeFunction:
    bytes_allocated += sizeof(NativeFunction);
    break;
  case Type::Module:
    bytes_allocated += sizeof(Module);
    for (auto &pair : o->as<Module>()->module_variables) {
      mark(pair.first);
    }
    break;
  case Type::Class:
    bytes_allocated += sizeof(Class);
    for (auto pair : o->as<Class>()->methods) {
      mark(pair.first);
      mark(pair.second);
    }
    break;
  case Type::Task: {
    bytes_allocated += sizeof(Task);
    auto task = o->as<Task>();
    if (task->uncaught_exception.is_object())
      mark(task->uncaught_exception.as_object());
    for (auto v = task->stack.get(); v < task->stack_top; v++)
      if (v->is_object())
        mark(v->as_object());
    for (auto frame : task->frames)
      mark(frame.f);
    auto upvalue = task->open_upvalues;
    while (upvalue != nullptr) {
      mark(upvalue);
      upvalue = upvalue->next;
    }
    for (auto chan : task->monitors)
      mark(chan);
    for (auto link : task->links)
      mark(link);
    mark(task->name);
    break;
  }
  case Type::Instance:
    mark(o->as<Instance>()->class_);
    for (auto pair : o->as<Instance>()->properties) {
      mark(pair.first);
      if (pair.second.is_object())
        mark(pair.second.as_object());
    }
    bytes_allocated += sizeof(Instance);
    break;
  case Type::Range:
    bytes_allocated += sizeof(Range);
    break;
  case Type::ArrayIterator:
    bytes_allocated += sizeof(ArrayIterator);
    mark(o->as<ArrayIterator>()->array);
    break;
  case Type::MapIterator: {
    bytes_allocated += sizeof(MapIterator);
    auto mi = o->as<MapIterator>();
    mark(mi->map);
    if (mi->last_key.is_object())
      mark(mi->last_key.as_object());
    break;
  }
  case Type::StringIterator:
    bytes_allocated += sizeof(StringIterator);
    mark(o->as<StringIterator>()->string);
    break;
  case Type::Channel:
    bytes_allocated += sizeof(Channel);
    for (auto val : o->as<Channel>()->queue)
      if (val.is_object())
        mark(val.as_object());
    for (auto waiter : o->as<Channel>()->wait_list)
      mark(waiter);
    break;
  default:
    unreachable();
  }
}

static uint32_t get_line_number(FunctionInfo *f, const uint8_t *ip) {
  uint32_t instruction = static_cast<uint32_t>(ip - f->bytecode.data());
  uint32_t start = 0;
  uint32_t end = static_cast<uint32_t>(f->lines.size() - 1);
  for (;;) {
    uint32_t mid = (start + end) / 2;
    LineInfo *line = &f->lines[mid];
    if (instruction < line->offset) {
      end = mid - 1;
    } else if (mid == f->lines.size() - 1 ||
               instruction < f->lines[mid + 1].offset) {
      return line->line;
    } else {
      start = mid + 1;
    }
  }
}

std::string VM::generate_stack_trace(bool include_native_function,
                                     uint32_t depth) {
  std::ostringstream os;
  if (include_native_function && last_native_function != nullptr) {
    os << "at " << last_native_function->name << " ("
       << last_native_function->module_name << ")\n";
    last_native_function = nullptr;
  }
  if (depth > current_task->frames.size())
    return "";
  for (auto frame = current_task->frames.rbegin() + depth;
       frame != current_task->frames.rend(); frame++) {
    os << "at " << frame->f->function_info->name << " ("
       << frame->f->function_info->module << ':'
       << get_line_number(frame->f->function_info, frame->ip - 1) << ")\n";
  }
  return os.str();
}

const uint8_t *VM::throw_(const uint8_t *ip, const char *type) {
  auto message = throw_message.str();
  throw_message.str("");
  current_task->frames.back().ip = ip;
  return throw_(create_error(type, message));
}

const uint8_t *VM::throw_(Value v) {
  auto task = current_task;
  do {
    auto frame = task->frames.back();
    auto bytecode = frame.f->function_info->bytecode.data();
    auto handlers = frame.f->function_info->exception_handlers;
    auto ip = frame.ip;
    auto bp = frame.bp;
    for (auto handler : handlers) {
      if (ip > bytecode + handler.try_begin &&
          ip <= bytecode + handler.try_end) {
        CLOSE(handler.error_reg);
        bp[handler.error_reg] = v;
        task->stack_top = frame.f->function_info->max_registers + bp;
        return bytecode + handler.catch_begin;
      }
    }
    CLOSE(0);
    task->frames.pop_back();
  } while (!task->frames.empty());
  task->stack_top = task->stack.get();
  return_value = v;
  return nullptr;
}

bool VM::declare_native_function(std::string module, std::string name,
                                 bool exported, uint8_t arity,
                                 NativeFunctionCallback *callback) const {
  if (!add_module_variable(module, name, false, exported))
    return false;
  auto n = const_cast<VM *>(this)->allocate<NativeFunction>(
      callback, std::move(name), std::move(module), arity);
  module_variables[module_variables.size() - 1] = Value(n);
  return true;
}

Function *VM::make_function(Value *bp, FunctionInfo *function_info) {
  auto function = static_cast<Function *>(mi_malloc(
      sizeof(Function) + sizeof(UpValue *) * function_info->upvalues.size()));
  function->function_info = function_info;
  function->super_class = nullptr;
  if (function == nullptr)
    throw std::bad_alloc();
  function->num_upvalues = 0;
  temp_roots.push_back(Value(manage(function)));
  for (auto upvalue : function_info->upvalues) {
    if (upvalue.is_local) {
      auto loc = &bp[upvalue.index];
      UpValue *prev = nullptr;
      UpValue *upval;
      auto curr = current_task->open_upvalues;
      while (curr != nullptr && curr->location > loc) {
        prev = curr;
        curr = curr->next;
      }
      if (curr != nullptr && curr->location == loc) {
        upval = curr;
      } else {
        upval = allocate<UpValue>(loc);
        upval->next = curr;
        if (prev == nullptr) {
          current_task->open_upvalues = upval;
        } else {
          prev->next = upval;
        }
      }
      function->upvalues[function->num_upvalues++] = upval;
    } else {
      function->upvalues[function->num_upvalues++] =
          current_task->frames.back().f->upvalues[upvalue.index];
    }
  }
  temp_roots.pop_back();
  return function;
}

bool VM::module_exists(StringSlice module_name) const {
  return modules.find(module_name) != modules.end();
}

void VM::create_module(StringSlice module_name) const {
  if (!module_exists(module_name)) {
    auto this_ = const_cast<VM *>(this);
    auto name = this_->allocate<String>(module_name);
    this_->temp_roots.push_back(Value(name));
    this_->modules.insert({name, this_->allocate<Module>(std::string(
                                     module_name.data, module_name.len))});
    this_->temp_roots.pop_back();
  }
}

void VM::create_module_with_prelude(StringSlice module_name) const {
  if (!module_exists(module_name)) {
    auto this_ = const_cast<VM *>(this);
    auto module =
        this_->allocate<Module>(std::string(module_name.data, module_name.len));
    this_->temp_roots.push_back(Value(module));
    auto name = this_->allocate<String>(module_name);
    this_->temp_roots.pop_back();
    this_->modules.insert({name, module});
    auto prelude = modules.find(StringSlice("<prelude>"))->second;
    for (auto &pair : prelude->module_variables)
      if (pair.second.exported) {
        module->module_variables.insert(
            {pair.first,
             ModuleVariable{static_cast<uint32_t>(module_variables.size()),
                            false, false}});
        module_variables.push_back(module_variables[pair.second.position]);
      }
  }
}

Module *VM::get_module(StringSlice module_name) const {
  auto module_iter = modules.find(module_name);
  if (module_iter == modules.end())
    return nullptr;
  else
    return module_iter->second;
}

Class *VM::get_class(Value v) const {
  if (v.is_object()) {
    auto o = v.as_object();
    switch (o->type) {
    case Type::Class:
      return builtin_classes.Class_;
    case Type::String:
      return builtin_classes.String;
    case Type::Symbol:
      return builtin_classes.Symbol;
    case Type::Array:
      return builtin_classes.Array;
    case Type::Map:
      return builtin_classes.Map;
    case Type::Function:
      return builtin_classes.Function;
    case Type::NativeFunction:
      return builtin_classes.Function;
    case Type::Module:
      return builtin_classes.Module;
    case Type::Task:
      return builtin_classes.Task;
    case Type::Range:
      return builtin_classes.Range;
    case Type::ArrayIterator:
      return builtin_classes.ArrayIterator;
    case Type::MapIterator:
      return builtin_classes.MapIterator;
    case Type::StringIterator:
      return builtin_classes.StringIterator;
    case Type::Instance:
      return o->as<Instance>()->class_;
    case Type::Channel:
      return builtin_classes.Channel;
    default:
      unreachable();
    }
  } else if (v.is_int())
    return builtin_classes.Int;
  else if (v.is_float())
    return builtin_classes.Float;
  else if (v.is_null())
    return builtin_classes.Null;
  else if (v.is_true())
    return builtin_classes.Bool;
  else if (v.is_false())
    return builtin_classes.Bool;
  else
    unreachable();
}

Value *Task::grow_stack(Value *bp, size_t extra_needed) {
  size_t needed = stack_size + extra_needed;
  size_t new_capacity = power_of_two_ceil(needed);
  auto old_stack = std::move(stack);
  stack = std::unique_ptr<Value[]>(new Value[new_capacity]);
  memcpy(stack.get(), old_stack.get(), stack_size * sizeof(Value));
  stack_size = new_capacity;
  stack_top = stack.get() + (stack_top - old_stack.get());
  for (auto &frame : frames) {
    frame.bp = stack.get() + (frame.bp - old_stack.get());
  }
  for (auto upvalue = open_upvalues; upvalue != nullptr;
       upvalue = upvalue->next) {
    upvalue->location = stack.get() + (upvalue->location - old_stack.get());
  }
  return stack.get() + (bp - old_stack.get());
}

bool VM::create_efunc(StringSlice name, EFuncCallback *callback, Data *data,
                      FreeDataCallback *free_data) const {
  if (efuncs.find(name) != efuncs.end())
    return false;
  auto this_ = const_cast<VM *>(this);
  auto name_sym = this_->intern(name);
  this_->efuncs.insert({name_sym, EFunc{callback, data, free_data}});
  return true;
}

Value VM::create_error(StringSlice type, StringSlice message) {
  return create_error(StringSlice("<prelude>"), type, message);
}

Value VM::create_error(StringSlice module, StringSlice type,
                       StringSlice message) {
  try {
    auto class_val =
        module_variables[get_module_variable(module, type).position];
    if (class_val.is_object() && class_val.as_object()->is<Class>()) {
      Class *class_ = class_val.as_object()->as<Class>();
      if (class_->is_native)
        return Value::null();
      auto error = allocate<Instance>();
      error->class_ = class_;
      temp_roots.push_back(Value(error));
      error->properties.insert(
          {builtin_symbols.message, Value(allocate<String>(message))});
      auto stack_trace = generate_stack_trace(true, 0);
      error->properties.insert(
          {builtin_symbols.stack,
           Value(allocate<String>(std::move(stack_trace)))});
      error->properties.insert({builtin_symbols.task, Value(current_task)});
      temp_roots.pop_back();
      return Value(error);
    } else
      return Value::null();
  } catch (...) {
    return Value::null();
  }
}

static bool is_descendant(Class *base, Class *c) {
  if (c == nullptr)
    return false;
  if (c == base)
    return true;
  else
    return is_descendant(base, c->super);
}

std::string VM::report_error(Value error) {
  auto error_class_val =
      module_variables[get_module_variable("<prelude>", "Error").position];
  if (error_class_val.is_object() && error_class_val.as_object()->is<Class>()) {
    auto error_class = error_class_val.as_object()->as<Class>();
    if (error_class->is_native)
      throw std::runtime_error("Expect Error class to not be native");
    auto class_ = get_class(error);
    if (is_descendant(error_class, class_)) {
      std::ostringstream os;
      auto error_object = error.as_object()->as<Instance>();
      auto task_iter = error_object->properties.find(builtin_symbols.task);
      if (task_iter != error_object->properties.end()) {
        os << "In " << task_iter->second << " ";
      }
      os << class_->name << ": ";
      auto message_iter =
          error_object->properties.find(builtin_symbols.message);
      if (message_iter != error_object->properties.end()) {
        auto message = message_iter->second;
        if (message.is_object() && message.as_object()->is<String>())
          os << StringSlice(*message.as_object()->as<String>());
        else
          os << message;
      }
      os << '\n';
      auto stack_iter = error_object->properties.find(builtin_symbols.stack);
      if (stack_iter != error_object->properties.end()) {
        auto stack = stack_iter->second;
        if (stack.is_object() && stack.as_object()->is<String>())
          os << StringSlice(*stack.as_object()->as<String>());
        else
          os << stack;
      }
      return os.str();
    } else {
      std::ostringstream os;
      os << error;
      return os.str();
    }
  } else {
    throw std::runtime_error("Expect Error to be a class");
  }
}

Task::Task(Function *f)
    : status(VMStatus::Suspend), uncaught_exception(Value(nullptr)),
      open_upvalues(nullptr), name(nullptr) {
  stack_size = f->function_info->max_registers;
  if (stack_size == 0)
    stack_size = 1;
  stack = std::unique_ptr<Value[]>(new Value[stack_size]);
  stack_top = stack.get();
  for (size_t i = 0; i < stack_size; i++)
    stack[i] = Value(nullptr);
  frames.push_back(Frame{&stack[0], f, f->function_info->bytecode.data()});
}

void Channel::send(Value v, VM *vm) {
  if (wait_list.empty())
    queue.push_back(v);
  else {
    while (wait_list.back()->status == VMStatus::Error)
      wait_list.pop_back();
    vm->tasks_queue.push_back({wait_list.back(), v, false});
    wait_list.pop_back();
  }
}

void VM::kill(Task *task, Value uncaught_exception) {
  if (task->status != VMStatus::Suspend)
    return;
  task->status = VMStatus::Error;
  task->uncaught_exception = uncaught_exception;
  for (auto link : task->links)
    kill(link, uncaught_exception);
  for (auto chan : task->monitors)
    chan->send(Value(task), this);
}

rust::String VM::kill_main_task(StringSlice error, StringSlice message) const {
  auto this_ = const_cast<VM *>(this);
  if (main_task == nullptr) {
    throw std::runtime_error("No main task is there");
  }
  this_->current_task = main_task;
  auto err_val = this_->create_error(error, message);
  this_->current_task = nullptr;
  this_->kill(main_task, err_val);
  this_->main_task = nullptr;
  auto s = this_->report_error(err_val);
  return rust::String(std::move(s));
}

TaskHandle::TaskHandle(VM *vm, Task *task) {
  handle = vm->make_handle(task);
  this->vm = vm;
}

void TaskHandle::release() {
  vm->release(handle);
  handle = nullptr;
}

VMStatus TaskHandle::resume(EFuncCallback *callback, Data *data) {
  auto task = handle->object;
  if (!task->waiting_for_rust_future)
    return task->status;
  auto old_stack_top = task->stack_top;
  VMStatus status = callback(EFuncContext(vm, task->stack_top, task), data);
  auto accumulator = Value::null();
  if (task->stack_top != old_stack_top) {
    accumulator = *(task->stack_top - 1);
  }
  auto frame = task->frames.back();
  task->stack_top = frame.bp + frame.f->function_info->max_registers;
  vm->tasks_queue.push_back(
      TaskQueueEntry{task, accumulator, status == VMStatus::Error});
  task->waiting_for_rust_future = false;
  return vm->run();
}
} // namespace neptune_vm
