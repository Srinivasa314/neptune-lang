handler(LoadRegister, accumulator = bp[READ(utype)];);

handler(LoadConstant, accumulator = constants[READ(utype)];);
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
      if (unlikely(!intfn(bp[reg].as_int(), accumulator.as_int(), res)))       \
        PANIC("Cannot " #opname " "                                            \
              << bp[reg].as_int() << " and " << accumulator.as_int()           \
              << " as the result does not fit in an int");                     \
      accumulator = Value(res);                                                \
    } else if (accumulator.is_float() && bp[reg].is_float()) {                 \
      accumulator = Value(bp[reg].as_float() op accumulator.as_float());       \
    } else if (accumulator.is_int() && bp[reg].is_float()) {                   \
      accumulator = Value(bp[reg].as_float() op accumulator.as_int());         \
    } else if (accumulator.is_float() && bp[reg].is_int()) {                   \
      accumulator = Value(bp[reg].as_int() op accumulator.as_float());         \
    } else {                                                                   \
      PANIC("Cannot " #opname " types " << bp[reg].type_string() << " and "    \
                                        << accumulator.type_string());         \
    }                                                                          \
  } while (0)

#define COMPARE_OP_REGISTER(op)                                                \
  do {                                                                         \
    auto reg = READ(utype);                                                    \
    if (accumulator.is_int() && bp[reg].is_int()) {                            \
      accumulator = Value(bp[reg].as_int() op accumulator.as_int());           \
    } else if (accumulator.is_float() && bp[reg].is_float()) {                 \
      accumulator = Value(bp[reg].as_float() op accumulator.as_float());       \
    } else if (accumulator.is_int() && bp[reg].is_float()) {                   \
      accumulator = Value(bp[reg].as_float() op accumulator.as_int());         \
    } else if (accumulator.is_float() && bp[reg].is_int()) {                   \
      accumulator = Value(bp[reg].as_int() op accumulator.as_float());         \
    } else {                                                                   \
      PANIC("Cannot compare types " << bp[reg].type_string() << " and "        \
                                    << accumulator.type_string());             \
    }                                                                          \
  } while (0)

handler(AddRegister, BINARY_OP_REGISTER(add, SafeAdd, +););
handler(SubtractRegister, BINARY_OP_REGISTER(subtract, SafeSubtract, -););
handler(MultiplyRegister, BINARY_OP_REGISTER(multiply, SafeMultiply, *););
handler(DivideRegister, BINARY_OP_REGISTER(divide, SafeDivide, /););
handler(ModRegister, {
  auto reg = READ(utype);
  if (accumulator.is_int() && bp[reg].is_int()) {
    int res;
    if (unlikely(!SafeModulus(bp[reg].as_int(), accumulator.as_int(), res)))
      PANIC("Cannot mod " << bp[reg].as_int() << " and " << accumulator.as_int()
                          << " as the result does not fit in an int");
    accumulator = Value(res);
  } else if (accumulator.is_float() && bp[reg].is_float()) {
    accumulator = Value(fmod(bp[reg].as_float(), accumulator.as_float()));
  } else if (accumulator.is_int() && bp[reg].is_float()) {
    accumulator = Value(fmod(bp[reg].as_float(), accumulator.as_int()));
  } else if (accumulator.is_float() && bp[reg].is_int()) {
    accumulator = Value(fmod(bp[reg].as_int(), accumulator.as_float()));
  } else {
    PANIC("Cannot mod types " << bp[reg].type_string() << " and "
                              << accumulator.type_string());
  }
});
handler(ConcatRegister, {
  auto reg = READ(utype);
  if (likely(accumulator.is_object() && accumulator.as_object()->is<String>() &&
             bp[reg].is_object() && bp[reg].as_object()->is<String>())) {
    accumulator = Value(manage(bp[reg].as_object()->as<String>()->concat(
        accumulator.as_object()->as<String>())));
  } else {
    PANIC("Cannot concat types " << bp[reg].type_string() << " and "
                                 << accumulator.type_string());
  }
});
handler(Equal, accumulator = Value(bp[READ(utype)] == accumulator););
handler(NotEqual, accumulator = Value(!(bp[READ(utype)] == accumulator)););
handler(GreaterThan, COMPARE_OP_REGISTER(>););
handler(LesserThan, COMPARE_OP_REGISTER(<););
handler(GreaterThanOrEqual, COMPARE_OP_REGISTER(>=););
handler(LesserThanOrEqual, COMPARE_OP_REGISTER(<=););

