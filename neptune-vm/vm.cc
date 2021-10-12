#include "checked_arithmetic.cc"
#include "neptune-vm.h"
#include "object.h"
#include <sstream>

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
#define PANIC(fmt)                                                             \
  do {                                                                         \
    std::ostringstream stream;                                                 \
    stream << fmt;                                                             \
    auto str = stream.str();                                                   \
    auto stack_trace = panic(ip);                                              \
    return VMResult{VMStatus::Error, std::move(str), std::move(stack_trace)};  \
  } while (0)

#define CALL(n)                                                                \
  do {                                                                         \
    constants = f->constants.data();                                           \
    stack_top = bp + f->max_registers;                                         \
    if (stack_top > stack.get() + STACK_SIZE)                                  \
      PANIC("Stack overflow");                                                 \
    ip = f->bytecode.data();                                                   \
    for (size_t i = n; i < f->max_registers; i++)                              \
      bp[i] = Value::empty();                                                  \
    frames[num_frames++] = Frame{bp, f, ip};                                   \
  } while (0)

namespace neptune_vm {
VMResult VM::run(FunctionInfo *f, bool eval) {
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
  Value accumulator = Value::null();
  Value *bp = &stack[0];
  auto globals = this->globals.begin();
  Value *constants;
  const uint8_t *ip;
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
  if (eval) {
    std::ostringstream os;
    os << accumulator;
    return VMResult{VMStatus::Success, os.str(), ""};
  } else {
    return VMResult{VMStatus::Success, "", ""};
  }
}
#undef READ

void VM::add_global(StringSlice name) const {
  globals.push_back(Value::empty());
  global_names.push_back(std::string(name.data, name.len));
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
  bytes_allocated += sizeof(O);
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
                                     handles->next));
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
    sym->hash = StringHasher{}(static_cast<StringSlice>(*sym));
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
  if (o->is<String>())
    free(o);
  else if (o->is<Symbol>()) {
    symbols.erase(o->as<Symbol>());
    free(o);
  } else if (o->is<Array>()) {
    delete o->as<Array>();
  } else if (o->is<Map>()) {
    delete o->as<Map>();
  } else if (o->is<FunctionInfo>()) {
    delete o->as<FunctionInfo>();
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
    return Value(manage(String::from_string_slice(StringSlice{buffer, len})));
  } else if (val.is_float()) {
    auto f = val.as_float();
    if (std::isnan(f)) {
      const char *result = std::signbit(f) ? "-nan" : "nan";
      return Value(manage(
          String::from_string_slice(StringSlice{result, strlen(result)})));
    } else {
      char buffer[24];
      size_t len = static_cast<size_t>(sprintf(buffer, "%.14g", f));
      if (strspn(buffer, "0123456789-") == len) {
        buffer[len] = '.';
        buffer[len + 1] = '0';
        len += 2;
      }
      return Value(manage(String::from_string_slice(StringSlice{buffer, len})));
    }
  } else if (val.is_object()) {
    if (val.as_object()->is<String>()) {
      return val;
    } else if (val.as_object()->is<Symbol>()) {
      return Value(manage(String::from_string_slice(
          static_cast<StringSlice>(*val.as_object()->as<Symbol>()))));
    } else {
      std::ostringstream os;
      os << val;
      auto s = os.str();
      return Value(
          manage(String::from_string_slice(StringSlice{s.data(), s.length()})));
    }
  } else if (val.is_true()) {
    return Value(
        manage(String::from_string_slice(StringSlice{"true", strlen("true")})));
  } else if (val.is_false()) {
    return Value(manage(
        String::from_string_slice(StringSlice{"false", strlen("false")})));
  } else if (val.is_null()) {
    return Value(
        manage(String::from_string_slice(StringSlice{"null", strlen("null")})));
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
  for (auto i = stack.get(); i < stack_top; i++) {
    if (!i->is_empty() && i->is_object())
      grey(i->as_object());
  }
  for (auto i : globals) {
    if (!i.is_empty() && i.is_object())
      grey(i.as_object());
  }
  for (auto frame = frames.get(); frame < frames.get() + num_frames; frame++) {
    grey(frame->f);
  }

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
    bytes_allocated += sizeof(String);
    break;
  case Type::Symbol:
    bytes_allocated += sizeof(Symbol);
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

std::string VM::panic(const uint8_t *ip) {
  std::ostringstream os;
  frames[num_frames - 1].ip = ip;
  while (num_frames != 0) {
    auto frame = frames[num_frames - 1];
    os << "at " << frame.f->name << " (line "
       << get_line_number(frame.f, frame.ip - 1) << ")\n";
    num_frames--;
  }
  stack_top = stack.get();
  return os.str();
}
} // namespace neptune_vm
