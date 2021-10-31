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
VMResult VM::run(Function *f, bool eval) {
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
  last_panic = "";
  Value accumulator = Value::null();
  Value *bp = &stack[0];
  auto globals = this->globals.begin();
  Value *constants;
  const uint8_t *ip = nullptr;
  static_assert(
      STACK_SIZE > 65536,
      "Stack size must be greater than the maximum number of registers");
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
  if (eval) {
    std::ostringstream os;
    os << accumulator;
    return VMResult(VMStatus::Success, os.str(), "");
  } else {
    return VMResult(VMStatus::Success, "", "");
  }
panic_end:
  is_running = false;
  return VMResult(VMStatus::Error, std::move(last_panic), stack_trace);
}
#undef READ
void VM::close(Value *last) {
  while (open_upvalues != nullptr && open_upvalues->location >= last) {
    open_upvalues->closed = *open_upvalues->location;
    open_upvalues->location = &open_upvalues->closed;
    open_upvalues = open_upvalues->next;
  }
}
bool VM::add_global(StringSlice name, bool mutable_) const {
  if (!global_names
           .insert({std::string(name.data, name.len),
                    Global{static_cast<uint32_t>(globals.size()), mutable_}})
           .second)
    return false;
  globals.push_back(Value::null());
  return true;
}

Global VM::get_global(StringSlice name) const {
  auto it = global_names.find(std::string(name.data, name.len));
  if (it == global_names.end())
    throw std::runtime_error("Global does not exist");
  return it->second;
}
std::unique_ptr<VM> new_vm() { return std::unique_ptr<VM>{new VM}; }

FunctionInfoWriter VM::new_function_info(StringSlice name,
                                         uint8_t arity) const {
  auto this_ = const_cast<VM *>(this);
  auto function_info = new FunctionInfo(name, arity);
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
      return Value(manage(String::from(StringSlice{result, strlen(result)})));
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
    return Value(manage(String::from(StringSlice{"true", strlen("true")})));
  } else if (val.is_false()) {
    return Value(manage(String::from(StringSlice{"false", strlen("false")})));
  } else if (val.is_null()) {
    return Value(manage(String::from(StringSlice{"null", strlen("null")})));
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
  for (auto i = stack.get(); i < stack_top; i++) {
    if (!i->is_empty() && i->is_object())
      grey(i->as_object());
  }
  for (auto i : globals) {
    grey_value(i);
  }
  for (auto frame = frames.get(); frame < frames.get() + num_frames; frame++) {
    grey(frame->f);
  }
  grey_value(return_value);

  // Blacken all objects
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

void VM::grey_value(Value v) {
  if (!v.is_empty() && v.is_object())
    grey(v.as_object());
}

void VM::blacken(Object *o) {
  switch (o->type) {
  case Type::Array:
    for (auto i : o->as<Array>()->inner) {
      if (i.is_object())
        grey(i.as_object());
    }
    bytes_allocated += sizeof(Array);
    break;
  case Type::Map:
    for (auto i : o->as<Map>()->inner) {
      if (i.first.is_object())
        grey(i.first.as_object());
      if (i.second.is_object())
        grey(i.second.as_object());
    }
    bytes_allocated += sizeof(Map);
    break;
  case Type::FunctionInfo:
    for (auto i : o->as<FunctionInfo>()->constants) {
      if (i.is_object())
        grey(i.as_object());
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
    grey_value(o->as<UpValue>()->closed);
    break;
  case Type::NativeFunction:
    bytes_allocated += sizeof(NativeFunction);
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

std::string VM::get_stack_trace() {
  std::ostringstream os;
  if (!temp_roots.empty() && temp_roots.back()->is<NativeFunction>()) {
    os << "at " << temp_roots.back()->as<NativeFunction>()->name << '\n';
    temp_roots.pop_back();
  }
  for (size_t i = num_frames; i-- > 0;) {
    auto frame = frames[i];
    os << "at " << frame.f->function_info->name << " (line "
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
  stack_trace = get_stack_trace();
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
  std::ostringstream os;
  os << v;
  last_panic = os.str();
  return nullptr;
}

bool VM::declare_native_function(StringSlice name, uint8_t arity,
                                 uint16_t extra_slots,
                                 bool (*inner)(FunctionContext ctx, void *data),
                                 void *data = nullptr,
                                 void (*free_data)(void *data) = nullptr) {
  if (!global_names
           .insert({std::string(name.data, name.len),
                    Global{static_cast<uint32_t>(globals.size()), false}})
           .second)
    return false;
  auto n = new NativeFunction();
  n->arity = arity;
  n->max_slots = arity + extra_slots;
  n->inner = inner;
  n->data = data;
  n->free_data = free_data;
  n->name = std::string{name.data, name.len};
  globals.push_back(Value(n));
  manage(n);
  return true;
}
namespace native_builtins {
bool print(FunctionContext ctx, void *data) {
  auto s = static_cast<StringSlice>(
      *(ctx.vm->to_string(ctx.slots[0]).as_object()->as<String>()));
  std::cout.write(s.data, s.len);
  std::cout << std::endl;
  return true;
}
bool dissasemble(FunctionContext ctx, void *data) {
  auto fn = ctx.slots[0];
  if (fn.is_object() && fn.as_object()->is<Function>()) {
    std::ostringstream os;
    disassemble(os, *fn.as_object()->as<Function>()->function_info);
    auto str = os.str();
    ctx.vm->return_value = Value(
        ctx.vm->manage(String::from(StringSlice(str.data(), str.length()))));
    return true;
  } else {
    ctx.vm->return_value = Value(ctx.vm->manage(
        String::from(StringSlice("Only functions can be dissasembled"))));
    return false;
  }
}
} // namespace native_builtins
} // namespace neptune_vm
