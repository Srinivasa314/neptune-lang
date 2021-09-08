handler(LoadInt, accumulator = static_cast<Value>(READ(itype)););

handler(LoadGlobal, {
  auto &g = globals[READ(utype)];
  if (g.value.is_empty()) {
    PANIC("Cannot access uninitialized variable " << g.name);
  } else {
    accumulator = g.value;
  }
});
handler(StoreGlobal, globals[READ(utype)].value = accumulator;);

#define BINARY_OP_INT(opname, intfn, op)                                       \
  do {                                                                         \
    if (accumulator.is_int()) {                                                \
      int result;                                                              \
      int i = READ(itype);                                                     \
      if (!intfn(accumulator.as_int(), i, result)) {                           \
        PANIC("Cannot " #opname " "                                            \
              << accumulator.as_int() << " and " << i                          \
              << " as the result does not fit in an int");                     \
      }                                                                        \
      accumulator = static_cast<Value>(result);                                \
    } else if (accumulator.is_float()) {                                       \
      accumulator = static_cast<Value>(accumulator.as_float() op READ(itype)); \
    } else {                                                                   \
      PANIC("Cannot " #opname " types " << accumulator.type_string()           \
                                        << " and int");                        \
    }                                                                          \
  } while (0)

handler(AddInt, BINARY_OP_REGISTER(add, SafeAdd, +););
handler(SubtractInt, BINARY_OP_REGISTER(subtract, SafeSubtract, -););
handler(MultiplyInt, BINARY_OP_REGISTER(multiply, SafeMultiply, *););
handler(DivideInt, BINARY_OP_REGISTER(divide, SafeDivide, /););

handler(Jump, TODO(););
handler(JumpBack, TODO(););
handler(JumpIfFalse, TODO(););
