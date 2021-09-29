handler(LoadGlobal, {
  auto g = READ(utype);
  if (unlikely(globals[g].is_empty())) {
    PANIC("Cannot access uninitialized variable " << global_names[g]);

  } else {
    accumulator = globals[g];
  }
});
handler(StoreGlobal, globals[READ(utype)] = accumulator;);

#define BINARY_OP_INT(opname, intfn, op)                                       \
  do {                                                                         \
    if (accumulator.is_int()) {                                                \
      int result;                                                              \
      int i = READ(itype);                                                     \
      if (unlikely(!intfn(accumulator.as_int(), i, result))) {                 \
        PANIC("Cannot " #opname " "                                            \
              << accumulator.as_int() << " and " << i                          \
              << " as the result does not fit in an int");                     \
      }                                                                        \
      accumulator = Value(result);                                             \
    } else if (accumulator.is_float()) {                                       \
      accumulator = Value(accumulator.as_float() op READ(itype));              \
    } else {                                                                   \
      PANIC("Cannot " #opname " types " << accumulator.type_string()           \
                                        << " and int");                        \
    }                                                                          \
  } while (0)

handler(AddInt, BINARY_OP_INT(add, SafeAdd, +););
handler(SubtractInt, BINARY_OP_INT(subtract, SafeSubtract, -););
handler(MultiplyInt, BINARY_OP_INT(multiply, SafeMultiply, *););
handler(DivideInt, BINARY_OP_INT(divide, SafeDivide, /););
handler(ModInt, {
  if (accumulator.is_int()) {
    int result;
    int i = READ(itype);
    if (unlikely(!SafeModulus(accumulator.as_int(), i, result))) {
      PANIC("Cannot mod " << accumulator.as_int() << " and " << i
                          << " as the result does not fit in an int");
    }
    accumulator = Value(result);
  } else if (accumulator.is_float()) {
    accumulator = Value(fmod(accumulator.as_float(), READ(itype)));
  } else {
    PANIC("Cannot mod types " << accumulator.type_string() << " and int");
  }
});

handler(JumpBack, {
  auto offset = READ(utype);
  ip -= (offset + 1 + sizeof(utype));
});

handler(ForLoop, {
  auto offset = READ(utype);
  auto iter = READ(utype);
  auto end = iter + 1;
  bp[iter].inc();
  if (bp[iter].as_int() < bp[end].as_int()) {
    ip -= (offset + 1 + 2 * sizeof(utype));
  }
});
