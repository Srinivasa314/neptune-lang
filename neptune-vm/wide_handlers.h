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
              << " as the result does not fit in an Int");                     \
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
                          << " as the result does not fit in an Int");
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

#ifndef CALLOP
#define CALLOP
uint32_t callop_n, callop_nargs, callop_offset;
callop : {
  if (likely(accumulator.is_object())) {
    if (accumulator.as_object()->is<Function>()) {
      auto f = accumulator.as_object()->as<Function>();
      auto arity = f->function_info->arity;
      if (unlikely(arity != callop_nargs))
        PANIC("Function " << f->function_info->name << " takes "
                          << static_cast<uint32_t>(arity) << " arguments but "
                          << static_cast<uint32_t>(callop_nargs)
                          << " were given");
      task->frames.back().ip = ip;
      bp += callop_offset;
      CALL(callop_n);
    } else if (accumulator.as_object()->is<NativeFunction>()) {
      auto f = accumulator.as_object()->as<NativeFunction>();
      auto arity = f->arity;
      if (unlikely(arity != callop_nargs))
        PANIC("Function " << f->name << " takes "
                          << static_cast<uint32_t>(arity) << " arguments but "
                          << static_cast<uint32_t>(callop_nargs)
                          << " were given");
      last_native_function = f;
      auto old_stack_top = task->stack_top;
      auto ok = f->inner(this, bp + callop_offset);
      accumulator = return_value;
      return_value = Value::null();
      bp = bp + (task->stack_top - old_stack_top);
      if (!ok) {
        if ((ip = panic(ip, accumulator)) != nullptr) {
          bp = task->frames.back().bp;
          auto f = task->frames.back().f;
          constants = f->function_info->constants.data();
          DISPATCH();
        } else
          goto panic_end;
      }
      last_native_function = nullptr;
    } else {
      PANIC(accumulator.type_string() << " is not callable");
    }
  } else {
    PANIC(accumulator.type_string() << " is not callable");
  }
  DISPATCH();
}
#endif

handler(Call, {
  callop_offset = READ(utype);
  auto n = READ(uint8_t);
  callop_n = n;
  callop_nargs = n;
  goto callop;
});

handler(CallMethod, {
  auto object = bp[READ(utype)];
  auto member = constants[READ(utype)].as_object()->as<Symbol>();
  callop_offset = READ(utype);
  auto n = READ(uint8_t);

  auto class_ = get_class(object);
  auto method = class_->find_method(member);
  if (method != nullptr) {
    accumulator = Value(method);
    bp[callop_offset] = object;
    callop_n = n + 1;
    callop_nargs = n;
    goto callop;

  } else if (object.is_object() && object.as_object()->is<Module>()) {
    auto module = object.as_object()->as<Module>();
    auto iter = module->module_variables.find(member);
    if (iter == module->module_variables.end() || !iter->second.exported)
      PANIC("Module " << module->name << " does not export any variable named "
                      << static_cast<StringSlice>(*member));
    else
      accumulator = module_variables[iter->second.position];
    callop_offset++;
    callop_n = n;
    callop_nargs = n;
    goto callop;

  } else if (object.is_object() && object.as_object()->is<Instance>()) {
    auto instance = object.as_object()->as<Instance>();
    if (instance->properties.find(member) == instance->properties.end())
      PANIC("Object does not have any property named "
            << static_cast<StringSlice>(*member));
    else
      accumulator = instance->properties[member];
    callop_offset++;
    callop_n = n;
    callop_nargs = n;
    goto callop;

  } else {
    PANIC("Object does not have method named "
          << static_cast<StringSlice>(*member));
  }
});

handler(SuperCall, {
  auto object = bp[0];
  auto member = constants[READ(utype)].as_object()->as<Symbol>();
  callop_offset = READ(utype);
  auto n = READ(uint8_t);

  auto class_ = task->frames.back().f->super_class;
  auto method = class_->find_method(member);
  if (method != nullptr) {
    accumulator = Value(method);
    bp[callop_offset] = object;
    callop_n = n + 1;
    callop_nargs = n;
    goto callop;

  } else {
    PANIC("Object does not have method named "
          << static_cast<StringSlice>(*member));
  }
});

