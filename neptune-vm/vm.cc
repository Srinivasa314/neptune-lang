#include "neptune-vm.h"

#if defined(__GNUC__) || defined(__clang__)
#define COMPUTED_GOTO
#endif

constexpr uint32_t WIDE_OFFSET =
    static_cast<uint32_t>(neptune_vm::Op::Exit) + 1;
constexpr uint32_t EXTRAWIDE_OFFSET = 2 * WIDE_OFFSET;

#define WIDE(x) (static_cast<uint32_t>(x) + WIDE_OFFSET)
#define EXTRAWIDE(x) (static_cast<uint32_t>(x) + EXTRAWIDE_OFFSET)

#ifdef __GNUC__ // gcc or clang
void unreachable() { __builtin_unreachable(); }
#elif defined(_MSC_VER) // MSVC
void unreachable() { __assume(false); }
#else
void unreachable() { abort(); }
#endif

#ifdef COMPUTED_GOTO

#define HANDLER(x) __##x##_handler
#define WIDE_HANDLER(x) __##x##_wide_handler
#define EXTRAWIDE_HANDLER(x) __##x##_extrawide_handler

#define DISPATCH() goto *dispatch_table[*bytecode++]
#define DISPATCH_WIDE() goto *dispatch_table[WIDE(*bytecode++)]
#define DISPATCH_EXTRAWIDE() goto *dispatch_table[EXTRAWIDE(*bytecode++)]
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
  __op = *bytecode++;                                                          \
  goto loop

#define DISPATCH_WIDE()                                                        \
  __op = WIDE(*bytecode++);                                                    \
  goto loop

#define DISPATCH_EXTRAWIDE()                                                   \
  __op = EXTRAWIDE(*bytecode++);                                               \
  goto loop

#endif

