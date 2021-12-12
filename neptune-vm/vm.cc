#include "checked_arithmetic.cc"
#include "neptune-vm.h"
#include <cstring>

#if defined(__GNUC__) || defined(__clang__)
#define COMPUTED_GOTO
#endif

constexpr uint32_t WIDE_OFFSET =
    static_cast<uint32_t>(neptune_vm::Op::Panic) + 1;
constexpr uint32_t EXTRAWIDE_OFFSET = 2 * WIDE_OFFSET;

#define WIDE(x) (static_cast<uint32_t>(x) + WIDE_OFFSET)
#define EXTRAWIDE(x) (static_cast<uint32_t>(x) + EXTRAWIDE_OFFSET)

#ifdef COMPUTED_GOTO

#define HANDLER(x) __##x##_handler
#define WIDE_HANDLER(x) __##x##_wide_handler
#define EXTRAWIDE_HANDLER(x) __##x##_extrawide_handler

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

#define CALL(n)                                                                \
  do {                                                                         \
    constants = f->function_info->constants.data();                            \
    if (size_t(bp - task->stack.get()) + f->function_info->max_registers >     \
        task->stack_size / sizeof(Value))                                      \
      bp = task->grow_stack(bp,                                                \
                            f->function_info->max_registers * sizeof(Value));  \
    task->stack_top = bp + f->function_info->max_registers;                    \
    ip = f->function_info->bytecode.data();                                    \
    for (size_t i = n; i < f->function_info->max_registers; i++)               \
      bp[i] = Value::empty();                                                  \
    task->frames.push_back(Frame{bp, f, ip});                                  \
  } while (0)

#define PANIC(fmt)                                                             \
  do {                                                                         \
    panic_message << fmt;                                                      \
    if ((ip = panic(ip)) != nullptr) {                                         \
      bp = task->frames.back().bp;                                             \
      auto f = task->frames.back().f;                                          \
      constants = f->function_info->constants.data();                          \
      DISPATCH();                                                              \
    } else {                                                                   \
      goto panic_end;                                                          \
    }                                                                          \
  } while (0)

namespace neptune_vm {
VMStatus VM::run(Task *task, Function *f) {
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

  if (is_running)
    throw std::runtime_error("Cannot call run() while VM is already running");
  current_task = task;
  is_running = true;
  Value accumulator = Value::null();
  Value *bp = &task->stack[0];
  Value *constants;
  const uint8_t *ip = nullptr;
  CALL(0);

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

#define handler(op, impl) EXTRAWIDE_HANDLER(op) : unreachable()
#include "wide_handlers.h"
#undef handler

#define utype uint8_t
#define itype int8_t
#define handler(op, impl) HANDLER(op) : impl DISPATCH()
#include "extrawide_handlers.h"
#undef utype
#undef itype
#undef handler

#define utype uint16_t
#define itype int16_t
#define handler(op, impl) WIDE_HANDLER(op) : impl DISPATCH()
#include "extrawide_handlers.h"
#undef utype
#undef itype
#undef handler

#define utype uint32_t
#define itype int32_t
#define handler(op, impl) EXTRAWIDE_HANDLER(op) : impl DISPATCH()
#include "extrawide_handlers.h"
#undef utype
#undef itype
#undef handler

// Doubles performance for MSVC!!
#ifndef COMPUTED_GOTO
  default:
    unreachable();
#endif
  }
end:
  is_running = false;
  return_value = accumulator;
  return VMStatus::Success;
panic_end:
  is_running = false;
  return VMStatus::Error;
}
#undef READ
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
    if (!module->module_variables
             .insert(
                 {const_cast<VM *>(this)->intern(name),
                  ModuleVariable{static_cast<uint32_t>(module_variables.size()),
                                 mutable_, exported}})
             .second)
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
std::unique_ptr<VM> new_vm() { return std::unique_ptr<VM>{new VM}; }

FunctionInfoWriter VM::new_function_info(StringSlice module, StringSlice name,
                                         uint8_t arity) const {
  auto this_ = const_cast<VM *>(this);
  auto function_info = new FunctionInfo(module, name, arity);
  this_->manage(function_info);
  return FunctionInfoWriter(this_->make_handle(function_info), this);
}