handler(Construct, {
  auto offset = READ(utype);
  auto n = READ(uint8_t);
  if (likely(accumulator.is_object() && accumulator.as_object()->is<Class>())) {
    auto construct_sym = builtin_symbols.construct;
    auto class_ = accumulator.as_object()->as<Class>();
    temp_roots.push_back(Value(class_));
    Value obj;
    if (class_->is_native) {
      if (class_->construct == nullptr)
        PANIC("Type " << class_->name << " cannot be constructed");
      else
        obj = class_->construct(this);
    } else {
      auto instance = manage(new Instance());
      instance->class_ = class_;
      obj = Value(instance);
    }
    temp_roots.pop_back();
    if (class_->methods.find(construct_sym) != class_->methods.end()) {
      auto f = class_->methods[construct_sym]->as<Function>();
      auto arity = f->function_info->arity;
      if (unlikely(arity != n))
        PANIC("Function " << f->function_info->name << " takes "
                          << static_cast<uint32_t>(arity) << " arguments but "
                          << static_cast<uint32_t>(n) << " were given");
      task->frames.back().ip = ip;
      bp += offset;
      *bp = obj;
      CALL(n + 1);
    } else {
      if (n != 0)
        PANIC("Function construct takes 0 arguments but "
              << static_cast<uint32_t>(n) << " were given");
      accumulator = Value(obj);
    }
  } else {
    PANIC("new can be called only on classes not "
          << accumulator.type_string());
  }
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
        PANIC("Array indices must be Int not " << accumulator.type_string());
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
        PANIC("Array indices must be Int not" << subscript.type_string());
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

handler(NewObject, {
  auto len = READ(utype);
  auto reg = READ(utype);
  bp[reg] = Value(manage(new Instance(len)));
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
    PANIC("Expected Int and Int for the start and end of for loop got "
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
    PANIC("Expected Int and Int for the start and end of for loop got "
          << bp[iter].type_string() << " and " << bp[end].type_string()
          << " instead");
  }
});

handler(MakeFunction, {
  auto function = constants[READ(utype)].as_object()->as<FunctionInfo>();
  accumulator = Value(make_function(bp, function));
});

handler(MakeClass, {
  auto class_ =
      manage(new Class(*constants[READ(utype)].as_object()->as<Class>()));
  temp_roots.push_back(Value(class_));
  if (accumulator.is_object() && accumulator.as_object()->is<Class>())
    class_->super = accumulator.as_object()->as<Class>();
  else
    PANIC("Expected to inherit from Class got " << accumulator.type_string());
  for (auto p : class_->methods)
    if (p.second->is<FunctionInfo>()) {
      class_->methods[p.first] =
          make_function(bp, p.second->as<FunctionInfo>());
      class_->methods[p.first]->as<Function>()->super_class =
          accumulator.as_object()->as<Class>();
    }
  temp_roots.pop_back();
  accumulator = Value(class_);
});

handler(LoadUpvalue,
        accumulator = *task->frames.back().f->upvalues[READ(utype)]->location;);
handler(StoreUpvalue,
        *task->frames.back().f->upvalues[READ(utype)]->location = accumulator;);
handler(Close, CLOSE(READ(utype)););

handler(LoadProperty, {
  auto object = bp[READ(utype)];
  auto property = constants[READ(utype)].as_object()->as<Symbol>();
  if (object.is_object() && object.as_object()->is<Module>()) {
    auto module = object.as_object()->as<Module>();
    auto iter = module->module_variables.find(property);
    if (iter == module->module_variables.end() || !iter->second.exported)
      PANIC("Module " << module->name << " does not export any variable named "
                      << static_cast<StringSlice>(*property));
    else
      accumulator = module_variables[iter->second.position];
  } else if (object.is_object() && object.as_object()->is<Instance>()) {
    auto instance = object.as_object()->as<Instance>();
    if (instance->properties.find(property) == instance->properties.end())
      PANIC("Object does not have any property named "
            << static_cast<StringSlice>(*property));
    else
      accumulator = instance->properties[property];
  } else {
    PANIC("Cannot get property from type " << object.type_string());
  }
});

handler(StoreProperty, {
  auto object = bp[READ(utype)];
  auto property = constants[READ(utype)].as_object()->as<Symbol>();
  if (object.is_object() && object.as_object()->is<Instance>()) {
    auto instance = object.as_object()->as<Instance>();
    instance->properties[property] = accumulator;
  } else {
    PANIC("Cannot set property for type " << object.type_string());
  }
});