#define CALLOP(n)                                                              \
  if (likely(accumulator.is_object())) {                                       \
    if (accumulator.as_object()->is<Function>()) {                             \
      auto f = accumulator.as_object()->as<Function>();                        \
      auto arity = f->function_info->arity;                                    \
      if (unlikely(arity != n))                                                \
        PANIC("Function " << f->function_info->name << " takes "               \
                          << static_cast<uint32_t>(arity) << " arguments but " \
                          << static_cast<uint32_t>(n) << " were given");       \
      if (num_frames == MAX_FRAMES)                                            \
        PANIC("Recursion depth exceeded");                                     \
      frames[num_frames - 1].ip = ip;                                          \
      bp += offset;                                                            \
      CALL(n);                                                                 \
    } else if (accumulator.as_object()->is<NativeFunction>()) {                \
      auto f = accumulator.as_object()->as<NativeFunction>();                  \
      auto arity = f->arity;                                                   \
      if (unlikely(arity != n))                                                \
        PANIC("Function " << f->name << " takes "                              \
                          << static_cast<uint32_t>(arity) << " arguments but " \
                          << static_cast<uint32_t>(n) << " were given");       \
      bp += offset;                                                            \
      if (bp + f->max_slots > stack.get() + STACK_SIZE)                        \
        PANIC("Stack overflow");                                               \
      stack_top = bp + f->max_slots;                                           \
      for (auto v = bp + arity; v < bp + f->max_slots; v++)                    \
        *v = Value::null();                                                    \
      last_native_function = f;                                                \
      auto ok = f->inner(FunctionContext{this, bp, f->max_slots}, f->data);    \
      accumulator = return_value;                                              \
      return_value = Value::null();                                            \
      if (!ok) {                                                               \
        if ((ip = panic(ip, accumulator)) != nullptr) {                        \
          bp = frames[num_frames - 1].bp;                                      \
          auto f = frames[num_frames - 1].f;                                   \
          constants = f->function_info->constants.data();                      \
          DISPATCH();                                                          \
        } else                                                                 \
          goto panic_end;                                                      \
      }                                                                        \
      last_native_function = nullptr;                                          \
      bp -= offset;                                                            \
      stack_top = bp + frames[num_frames - 1].f->function_info->max_registers; \
    } else {                                                                   \
      PANIC(accumulator.type_string() << " is not callable");                  \
    }                                                                          \
  } else {                                                                     \
    PANIC(accumulator.type_string() << " is not callable");                    \
  }

#define CALLNARGUMENT(n)                                                       \
  do {                                                                         \
    auto offset = READ(utype);                                                 \
    CALLOP(n)                                                                  \
  } while (0)

handler(Call0Argument, CALLNARGUMENT(0););
handler(Call1Argument, CALLNARGUMENT(1););
handler(Call2Argument, CALLNARGUMENT(2););
handler(Call3Argument, CALLNARGUMENT(3););
handler(Call, {
  auto offset = READ(utype);
  auto n = READ(uint8_t);
  CALLOP(n)
});

handler(NewArray, {
  auto len = READ(utype);
  auto reg = READ(utype);

  bp[reg] = Value(manage(new Array(len)));
});

