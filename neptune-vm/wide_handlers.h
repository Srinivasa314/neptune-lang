handler(LoadRegister, accumulator = bp[READ(utype)];);

handler(LoadConstant, accumulator = f->constants[READ(utype)];);
handler(StoreRegister, bp[READ(utype)] = accumulator;);
handler(Move, {
  auto src = READ(utype);
  auto dest = READ(utype);
  bp[dest] = bp[src];
});

#define BINARY_OP_REGISTER(opname, intfn, op)                                  \
  do {                                                                         \
    auto reg = READ(utype);                                                    \
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
  } while (0)
handler(AddRegister, BINARY_OP_REGISTER(add, SafeAdd, +););
handler(SubtractRegister, BINARY_OP_REGISTER(subtract, SafeSubtract, -););
handler(MultiplyRegister, BINARY_OP_REGISTER(multiply, SafeMultiply, *););
handler(DivideRegister, BINARY_OP_REGISTER(divide, SafeDivide, /););
handler(ConcatRegister, {
  auto reg = READ(utype);
  if (accumulator.is_object() && accumulator.as_object()->is<String>() &&
      bp[reg].is_object() && bp[reg].as_object()->is<String>()) {
    accumulator =
        static_cast<Value>(manage(bp[reg].as_object()->as<String>()->concat(
            accumulator.as_object()->as<String>())));
  } else {
    PANIC("Cannot concat types" << bp[reg].type_string() << " and "
                                << accumulator.type_string());
  }
});
handler(Call, TODO(););
handler(Call0Argument, TODO(););
handler(Call1Argument, TODO(););
handler(Call2Argument, TODO(););

handler(NewArray, {
  auto len = READ(utype);
  auto reg = READ(utype);

  bp[reg] = static_cast<Value>(manage(new Array(len)));
});

handler(LoadSubscript, {
  auto obj = bp[READ(utype)];
  if (obj.is_object() && obj.as_object()->is<Array>()) {
    if (accumulator.is_int()) {
      auto i = accumulator.as_int();
      auto a = obj.as_object()->as<Array>();
      if (i < 0 || static_cast<size_t>(i) >= a->inner.size())
        PANIC("Array index out of range");
      else
        accumulator = a->inner[static_cast<size_t>(i)];
    } else {
      PANIC("Array indices must be int not" << accumulator.type_string());
    }
  } else {
    PANIC("Cannot index type" << obj.type_string());
  }
});

handler(StoreArrayUnchecked, {
  auto &array = bp[READ(utype)].as_object()->as<Array>()->inner;
  auto index = READ(utype);
  array[index] = accumulator;
});

handler(StoreSubscript, {
  auto obj = bp[READ(utype)];
  auto subscript = bp[READ(utype)];
  if (obj.is_object() && obj.as_object()->is<Array>()) {

    if (subscript.is_int()) {
      auto i = subscript.as_int();
      auto &a = obj.as_object()->as<Array>()->inner;
      if (i < 0 || static_cast<size_t>(i) >= a.size())
        PANIC("Array index out of range");
      else
        a[static_cast<size_t>(i)] = accumulator;
    } else {
      PANIC("Array indices must be int not" << subscript.type_string());
    }
  } else {
    PANIC("Cannot index type" << obj.type_string());
  }
  DISPATCH();
});
