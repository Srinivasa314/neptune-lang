#include "neptune-vm.h"
#include "object.h"
#include <SafeInt/SafeInt.hpp>
#include <iostream>
#include <sstream>

#if defined(__GNUC__) || defined(__clang__)
#define COMPUTED_GOTO
#endif

constexpr uint32_t WIDE_OFFSET =
    static_cast<uint32_t>(neptune_vm::Op::Exit) + 1;
constexpr uint32_t EXTRAWIDE_OFFSET = 2 * WIDE_OFFSET;

#define WIDE(x) (static_cast<uint32_t>(x) + WIDE_OFFSET)
#define EXTRAWIDE(x) (static_cast<uint32_t>(x) + EXTRAWIDE_OFFSET)

#define TODO()                                                                 \
  std::cout << "TODO at: " << __FILE__ << " : " << __LINE__ << std::endl;      \
  exit(1)

#define BINARY_OP_REGISTER(type, opname, intfn, op)                            \
  do {                                                                         \
    uint8_t reg = READ(type);                                                  \
    int res;                                                                   \
    if (accumulator.is_int() && bp[reg].is_int()) {                            \
      if (!intfn(bp[reg].as_int(), accumulator.as_int(), res))                 \
        PANIC("Cannot " #opname " "                                            \
              << bp[reg].as_int() << " and " << accumulator.as_int()           \
              << " as the result does not fit in an int");                     \
      accumulator = static_cast<Value>(res);                                   \
    } else if (accumulator.is_float() && bp[reg].is_float()) {                 \
      accumulator =                                                            \
          static_cast<Value>(bp[reg].as_float() op accumulator.as_float());    \
    } else if (accumulator.is_int() && bp[reg].is_float()) {                   \
      accumulator =                                                            \
          static_cast<Value>(bp[reg].as_float() op accumulator.as_int());      \
    } else if (accumulator.is_float() && bp[reg].is_int()) {                   \
      accumulator =                                                            \
          static_cast<Value>(bp[reg].as_int() op accumulator.as_float());      \
    } else {                                                                   \
      PANIC("Cannot " #opname " types" << bp[reg].type_string() << " and "     \
                                       << accumulator.type_string());          \
    }                                                                          \
    DISPATCH();                                                                \
  } while (0)

#define BINARY_OP_INT(type, opname, intfn, op)                                 \
  do {                                                                         \
    if (accumulator.is_int()) {                                                \
      int result;                                                              \
      int i = READ(type);                                                      \
      if (!intfn(accumulator.as_int(), i, result)) {                           \
        PANIC("Cannot " #opname " "                                            \
              << accumulator.as_int() << " and " << i                          \
              << " as the result does not fit in an int");                     \
      }                                                                        \
      accumulator = static_cast<Value>(result);                                \
    } else if (accumulator.is_float()) {                                       \
      accumulator = static_cast<Value>(accumulator.as_float() op READ(type));  \
    } else {                                                                   \
      PANIC("Cannot " #opname " types " << accumulator.type_string()           \
                                        << " and int");                        \
    }                                                                          \
    DISPATCH();                                                                \
  } while (0)

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