handler(LoadSubscript, {
  auto obj = bp[READ(utype)];
  if (likely(obj.is_object())) {
    if (obj.as_object()->is<Array>()) {
      if (likely(accumulator.is_int())) {
        auto i = accumulator.as_int();
        auto a = obj.as_object()->as<Array>();
        if (unlikely(i < 0 || static_cast<size_t>(i) >= a->inner.size()))
          PANIC("Array index out of range");
        else
          accumulator = a->inner[static_cast<size_t>(i)];
      } else {
        PANIC("Array indices must be int not " << accumulator.type_string());
      }
    } else if (obj.as_object()->is<Map>()) {
      auto &m = obj.as_object()->as<Map>()->inner;
      auto to_find = accumulator; // hack so that makes accumulator a register
      auto it = m.find(to_find);
      if (likely(it != m.end()))
        accumulator = it->second;
      else
        PANIC("Key " << accumulator << " does not exist in map");
    } else {
      PANIC("Cannot index type " << obj.type_string());
    }
  } else {
    PANIC("Cannot index type " << obj.type_string());
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
  if (likely(obj.is_object())) {
    if (obj.as_object()->is<Array>()) {
      if (likely(subscript.is_int())) {
        auto i = subscript.as_int();
        auto &a = obj.as_object()->as<Array>()->inner;
        if (unlikely(i < 0 || static_cast<size_t>(i) >= a.size()))
          PANIC("Array index out of range");
        else
          a[static_cast<size_t>(i)] = accumulator;
      } else {
        PANIC("Array indices must be int not" << subscript.type_string());
      }
    } else if (obj.as_object()->is<Map>()) {
      auto m = obj.as_object()->as<Map>();
      m->inner[subscript] = accumulator;
    } else {
      PANIC("Cannot index type " << obj.type_string());
    }
  } else {
    PANIC("Cannot index type " << obj.type_string());
  }
});

handler(NewMap, {
  auto len = READ(utype);
  auto reg = READ(utype);
  bp[reg] = Value(manage(new Map(len)));
});

handler(StrictEqual, accumulator = Value(ValueStrictEquality{}(bp[READ(utype)],
                                                               accumulator)););
handler(StrictNotEqual,
        accumulator = Value(!ValueStrictEquality{}(bp[READ(utype)],
                                                   accumulator)););

handler(Jump, {
  auto offset = READ(utype);
  ip += (offset - (1 + sizeof(utype) + header_size<utype>()));
});

handler(JumpIfFalseOrNull, {
  auto offset = READ(utype);
  // This is because clang thinks that this is extremely unlikely
  if (IF_CLANG(likely)(accumulator.is_null_or_false())) {
    ip += (offset - (1 + sizeof(utype) + header_size<utype>()));
  }
});

handler(JumpIfNotFalseOrNull, {
  auto offset = READ(utype);
  // This is because clang thinks that this is extremely unlikely
  if (IF_CLANG(likely)(!accumulator.is_null_or_false())) {
    ip += (offset - (1 + sizeof(utype) + header_size<utype>()));
  }
});

handler(JumpConstant, {
  auto offset = static_cast<uint32_t>(constants[READ(utype)].as_int());
  ip += (offset - (1 + sizeof(utype) + header_size<utype>()));
});

handler(JumpIfFalseOrNullConstant, {
  // This is because clang thinks that this is extremely unlikely
  auto offset = static_cast<uint32_t>(constants[READ(utype)].as_int());
  if (IF_CLANG(likely)(accumulator.is_null_or_false())) {
    ip += (offset - (1 + sizeof(utype) + header_size<utype>()));
  }
});

handler(JumpIfNotFalseOrNullConstant, {
  // This is because clang thinks that this is extremely unlikely
  auto offset = static_cast<uint32_t>(constants[READ(utype)].as_int());
  if (IF_CLANG(likely)(!accumulator.is_null_or_false())) {
    ip += (offset - (1 + sizeof(utype) + header_size<utype>()));
  }
});

handler(BeginForLoop, {
  auto offset = READ(utype);
  auto iter = READ(utype);
  uint16_t end = iter + 1;
  if (likely(bp[iter].is_int() && bp[end].is_int())) {
    if (bp[iter].as_int() >= bp[end].as_int()) {
      ip += (offset - (1 + 2 * sizeof(utype) + header_size<utype>()));
    }
  } else {
    PANIC("Expected int and int for the start and end of for loop got "
          << bp[iter].type_string() << " and " << bp[end].type_string()
          << " instead");
  }
});
handler(BeginForLoopConstant, {
  auto offset = static_cast<uint32_t>(constants[READ(utype)].as_int());
  auto iter = READ(utype);
  uint16_t end = iter + 1;
  if (likely(bp[iter].is_int() && bp[end].is_int())) {
    if (bp[iter].as_int() >= bp[end].as_int()) {
      ip += (offset - (1 + 2 * sizeof(utype) + header_size<utype>()));
    }
  } else {
    PANIC("Expected int and int for the start and end of for loop got "
          << bp[iter].type_string() << " and " << bp[end].type_string()
          << " instead");
  }
});

handler(MakeFunction, {
  auto constant = constants[READ(utype)];
  accumulator = make_function(bp, constant);
});

handler(
    LoadUpvalue,
    accumulator = *frames[num_frames - 1].f->upvalues[READ(utype)]->location;);
handler(
    StoreUpvalue,
    *frames[num_frames - 1].f->upvalues[READ(utype)]->location = accumulator;);
handler(Close, CLOSE(READ(utype)););

handler(LoadProperty, {
  auto property = constants[READ(utype)].as_object()->as<Symbol>();
  if (accumulator.is_object() && accumulator.as_object()->is<Module>()) {
    auto module = accumulator.as_object()->as<Module>();
    auto iter = module->module_variables.find(property);
    if (iter == module->module_variables.end())
      PANIC("Module " << module->name << " does not have variable "
                      << static_cast<StringSlice>(*property));
    else
      accumulator = module_variables[iter->second.position];
  } else {
    PANIC("Cannot get property from type " << accumulator.type_string());
  }
});
