handler(LoadModuleVariable, accumulator = module_variables[READ(utype)];);
handler(StoreModuleVariable, module_variables[READ(utype)] = accumulator;);

#define BINARY_OP_INT(opname, intfn, op)                                       \
  do {                                                                         \
    if (accumulator.is_int()) {                                                \
      int result;                                                              \
      int i = READ(itype);                                                     \
      if (unlikely(!intfn(accumulator.as_int(), i, result))) {                 \
        PANIC("Cannot " #opname " "                                            \
              << accumulator.as_int() << " and " << i                          \
              << " as the result does not fit in an Int");                     \
      }                                                                        \
      accumulator = Value(result);                                             \
    } else if (accumulator.is_float()) {                                       \
      accumulator = Value(accumulator.as_float() op READ(itype));              \
    } else {                                                                   \
      PANIC("Cannot " #opname " types " << accumulator.type_string()           \
                                        << " and Int");                        \
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
                          << " as the result does not fit in an Int");
    }
    accumulator = Value(result);
  } else if (accumulator.is_float()) {
    accumulator = Value(fmod(accumulator.as_float(), READ(itype)));
  } else {
    PANIC("Cannot mod types " << accumulator.type_string() << " and Int");
  }
});

handler(JumpBack, {
  auto offset = READ(utype);
  ip -= (offset + 1 + sizeof(utype) + header_size<utype>());
});

handler(ForLoop, {
  auto offset = READ(utype);
  auto iter = READ(utype);
  uint16_t end = static_cast<uint16_t>(iter + 1);
  bp[iter].inc();
  if (bp[iter].as_int() < bp[end].as_int()) {
    ip -= (offset + 1 + 2 * sizeof(utype) + header_size<utype>());
  }
});