namespace neptune_vm {
void VM::run(FunctionInfo *f) const {
  auto bytecode = f->bytecode.data();
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
    HANDLER(LoadRegister)
        : HANDLER(LoadInt)
        : HANDLER(LoadNull)
        : HANDLER(LoadTrue)
        : HANDLER(LoadFalse)
        : HANDLER(LoadConstant)
        : HANDLER(StoreRegister)
        : HANDLER(Move)
        : HANDLER(LoadGlobal)
        : HANDLER(StoreGlobal)
        : HANDLER(AddRegister)
        : HANDLER(SubtractRegister)
        : HANDLER(MultiplyRegister)
        : HANDLER(DivideRegister)
        : HANDLER(ConcatRegister)
        : HANDLER(AddInt)
        : HANDLER(SubtractInt)
        : HANDLER(MultiplyInt)
        : HANDLER(DivideInt)
        : HANDLER(Negate)
        : HANDLER(Call)
        : HANDLER(Call0Argument)
        : HANDLER(Call1Argument)
        : HANDLER(Call2Argument)
        : HANDLER(Less)
        : HANDLER(ToString)
        : HANDLER(Jump)
        : HANDLER(JumpBack)
        : HANDLER(JumpIfFalse)
        : HANDLER(Return)
        : HANDLER(Exit)
        : HANDLER(StoreR0)
        : HANDLER(StoreR1)
        : HANDLER(StoreR2)
        : HANDLER(StoreR3)
        : HANDLER(StoreR4)
        : HANDLER(StoreR5)
        : HANDLER(StoreR6)
        : HANDLER(StoreR7)
        : HANDLER(StoreR8)
        : HANDLER(StoreR9)
        : HANDLER(StoreR10)
        : HANDLER(StoreR11)
        : HANDLER(StoreR12)
        : HANDLER(StoreR13)
        : HANDLER(StoreR14)
        : HANDLER(StoreR15)
        : HANDLER(LoadR0)
        : HANDLER(LoadR1)
        : HANDLER(LoadR2)
        : HANDLER(LoadR3)
        : HANDLER(LoadR4)
        : HANDLER(LoadR5)
        : HANDLER(LoadR6)
        : HANDLER(LoadR7)
        : HANDLER(LoadR8)
        : HANDLER(LoadR9)
        : HANDLER(LoadR10)
        : HANDLER(LoadR11)
        : HANDLER(LoadR12)
        : HANDLER(LoadR13)
        : HANDLER(LoadR14)
        : HANDLER(LoadR15)
        : WIDE_HANDLER(Wide)
        : WIDE_HANDLER(ExtraWide)
        : WIDE_HANDLER(LoadRegister)
        : WIDE_HANDLER(LoadInt)
        : WIDE_HANDLER(LoadNull)
        : WIDE_HANDLER(LoadTrue)
        : WIDE_HANDLER(LoadFalse)
        : WIDE_HANDLER(LoadConstant)
        : WIDE_HANDLER(StoreRegister)
        : WIDE_HANDLER(Move)
        : WIDE_HANDLER(LoadGlobal)
        : WIDE_HANDLER(StoreGlobal)
        : WIDE_HANDLER(AddRegister)
        : WIDE_HANDLER(SubtractRegister)
        : WIDE_HANDLER(MultiplyRegister)
        : WIDE_HANDLER(DivideRegister)
        : WIDE_HANDLER(ConcatRegister)
        : WIDE_HANDLER(AddInt)
        : WIDE_HANDLER(SubtractInt)
        : WIDE_HANDLER(MultiplyInt)
        : WIDE_HANDLER(DivideInt)
        : WIDE_HANDLER(Negate)
        : WIDE_HANDLER(Call)
        : WIDE_HANDLER(Call0Argument)
        : WIDE_HANDLER(Call1Argument)
        : WIDE_HANDLER(Call2Argument)
        : WIDE_HANDLER(Less)
        : WIDE_HANDLER(ToString)
        : WIDE_HANDLER(Jump)
        : WIDE_HANDLER(JumpBack)
        : WIDE_HANDLER(JumpIfFalse)
        : WIDE_HANDLER(Return)
        : WIDE_HANDLER(Exit)
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
        : EXTRAWIDE_HANDLER(LoadInt)
        : EXTRAWIDE_HANDLER(LoadNull)
        : EXTRAWIDE_HANDLER(LoadTrue)
        : EXTRAWIDE_HANDLER(LoadFalse)
        : EXTRAWIDE_HANDLER(LoadConstant)
        : EXTRAWIDE_HANDLER(StoreRegister)
        : EXTRAWIDE_HANDLER(Move)
        : EXTRAWIDE_HANDLER(LoadGlobal)
        : EXTRAWIDE_HANDLER(StoreGlobal)
        : EXTRAWIDE_HANDLER(AddRegister)
        : EXTRAWIDE_HANDLER(SubtractRegister)
        : EXTRAWIDE_HANDLER(MultiplyRegister)
        : EXTRAWIDE_HANDLER(DivideRegister)
        : EXTRAWIDE_HANDLER(ConcatRegister)
        : EXTRAWIDE_HANDLER(AddInt)
        : EXTRAWIDE_HANDLER(SubtractInt)
        : EXTRAWIDE_HANDLER(MultiplyInt)
        : EXTRAWIDE_HANDLER(DivideInt)
        : EXTRAWIDE_HANDLER(Negate)
        : EXTRAWIDE_HANDLER(Call)
        : EXTRAWIDE_HANDLER(Call0Argument)
        : EXTRAWIDE_HANDLER(Call1Argument)
        : EXTRAWIDE_HANDLER(Call2Argument)
        : EXTRAWIDE_HANDLER(Less)
        : EXTRAWIDE_HANDLER(ToString)
        : EXTRAWIDE_HANDLER(Jump)
        : EXTRAWIDE_HANDLER(JumpBack)
        : EXTRAWIDE_HANDLER(JumpIfFalse)
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
}

void VM::add_global(StringSlice name) const {
  globals.push_back(Global{std::string(name.data, name.len), Value::empty()});
}

std::unique_ptr<VM> new_vm() { return std::make_unique<VM>(); }
FunctionInfoWriter VM::new_function_info() const {
  return FunctionInfoWriter(make_handle(new FunctionInfo), this);
}

template <typename O> O *VM::manage(O *t) const {
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

template <typename O> Handle<O> *VM::make_handle(O *object) const {
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

template <typename O> void VM::release(Handle<O> *handle) const {
  if (handle->previous != nullptr)
    handle->previous->next = handle->next;
  else
    handles = reinterpret_cast<Handle<Object> *>(handle->next);
  if (handle->next != nullptr)
    handle->next->previous = handle->previous;
  delete handle;
}

Symbol *VM::intern(StringSlice s) const {
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

void VM::release(Object *o) const {
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