template <typename O> O *VM::manage(O *t) {
  if (STRESS_GC || bytes_allocated > threshhold)
    collect();
  static_assert(std::is_base_of<Object, O>::value,
                "O must be a descendant of Object");
  bytes_allocated += size(t);
  auto o = reinterpret_cast<Object *>(t);
  o->type = O::type;
  switch (O::type) {
  case Type::Class:
    o->class_ = builtin_classes.Class_;
    break;
  case Type::String:
    o->class_ = builtin_classes.String;
    break;
  case Type::Symbol:
    o->class_ = builtin_classes.Symbol;
    break;
  case Type::Array:
    o->class_ = builtin_classes.Array;
    break;
  case Type::Map:
    o->class_ = builtin_classes.Map;
    break;
  case Type::Function:
    o->class_ = builtin_classes.Function;
    break;
  case Type::NativeFunction:
    o->class_ = builtin_classes.Function;
    break;
  case Type::Module:
    o->class_ = builtin_classes.Module;
    break;
  case Type::Task:
    o->class_ = builtin_classes.Task;
    break;
  case Type::Instance:
    o->class_ = builtin_classes.Object;
    break;
  default:
    o->class_ = nullptr;
  }
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
  else
    return reinterpret_cast<Handle<O> *>(
        handles = new Handle<Object>(nullptr, static_cast<Object *>(object),
                                     handles));
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
    Symbol *sym = static_cast<Symbol *>(malloc(sizeof(Symbol) + s.len));
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
    free(o);
    break;
  case Type::Symbol:
    symbols.erase(o->as<Symbol>());
    free(o);
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
    free(o);
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
    return Value(manage(String::from(StringSlice{buffer, len})));
  } else if (val.is_float()) {
    auto f = val.as_float();
    if (std::isnan(f)) {
      const char *result = std::signbit(f) ? "-nan" : "nan";
      return Value(manage(String::from(StringSlice(result))));
    } else {
      char buffer[24];
      size_t len = static_cast<size_t>(sprintf(buffer, "%.14g", f));
      if (strspn(buffer, "0123456789-") == len) {
        buffer[len] = '.';
        buffer[len + 1] = '0';
        len += 2;
      }
      return Value(manage(String::from(StringSlice{buffer, len})));
    }
  } else if (val.is_object()) {
    if (val.as_object()->is<String>()) {
      return val;
    } else if (val.as_object()->is<Symbol>()) {
      return Value(manage(String::from(*val.as_object()->as<Symbol>())));
    } else {
      std::ostringstream os;
      os << val;
      auto s = os.str();
      return Value(manage(String::from(StringSlice{s.data(), s.length()})));
    }
  } else if (val.is_true()) {
    return Value(manage(String::from(StringSlice("true"))));
  } else if (val.is_false()) {
    return Value(manage(String::from(StringSlice("false"))));
  } else if (val.is_null()) {
    return Value(manage(String::from(StringSlice("null"))));
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
    grey(builtin_classes.Object);
    grey(builtin_classes.Class_);
    grey(builtin_classes.Int);
    grey(builtin_classes.Float);
    grey(builtin_classes.Bool);
    grey(builtin_classes.Null);
    grey(builtin_classes.String);
    grey(builtin_classes.Symbol);
    grey(builtin_classes.Array);
    grey(builtin_classes.Map);
    grey(builtin_classes.Function);
    grey(builtin_classes.Module);
    grey(builtin_classes.Task);
    grey(builtin_symbols.construct);
  }

  auto current_handle = handles;
  while (current_handle != nullptr) {
    grey(current_handle->object);
    current_handle = current_handle->next;
  }
  for (auto root : temp_roots)
    if (root.is_object())
      grey(root.as_object());
  for (auto v : module_variables) {
    if (v.is_object())
      grey(v.as_object());
  }
  for (auto module : modules) {
    grey(module.second);
  }
  if (return_value.is_object())
    grey(return_value.as_object());
  // this might not be necessary since native functions are constants but just
  // in case
  grey(last_native_function);
  grey(current_task);
  for (auto efunc : efuncs)
    grey(efunc.first);

  while (!greyobjects.empty()) {
    Object *o = greyobjects.back();
    greyobjects.pop_back();
    blacken(o);
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

void VM::grey(Object *o) {
  if (o != nullptr) {
    if (o->is_dark)
      return;
    o->is_dark = true;
    greyobjects.push_back(o);
  }
}

void VM::blacken(Object *o) {
  grey(o->class_);
  switch (o->type) {
  case Type::Array:
    for (auto v : o->as<Array>()->inner) {
      if (v.is_object())
        grey(v.as_object());
    }
    bytes_allocated += sizeof(Array);
    break;
  case Type::Map:
    for (auto pair : o->as<Map>()->inner) {
      if (pair.first.is_object())
        grey(pair.first.as_object());
      if (pair.second.is_object())
        grey(pair.second.as_object());
    }
    bytes_allocated += sizeof(Map);
    break;
  case Type::FunctionInfo:
    for (auto constant : o->as<FunctionInfo>()->constants) {
      if (constant.is_object())
        grey(constant.as_object());
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
    grey(o->as<Function>()->function_info);
    for (size_t i = 0; i < o->as<Function>()->num_upvalues; i++) {
      grey(o->as<Function>()->upvalues[i]);
    }
    break;
  case Type::UpValue:
    bytes_allocated += sizeof(UpValue);
    if (o->as<UpValue>()->closed.is_object())
      grey(o->as<UpValue>()->closed.as_object());
    break;
  case Type::NativeFunction:
    bytes_allocated += sizeof(NativeFunction);
    break;
  case Type::Module:
    bytes_allocated += sizeof(Module);
    for (auto &pair : o->as<Module>()->module_variables) {
      grey(pair.first);
    }
    break;
  case Type::Class:
    bytes_allocated += sizeof(Class);
    for (auto pair : o->as<Class>()->methods) {
      grey(pair.first);
      grey(pair.second);
    }
    break;
  case Type::Task: {
    bytes_allocated += sizeof(Task);
    auto task = o->as<Task>();
    for (auto v = task->stack.get(); v < task->stack_top; v++)
      if (!v->is_empty() && v->is_object())
        grey(v->as_object());
    for (auto frame : task->frames)
      grey(frame.f);
    auto upvalue = task->open_upvalues;
    while (upvalue != nullptr) {
      grey(upvalue);
      upvalue = upvalue->next;
    }
  } break;
  case Type::Instance:
    for (auto pair : o->as<Instance>()->properties) {
      grey(pair.first);
      if (pair.second.is_object())
        grey(pair.second.as_object());
    }
    bytes_allocated += sizeof(Instance);
    break;
  default:
    break;
  }
}

static uint32_t get_line_number(FunctionInfo *f, const uint8_t *ip) {
  uint32_t instruction = ip - f->bytecode.data();
  uint32_t start = 0;
  uint32_t end = f->lines.size() - 1;
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

std::string VM::generate_stack_trace() {
  std::ostringstream os;
  if (last_native_function != nullptr) {
    os << "at " << last_native_function->name << " ("
       << last_native_function->module_name << ")\n";
    last_native_function = nullptr;
  }
  for (auto frame = current_task->frames.rbegin();
       frame != current_task->frames.rend(); frame++) {
    os << "at " << frame->f->function_info->name << " ("
       << frame->f->function_info->module << ':'
       << get_line_number(frame->f->function_info, frame->ip - 1) << ")\n";
  }
  return os.str();
}

const uint8_t *VM::panic(const uint8_t *ip) {
  auto message = panic_message.str();
  panic_message.str("");
  return panic(ip, Value(manage(String::from(message))));
}

const uint8_t *VM::panic(const uint8_t *ip, Value v) {
  auto task = current_task;
  task->frames.back().ip = ip;
  stack_trace = generate_stack_trace();
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

bool VM::declare_native_function(StringSlice module, StringSlice name,
                                 bool exported, uint8_t arity,
                                 NativeFunctionCallback *callback) const {
  if (!add_module_variable(module, name, false, exported))
    return false;
  auto n = new NativeFunction();
  n->arity = arity;
  n->inner = callback;
  n->name = std::string{name.data, name.len};
  n->module_name = std::string{module.data, module.len};
  module_variables[module_variables.size() - 1] =
      Value(const_cast<VM *>(this)->manage(n));
  return true;
}

namespace native_builtins {
bool object_tostring(VM *vm, Value *slots) {
  vm->return_value = vm->to_string(slots[0]);
  return true;
}
bool array_pop(VM *vm, Value *slots) {
  auto &arr = slots[0].as_object()->as<Array>()->inner;
  if (arr.empty()) {
    vm->return_value = Value(vm->manage(String::from("Array is empty")));
    return false;
  }
  vm->return_value = arr.back();
  arr.pop_back();
  return true;
}
bool array_push(VM *vm, Value *slots) {
  slots[0].as_object()->as<Array>()->inner.push_back(slots[1]);
  vm->return_value = Value::null();
  return true;
}
bool array_length(VM *vm, Value *slots) {
  vm->return_value = Value(
      static_cast<int32_t>(slots[0].as_object()->as<Array>()->inner.size()));
  return true;
}
bool sqrt(VM *vm, Value *slots) {
  auto num = slots[0];
  if (num.is_int()) {
    vm->return_value = Value(std::sqrt(num.as_int()));
    return true;
  } else if (num.is_float()) {
    vm->return_value = Value(std::sqrt(num.as_float()));
    return true;
  } else {
    std::ostringstream os;
    os << "Cannot disassemble " << num.type_string();
    vm->return_value = Value(vm->manage(String::from(os.str())));
    return false;
  }
}
bool disassemble(VM *vm, Value *slots) {
  auto fn = slots[0];
  if (fn.is_object() && fn.as_object()->is<Function>()) {
    std::ostringstream os;
    neptune_vm::disassemble(os, *fn.as_object()->as<Function>()->function_info);
    auto str = os.str();
    vm->return_value = Value(vm->manage(String::from(str)));
    return true;
  } else {
    std::ostringstream os;
    os << "Cannot disassemble " << fn.type_string();
    vm->return_value = Value(vm->manage(String::from(os.str())));
    return false;
  }
}

bool gc(VM *vm, Value *) {
  vm->collect();
  vm->return_value = Value::null();
  return true;
}

bool _getModule(VM *vm, Value *slots) {
  if (slots[0].is_object() && slots[0].as_object()->is<String>()) {
    auto module =
        vm->get_module(StringSlice(*slots[0].as_object()->as<String>()));
    if (module == nullptr)
      vm->return_value = Value::null();
    else
      vm->return_value = Value(module);
    return true;
  } else {
    vm->return_value = Value(vm->manage(
        String::from(StringSlice("First argument must be a string"))));
    return false;
  }
}

bool _getCallerModule(VM *vm, Value *) {
  if (vm->current_task->frames.size() < 2) {
    vm->return_value =
        Value(vm->manage(String::from(StringSlice("No caller exists"))));
    return false;
  } else {
    vm->return_value = Value(vm->manage(String::from(
        vm->current_task->frames[vm->current_task->frames.size() - 2]
            .f->function_info->module)));
    return true;
  }
}

bool ecall(VM *vm, Value *slots) {
  if (slots[0].is_object() && slots[0].as_object()->is<Symbol>()) {
    auto efunc_iter = vm->efuncs.find(slots[0].as_object()->as<Symbol>());
    if (efunc_iter == vm->efuncs.end()) {
      vm->return_value = Value(vm->manage(
          String::from(StringSlice("Attempt to call unknown efunc"))));
      return false;
    } else {
      auto task = vm->current_task;
      auto efunc = efunc_iter->second;
      auto old_stack_top = task->stack_top;
      task->stack_top = slots + 2;
      bool result =
          efunc.callback(EFuncContext(vm, task, slots + 1), efunc.data);
      if (task->stack_top == slots + 1)
        vm->return_value = Value::null();
      else {
        vm->return_value = slots[1];
      }
      task->stack_top = old_stack_top;
      return result;
    }
  } else {
    vm->return_value = Value(vm->manage(
        String::from(StringSlice("First argument must be a symbol"))));
    return false;
  }
}
} // namespace native_builtins

void VM::declare_native_builtins() {
#define DEFCLASS(Name)                                                         \
  builtin_classes.Name = manage(new Class());                                  \
  builtin_classes.Name->name = #Name;                                          \
  builtin_classes.Name->super = builtin_classes.Object;                        \
  add_module_variable(StringSlice("<prelude>"), StringSlice(#Name), false,     \
                      true);                                                   \
  module_variables[module_variables.size() - 1] = Value(builtin_classes.Name);
  DEFCLASS(Object)
  builtin_classes.Object->super = nullptr;
  builtin_classes.Class_ = manage(new Class());
  builtin_classes.Class_->name = "Class";
  builtin_classes.Class_->super = builtin_classes.Object;
  add_module_variable(StringSlice("<prelude>"), StringSlice("Class"), false,
                      true);
  module_variables[module_variables.size() - 1] = Value(builtin_classes.Class_);
  DEFCLASS(Int)
  DEFCLASS(Float)
  DEFCLASS(Bool)
  DEFCLASS(Null)
  DEFCLASS(String)
  DEFCLASS(Symbol)
  DEFCLASS(Array)
  DEFCLASS(Map)
  DEFCLASS(Function)
  DEFCLASS(Module)
#undef DEFCLASS
  temp_roots.push_back(Value(intern(StringSlice("toString"))));
  builtin_classes.Object->methods.insert(
      {intern(StringSlice("toString")),
       manage(new NativeFunction(native_builtins::object_tostring, "toString",
                                 0))});
  temp_roots.pop_back();
  temp_roots.push_back(Value(intern(StringSlice("push"))));
  builtin_classes.Array->methods.insert(
      {intern(StringSlice("push")),
       manage(new NativeFunction(native_builtins::array_push, "push", 1))});
  temp_roots.pop_back();
  temp_roots.push_back(Value(intern(StringSlice("pop"))));
  builtin_classes.Array->methods.insert(
      {intern(StringSlice("pop")),
       manage(new NativeFunction(native_builtins::array_pop, "pop", 0))});
  temp_roots.pop_back();
  temp_roots.push_back(Value(intern(StringSlice("length"))));
  builtin_classes.Array->methods.insert(
      {intern(StringSlice("length")),
       manage(new NativeFunction(native_builtins::array_length, "length", 0))});
  temp_roots.pop_back();
  create_module(StringSlice("vm"));
  declare_native_function(StringSlice("vm"), StringSlice("disassemble"), true,
                          1, native_builtins::disassemble);
  declare_native_function(StringSlice("vm"), StringSlice("gc"), true, 0,
                          native_builtins::gc);
  declare_native_function(StringSlice("vm"), StringSlice("sqrt"), true, 1,
                          native_builtins::sqrt);
  declare_native_function(StringSlice("vm"), StringSlice("ecall"), true, 2,
                          native_builtins::ecall);
  declare_native_function(StringSlice("<prelude>"), StringSlice("_getModule"),
                          false, 1, native_builtins::_getModule);
  declare_native_function(StringSlice("<prelude>"),
                          StringSlice("_getCallerModule"), false, 0,
                          native_builtins::_getCallerModule);
}

Function *VM::make_function(Value *bp, FunctionInfo *function_info) {
  auto function = (Function *)malloc(
      sizeof(Function) + sizeof(UpValue *) * function_info->upvalues.size());
  function->function_info = function_info;
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
        upval = manage(new UpValue(loc));
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
    this_->modules.insert({std::string(module_name.data, module_name.len),
                           this_->manage(new Module(std::string(
                               module_name.data, module_name.len)))});
  }
}

void VM::create_module_with_prelude(StringSlice module_name) const {
  if (!module_exists(module_name)) {
    auto this_ = const_cast<VM *>(this);
    auto module = this_->manage(
        new Module(std::string(module_name.data, module_name.len)));
    this_->modules.insert(
        {std::string(module_name.data, module_name.len), module});
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
  if (v.is_object())
    return v.as_object()->class_;
  else if (v.is_int())
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

static size_t power_of_two_ceil(size_t n) {
  n--;
  n |= n >> 1;
  n |= n >> 2;
  n |= n >> 4;
  n |= n >> 8;
  n |= n >> 16;
  n++;
  return n;
}

Value *Task::grow_stack(Value *bp, size_t extra_needed) {
  size_t needed = stack_size + extra_needed;
  size_t new_capacity = power_of_two_ceil(needed);
  auto old_stack = std::move(stack);
  stack = std::unique_ptr<Value[]>(new Value[new_capacity / sizeof(Value)]);
  memcpy(stack.get(), old_stack.get(), stack_size);
  stack_size = new_capacity;
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

} // namespace neptune_vm