namespace neptune_vm {
#define PANIC(fmt)                                                             \
  do {                                                                         \
    std::ostringstream stream;                                                 \
    stream << fmt;                                                             \
    auto str = stream.str();                                                   \
    String *s = manage(                                                        \
        String::from_string_slice(StringSlice{str.data(), str.size()}));       \
    stack[0] = static_cast<Value>(s);                                          \
    return VMResult::Error;                                                    \
  } while (0)

VMResult VM::run(FunctionInfo *f) {
  Value *bp = &stack[0];
  stack_top = bp + f->max_registers;
  const uint8_t *ip = f->bytecode.data();
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
  INTERPRET_LOOP {
    HANDLER(Wide) : DISPATCH_WIDE();

    HANDLER(ExtraWide) : DISPATCH_EXTRAWIDE();

    HANDLER(LoadRegister) : accumulator = bp[READ(uint8_t)];
    DISPATCH();

    HANDLER(LoadInt) : accumulator = static_cast<Value>(READ(int8_t));
    DISPATCH();

    HANDLER(LoadNull) : accumulator = Value::null();
    DISPATCH();

    HANDLER(LoadTrue) : accumulator = Value::new_true();
    DISPATCH();

    HANDLER(LoadFalse) : accumulator = Value::new_false();
    DISPATCH();

    HANDLER(LoadConstant) : accumulator = f->constants[READ(uint8_t)];
    DISPATCH();

    HANDLER(StoreRegister) : bp[READ(uint8_t)] = accumulator;
    DISPATCH();

    HANDLER(Move) : {
      auto src = READ(uint8_t);
      auto dest = READ(uint8_t);
      bp[dest] = bp[src];
      DISPATCH();
    }

    HANDLER(LoadGlobal) : {
      auto g = globals[READ(uint8_t)].value;
      if (g.is_empty()) {
        PANIC("Cannot access uninitialized variable "
              << globals[READ(uint8_t)].name);
      } else {
        accumulator = g;
      }
    }
    DISPATCH();

    HANDLER(StoreGlobal) : globals[READ(uint8_t)].value = accumulator;
    DISPATCH();

    HANDLER(AddRegister) : BINARY_OP_REGISTER(uint8_t, add, SafeAdd, +);

    HANDLER(SubtractRegister)
        : BINARY_OP_REGISTER(uint8_t, subtract, SafeSubtract, -);

    HANDLER(MultiplyRegister)
        : BINARY_OP_REGISTER(uint8_t, multiply, SafeMultiply, *);

    HANDLER(DivideRegister)
        : BINARY_OP_REGISTER(uint8_t, divide, SafeDivide, /);

    HANDLER(ConcatRegister) : TODO();

    HANDLER(AddInt) : BINARY_OP_INT(int8_t, add, SafeAdd, +);

    HANDLER(SubtractInt) : BINARY_OP_INT(int8_t, subtract, SafeSubtract, -);

    HANDLER(MultiplyInt) : BINARY_OP_INT(int8_t, multiply, SafeMultiply, *);

    HANDLER(DivideInt) : BINARY_OP_INT(int8_t, divide, SafeDivide, /);

    HANDLER(Negate) : if (accumulator.is_int()) {
      int result;
      if (!SafeNegation(accumulator.as_int(), result)) {
        PANIC("Cannot negate " << accumulator.as_int()
                               << " as the result cannot be stored in an int");
      }
      accumulator = static_cast<Value>(result);
    }
    else if (accumulator.is_float()) {
      accumulator = static_cast<Value>(-accumulator.as_float());
    }
    else {
      PANIC("Cannot negate type " << accumulator.type_string());
    }
    DISPATCH();

    HANDLER(Call) : TODO();

    HANDLER(Call0Argument) : TODO();

    HANDLER(Call1Argument) : TODO();

    HANDLER(Call2Argument) : TODO();

    HANDLER(ToString) : TODO();

    HANDLER(Jump) : TODO();

    HANDLER(JumpBack) : TODO();

    HANDLER(JumpIfFalse) : TODO();

    HANDLER(Return) : TODO();

    HANDLER(Exit) : goto end;

#define STORER(n)                                                              \
  bp[n] = accumulator;                                                         \
  DISPATCH()

    HANDLER(StoreR0) : STORER(0);

    HANDLER(StoreR1) : STORER(1);

    HANDLER(StoreR2) : STORER(2);

    HANDLER(StoreR3) : STORER(3);

    HANDLER(StoreR4) : STORER(4);

    HANDLER(StoreR5) : STORER(5);

    HANDLER(StoreR6) : STORER(6);

    HANDLER(StoreR7) : STORER(7);

    HANDLER(StoreR8) : STORER(8);

    HANDLER(StoreR9) : STORER(9);

    HANDLER(StoreR10) : STORER(10);

    HANDLER(StoreR11) : STORER(11);

    HANDLER(StoreR12) : STORER(12);

    HANDLER(StoreR13) : STORER(13);

    HANDLER(StoreR14) : STORER(14);

    HANDLER(StoreR15) : STORER(15);

#undef STORER

#define LOADR(n)                                                               \
  accumulator = bp[n];                                                         \
  DISPATCH()

    HANDLER(LoadR0) : LOADR(0);

    HANDLER(LoadR1) : LOADR(1);

    HANDLER(LoadR2) : LOADR(2);

    HANDLER(LoadR3) : LOADR(3);

    HANDLER(LoadR4) : LOADR(4);

    HANDLER(LoadR5) : LOADR(5);

    HANDLER(LoadR6) : LOADR(6);

    HANDLER(LoadR7) : LOADR(7);

    HANDLER(LoadR8) : LOADR(8);

    HANDLER(LoadR9) : LOADR(9);

    HANDLER(LoadR10) : LOADR(10);

    HANDLER(LoadR11) : LOADR(11);

    HANDLER(LoadR12) : LOADR(12);

    HANDLER(LoadR13) : LOADR(13);

    HANDLER(LoadR14) : LOADR(14);

    HANDLER(LoadR15) : LOADR(15);

#undef LOADR

    WIDE_HANDLER(LoadRegister) : accumulator = bp[READ(uint16_t)];
    DISPATCH();

    WIDE_HANDLER(LoadInt) : accumulator = static_cast<Value>(READ(int16_t));
    DISPATCH();

    WIDE_HANDLER(LoadConstant) : accumulator = f->constants[READ(uint16_t)];
    DISPATCH();

    WIDE_HANDLER(StoreRegister) : bp[READ(uint16_t)] = accumulator;
    DISPATCH();

    WIDE_HANDLER(Move) : {
      auto src = READ(uint16_t);
      auto dest = READ(uint16_t);
      bp[dest] = bp[src];
      DISPATCH();
    }

    WIDE_HANDLER(LoadGlobal) : accumulator = globals[READ(uint16_t)].value;
    DISPATCH();

    WIDE_HANDLER(StoreGlobal) : globals[READ(uint16_t)].value = accumulator;
    DISPATCH();

    WIDE_HANDLER(AddRegister) : BINARY_OP_REGISTER(uint16_t, add, SafeAdd, +);

    WIDE_HANDLER(SubtractRegister)
        : BINARY_OP_REGISTER(uint16_t, subtract, SafeSubtract, -);

    WIDE_HANDLER(MultiplyRegister)
        : BINARY_OP_REGISTER(uint16_t, multiply, SafeMultiply, *);

    WIDE_HANDLER(DivideRegister)
        : BINARY_OP_REGISTER(uint16_t, divide, SafeDivide, /);

    WIDE_HANDLER(ConcatRegister) : TODO();

    WIDE_HANDLER(AddInt) : BINARY_OP_INT(int16_t, add, SafeAdd, +);

    WIDE_HANDLER(SubtractInt)
        : BINARY_OP_INT(int16_t, subtract, SafeSubtract, -);

    WIDE_HANDLER(MultiplyInt)
        : BINARY_OP_INT(int16_t, multiply, SafeMultiply, *);

    WIDE_HANDLER(DivideInt) : BINARY_OP_INT(int16_t, divide, SafeDivide, /);

    WIDE_HANDLER(Call) : TODO();

    WIDE_HANDLER(Call0Argument) : TODO();

    WIDE_HANDLER(Call1Argument) : TODO();

    WIDE_HANDLER(Call2Argument) : TODO();

    WIDE_HANDLER(Jump) : TODO();

    WIDE_HANDLER(JumpBack) : TODO();

    WIDE_HANDLER(JumpIfFalse) : TODO();
    EXTRAWIDE_HANDLER(LoadInt) : TODO();
    EXTRAWIDE_HANDLER(LoadGlobal) : TODO();
    EXTRAWIDE_HANDLER(StoreGlobal) : TODO();
    EXTRAWIDE_HANDLER(AddInt)
        : EXTRAWIDE_HANDLER(SubtractInt)
        : EXTRAWIDE_HANDLER(MultiplyInt)
        : EXTRAWIDE_HANDLER(DivideInt) : TODO();
    EXTRAWIDE_HANDLER(Jump)
        : EXTRAWIDE_HANDLER(JumpBack) : EXTRAWIDE_HANDLER(JumpIfFalse) : TODO();
    WIDE_HANDLER(ToString)
        : WIDE_HANDLER(Return)
        : WIDE_HANDLER(Exit)
        : WIDE_HANDLER(Wide)
        : WIDE_HANDLER(ExtraWide)
        : WIDE_HANDLER(LoadNull)
        : WIDE_HANDLER(LoadTrue)
        : WIDE_HANDLER(LoadFalse)
        : WIDE_HANDLER(Negate)
        : WIDE_HANDLER(StoreR0)
        : WIDE_HANDLER(StoreR1)
        : WIDE_HANDLER(StoreR2)
        : WIDE_HANDLER(StoreR3)
        : WIDE_HANDLER(StoreR4)
        : WIDE_HANDLER(StoreR5)
        : WIDE_HANDLER(StoreR6)
        : WIDE_HANDLER(StoreR7)
        : WIDE_HANDLER(StoreR8)
        : WIDE_HANDLER(StoreR9)
        : WIDE_HANDLER(StoreR10)
        : WIDE_HANDLER(StoreR11)
        : WIDE_HANDLER(StoreR12)
        : WIDE_HANDLER(StoreR13)
        : WIDE_HANDLER(StoreR14)
        : WIDE_HANDLER(StoreR15)
        : WIDE_HANDLER(LoadR0)
        : WIDE_HANDLER(LoadR1)
        : WIDE_HANDLER(LoadR2)
        : WIDE_HANDLER(LoadR3)
        : WIDE_HANDLER(LoadR4)
        : WIDE_HANDLER(LoadR5)
        : WIDE_HANDLER(LoadR6)
        : WIDE_HANDLER(LoadR7)
        : WIDE_HANDLER(LoadR8)
        : WIDE_HANDLER(LoadR9)
        : WIDE_HANDLER(LoadR10)
        : WIDE_HANDLER(LoadR11)
        : WIDE_HANDLER(LoadR12)
        : WIDE_HANDLER(LoadR13)
        : WIDE_HANDLER(LoadR14)
        : WIDE_HANDLER(LoadR15)
        : EXTRAWIDE_HANDLER(Wide)
        : EXTRAWIDE_HANDLER(ExtraWide)
        : EXTRAWIDE_HANDLER(LoadRegister)
        : EXTRAWIDE_HANDLER(LoadNull)
        : EXTRAWIDE_HANDLER(LoadTrue)
        : EXTRAWIDE_HANDLER(LoadFalse)
        : EXTRAWIDE_HANDLER(LoadConstant)
        : EXTRAWIDE_HANDLER(StoreRegister)
        : EXTRAWIDE_HANDLER(Move)
        : EXTRAWIDE_HANDLER(AddRegister)
        : EXTRAWIDE_HANDLER(SubtractRegister)
        : EXTRAWIDE_HANDLER(MultiplyRegister)
        : EXTRAWIDE_HANDLER(DivideRegister)
        : EXTRAWIDE_HANDLER(ConcatRegister)
        : EXTRAWIDE_HANDLER(Negate)
        : EXTRAWIDE_HANDLER(Call)
        : EXTRAWIDE_HANDLER(Call0Argument)
        : EXTRAWIDE_HANDLER(Call1Argument)
        : EXTRAWIDE_HANDLER(Call2Argument)
        : EXTRAWIDE_HANDLER(ToString)
        : EXTRAWIDE_HANDLER(Return)
        : EXTRAWIDE_HANDLER(Exit)
        : EXTRAWIDE_HANDLER(StoreR0)
        : EXTRAWIDE_HANDLER(StoreR1)
        : EXTRAWIDE_HANDLER(StoreR2)
        : EXTRAWIDE_HANDLER(StoreR3)
        : EXTRAWIDE_HANDLER(StoreR4)
        : EXTRAWIDE_HANDLER(StoreR5)
        : EXTRAWIDE_HANDLER(StoreR6)
        : EXTRAWIDE_HANDLER(StoreR7)
        : EXTRAWIDE_HANDLER(StoreR8)
        : EXTRAWIDE_HANDLER(StoreR9)
        : EXTRAWIDE_HANDLER(StoreR10)
        : EXTRAWIDE_HANDLER(StoreR11)
        : EXTRAWIDE_HANDLER(StoreR12)
        : EXTRAWIDE_HANDLER(StoreR13)
        : EXTRAWIDE_HANDLER(StoreR14)
        : EXTRAWIDE_HANDLER(StoreR15)
        : EXTRAWIDE_HANDLER(LoadR0)
        : EXTRAWIDE_HANDLER(LoadR1)
        : EXTRAWIDE_HANDLER(LoadR2)
        : EXTRAWIDE_HANDLER(LoadR3)
        : EXTRAWIDE_HANDLER(LoadR4)
        : EXTRAWIDE_HANDLER(LoadR5)
        : EXTRAWIDE_HANDLER(LoadR6)
        : EXTRAWIDE_HANDLER(LoadR7)
        : EXTRAWIDE_HANDLER(LoadR8)
        : EXTRAWIDE_HANDLER(LoadR9)
        : EXTRAWIDE_HANDLER(LoadR10)
        : EXTRAWIDE_HANDLER(LoadR11)
        : EXTRAWIDE_HANDLER(LoadR12)
        : EXTRAWIDE_HANDLER(LoadR13)
        : EXTRAWIDE_HANDLER(LoadR14)
        : EXTRAWIDE_HANDLER(LoadR15) : unreachable();
  }
end:
  return VMResult::Success;
}

void VM::add_global(StringSlice name) const {
  globals.push_back(Global{std::string(name.data, name.len), Value::empty()});
}

std::unique_ptr<VM> new_vm() { return std::unique_ptr<VM>{new VM}; }

FunctionInfoWriter VM::new_function_info() const {
  auto this_ = const_cast<VM *>(this);
  return FunctionInfoWriter(this_->make_handle(new FunctionInfo), this);
}

template <typename O> O *VM::manage(O *t) {
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
        handles->next = new Handle<Object>(
            handles->next, static_cast<Object *>(object), nullptr));
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
    manage(sym);
    symbols.insert(sym);
    return sym;
  } else {
    return *reused_sym;
  }
}

void VM::release(Object *o) {
  // todo change this when more types are added
  if (o->is<String>())
    free(o);
  else if (o->is<Symbol>()) {
    symbols.erase(o->as<Symbol>());
    free(o);
  }
}

VM::~VM() {
  while (handles != nullptr) {
    auto old = handles;
    handles = handles->next;
    delete old;
  }

  while (first_obj != nullptr) {
    auto old = first_obj;
    first_obj = first_obj->next;
    delete old;
  }
}
} // namespace neptune_vm
