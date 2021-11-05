#include "checked_arithmetic.cc"
#include "neptune-vm.h"

#if defined(__GNUC__) || defined(__clang__)
#define COMPUTED_GOTO
#endif

constexpr uint32_t WIDE_OFFSET =
    static_cast<uint32_t>(neptune_vm::Op::Exit) + 1;
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
#define CLOSE(n) close(&bp[n])

#define CALL(n)                                                                \
  do {                                                                         \
    constants = f->function_info->constants.data();                            \
    stack_top = bp + f->function_info->max_registers;                          \
    if (stack_top > stack.get() + STACK_SIZE)                                  \
      PANIC("Stack overflow");                                                 \
    ip = f->function_info->bytecode.data();                                    \
    for (size_t i = n; i < f->function_info->max_registers; i++)               \
      bp[i] = Value::empty();                                                  \
    frames[num_frames++] = Frame{bp, f, ip};                                   \
  } while (0)

namespace neptune_vm {
VMStatus VM::run(Function *f) {
  if (is_running)
    throw std::runtime_error("Cannot call run() while VM is already running");
  is_running = true;
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
  stack_trace = "";
  return_value = Value::null();
  Value accumulator = Value::null();
  Value *bp = &stack[0];
  Value *constants;
  const uint8_t *ip = nullptr;
  static_assert(
      STACK_SIZE > 65536,
      "Stack size must be greater than the maximum number of registers");
#define PANIC(x) unreachable()
  CALL(0);
#undef PANIC

#define PANIC(fmt)                                                             \
  do {                                                                         \
    panic_message << fmt;                                                      \
    if ((ip = panic(ip)) != nullptr) {                                         \
      bp = frames[num_frames - 1].bp;                                          \
      auto f = frames[num_frames - 1].f;                                       \
      constants = f->function_info->constants.data();                          \
      DISPATCH();                                                              \
    } else {                                                                   \
      goto panic_end;                                                          \
    }                                                                          \
  } while (0)

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
void VM::close(Value *last) {
  while (open_upvalues != nullptr && open_upvalues->location >= last) {
    open_upvalues->closed = *open_upvalues->location;
    open_upvalues->location = &open_upvalues->closed;
    open_upvalues = open_upvalues->next;
  }
}
bool VM::add_module_variable(StringSlice module, StringSlice name,
                             bool mutable_) const {
  auto module_iter = modules.find(module);
  if (module_iter == modules.end())
    return false;
  else {
    auto module = module_iter->second;
    if (!module->module_variables
             .insert(
                 {const_cast<VM *>(this)->intern(name),
                  ModuleVariable{static_cast<uint32_t>(module_variables.size()),
                                 mutable_}})
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
    if (n->free_data != nullptr)
      n->free_data(n->data);
    delete o;
  } break;
  case Type::Module: {
    delete o->as<Module>();
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
  auto current_handle = handles;
  while (current_handle != nullptr) {
    grey(current_handle->object);
    current_handle = current_handle->next;
  }
  for (auto t : temp_roots)
    grey(t);
  for (auto v = stack.get(); v < stack_top; v++) {
    if (!v->is_empty() && v->is_object())
      grey(v->as_object());
  }
  for (auto v : module_variables) {
    if (v.is_object())
      grey(v.as_object());
  }
  for (auto module : modules) {
    grey(module.second);
  }
  for (auto frame = frames.get(); frame < frames.get() + num_frames; frame++) {
    grey(frame->f);
  }
  if (return_value.is_object())
    grey(return_value.as_object());
  // this might not be necessary since native functions are constants but just
  // in case
  if (last_native_function != nullptr)
    grey(last_native_function);

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
  if (o->is_dark)
    return;
  o->is_dark = true;
  greyobjects.push_back(o);
}

void VM::blacken(Object *o) {
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
    for (auto pair : o->as<Module>()->module_variables) {
      grey(pair.first);
    }
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
  for (size_t i = num_frames; i-- > 0;) {
    auto frame = frames[i];
    os << "at " << frame.f->function_info->name << " ("
       << frame.f->function_info->module << ':'
       << get_line_number(frame.f->function_info, frame.ip - 1) << ")\n";
  }
  return os.str();
}

const uint8_t *VM::panic(const uint8_t *ip) {
  auto message = panic_message.str();
  panic_message.str("");
  return panic(ip, Value(manage(String::from(message))));
}

const uint8_t *VM::panic(const uint8_t *ip, Value v) {
  frames[num_frames - 1].ip = ip;
  stack_trace = generate_stack_trace();
  do {
    auto frame = frames[num_frames - 1];
    auto bytecode = frame.f->function_info->bytecode.data();
    auto handlers = frame.f->function_info->exception_handlers;
    auto ip = frame.ip;
    auto bp = frame.bp;
    for (auto handler : handlers) {
      if (ip > bytecode + handler.try_begin &&
          ip <= bytecode + handler.try_end) {
        CLOSE(handler.error_reg);
        bp[handler.error_reg] = v;
        stack_top = frame.f->function_info->max_registers + bp;
        return bytecode + handler.catch_begin;
      }
    }
    CLOSE(0);
    num_frames--;
  } while (num_frames != 0);
  stack_top = stack.get();
  return_value = v;
  return nullptr;
}

bool VM::declare_native_function(StringSlice module, StringSlice name,
                                 uint8_t arity, uint16_t extra_slots,
                                 NativeFunctionCallback *callback,
                                 Data *data = nullptr,
                                 FreeDataCallback *free_data = nullptr) const {
  if (!add_module_variable(module, name, false))
    return false;
  auto n = new NativeFunction();
  n->arity = arity;
  n->max_slots = arity + extra_slots;
  n->inner = callback;
  n->data = data;
  n->free_data = free_data;
  n->name = std::string{name.data, name.len};
  n->module_name = std::string{module.data, module.len};
  module_variables[module_variables.size() - 1] = Value(n);
  const_cast<VM *>(this)->manage(n);
  return true;
}

namespace native_builtins {
bool disassemble(FunctionContext ctx, void *) {
  auto fn = ctx.slots[0];
  if (fn.is_object() && fn.as_object()->is<Function>()) {
    std::ostringstream os;
    disassemble(os, *fn.as_object()->as<Function>()->function_info);
    auto str = os.str();
    ctx.vm->return_value = Value(ctx.vm->manage(String::from(str)));
    return true;
  } else {
    std::ostringstream os;
    os << "Cannot disassemble " << fn.type_string();
    ctx.vm->return_value = Value(ctx.vm->manage(String::from(os.str())));
    return false;
  }
}

bool gc(FunctionContext ctx, void *) {
  ctx.vm->collect();
  return true;
}

bool _getModule(FunctionContext ctx, void *) {
  if (ctx.slots[0].is_object() && ctx.slots[0].as_object()->is<String>()) {
    auto module = ctx.vm->get_module(
        StringSlice(*ctx.slots[0].as_object()->as<String>()));
    if (module == NULL)
      ctx.vm->return_value = Value::null();
    else
      ctx.vm->return_value = Value(module);
    return true;
  } else {
    ctx.vm->return_value = Value(ctx.vm->manage(
        String::from(StringSlice("First argument must be a string"))));
    return false;
  }
}

bool _getCallerModule(FunctionContext ctx, void *) {
  if (ctx.vm->num_frames < 2) {
    ctx.vm->return_value =
        Value(ctx.vm->manage(String::from(StringSlice("No caller exists"))));
    return false;
  } else {
    ctx.vm->return_value = Value(
        ctx.vm->manage(String::from(ctx.vm->frames.get()[ctx.vm->num_frames - 2]
                                        .f->function_info->module)));
    return true;
  }
}
} // namespace native_builtins

void VM::declare_native_builtins() {
  create_module(StringSlice("vm"));
  declare_native_function(StringSlice("vm"), StringSlice("disassemble"), 1, 0,
                          native_builtins::disassemble);
  declare_native_function(StringSlice("vm"), StringSlice("gc"), 0, 0,
                          native_builtins::gc);
  declare_native_function(StringSlice("prelude"), StringSlice("_getModule"), 1,
                          0, native_builtins::_getModule);
  declare_native_function(StringSlice("prelude"),
                          StringSlice("_getCallerModule"), 0, 0,
                          native_builtins::_getCallerModule);
}

Value VM::make_function(Value *bp, Value constant) {
  auto info = constant.as_object()->as<FunctionInfo>();
  auto function = (Function *)malloc(sizeof(Function) +
                                     sizeof(UpValue *) * info->upvalues.size());
  function->function_info = info;
  if (function == nullptr)
    throw std::bad_alloc();
  function->num_upvalues = 0;
  temp_roots.push_back(static_cast<Object *>(manage(function)));
  for (auto upvalue : info->upvalues) {
    if (upvalue.is_local) {
      auto loc = &bp[upvalue.index];
      UpValue *prev = nullptr;
      UpValue *upval;
      auto curr = open_upvalues;
      while (curr != nullptr && curr->location > loc) {
        prev = curr;
        curr = curr->next;
      }
      if (curr != nullptr && curr->location == loc) {
        upval = curr;
      } else {
        upval = manage(new UpValue(loc));
        upval->next = curr;
        if (open_upvalues == nullptr) {
          open_upvalues = upval;
        } else {
          prev->next = upval;
        }
      }
      function->upvalues[function->num_upvalues++] = upval;
    } else {
      function->upvalues[function->num_upvalues++] =
          frames[num_frames - 1].f->upvalues[upvalue.index];
    }
  }
  temp_roots.pop_back();
  return Value(static_cast<Object *>(function));
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
    auto prelude = modules.find(StringSlice("prelude"))->second;
    for (auto pair : prelude->module_variables) {
      module->module_variables.insert(
          {pair.first,
           ModuleVariable{static_cast<uint32_t>(module_variables.size()),
                          false}});
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
} // namespace neptune_vm
