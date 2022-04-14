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
        THROW("OverflowError",                                                 \
              "Cannot " #opname " "                                            \
                  << bp[reg].as_int() << " and " << accumulator.as_int()       \
                  << " as the result does not fit in an Int");                 \
      accumulator = Value(res);                                                \
    } else if (accumulator.is_float() && bp[reg].is_float()) {                 \
      accumulator = Value(bp[reg].as_float() op accumulator.as_float());       \
    } else if (accumulator.is_int() && bp[reg].is_float()) {                   \
      accumulator = Value(bp[reg].as_float() op accumulator.as_int());         \
    } else if (accumulator.is_float() && bp[reg].is_int()) {                   \
      accumulator = Value(bp[reg].as_int() op accumulator.as_float());         \
    } else {                                                                   \
      THROW("TypeError", "Cannot " #opname " types "                           \
                             << bp[reg].type_string() << " and "               \
                             << accumulator.type_string());                    \
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
      THROW("TypeError", "Cannot compare types "                               \
                             << bp[reg].type_string() << " and "               \
                             << accumulator.type_string());                    \
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
      THROW("OverflowError",
            "Cannot mod " << bp[reg].as_int() << " and " << accumulator.as_int()
                          << " as the result does not fit in an Int");
    accumulator = Value(res);
  } else if (accumulator.is_float() && bp[reg].is_float()) {
    accumulator = Value(fmod(bp[reg].as_float(), accumulator.as_float()));
  } else if (accumulator.is_int() && bp[reg].is_float()) {
    accumulator = Value(fmod(bp[reg].as_float(), accumulator.as_int()));
  } else if (accumulator.is_float() && bp[reg].is_int()) {
    accumulator = Value(fmod(bp[reg].as_int(), accumulator.as_float()));
  } else {
    THROW("TypeError", "Cannot mod types " << bp[reg].type_string() << " and "
                                           << accumulator.type_string());
  }
});
handler(ConcatRegister, {
  auto reg = READ(utype);
  if (likely(accumulator.is_ptr() && accumulator.as_ptr()->is<String>() &&
             bp[reg].is_ptr() && bp[reg].as_ptr()->is<String>())) {
    accumulator = Value(concat(bp[reg].as_ptr()->as<String>(),
                               accumulator.as_ptr()->as<String>()));
  } else {
    THROW("TypeError", "Cannot concat types " << bp[reg].type_string()
                                              << " and "
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
uint8_t callop_actual_nargs, callop_nargs;
uint32_t callop_offset;
callop : {
  if (likely(accumulator.is_ptr())) {
    if (accumulator.as_ptr()->is<Function>()) {
      auto f = accumulator.as_ptr()->as<Function>();
      auto arity = f->function_info->arity;
      if (unlikely(arity != callop_nargs))
        THROW("ArgumentError",
              "Function " << f->function_info->name << " takes "
                          << static_cast<uint32_t>(arity) << " arguments but "
                          << static_cast<uint32_t>(callop_nargs)
                          << " were given");
      task->frames.back().ip = ip;
      bp += callop_offset;
      constants = f->function_info->constants.data();
      if (size_t(bp - task->stack.get()) + f->function_info->max_registers >
          task->stack_size)
        bp = task->grow_stack(bp, f->function_info->max_registers);
      task->stack_top = bp + f->function_info->max_registers;
      ip = f->function_info->bytecode.data();
      for (size_t i = callop_actual_nargs; i < f->function_info->max_registers;
           i++)
        bp[i] = Value(nullptr);
      task->frames.push_back(Frame{bp, f, ip});
    } else if (accumulator.as_ptr()->is<NativeFunction>()) {
      auto f = accumulator.as_ptr()->as<NativeFunction>();
      auto arity = f->arity;
      if (unlikely(arity != callop_nargs))
        THROW("ArgumentError",
              "Function " << f->name << " takes "
                          << static_cast<uint32_t>(arity) << " arguments but "
                          << static_cast<uint32_t>(callop_nargs)
                          << " were given");
      last_native_function = f;
      auto old_stack_top = task->stack_top;
      auto status = f->inner(this, bp + callop_offset);
      accumulator = return_value;
      return_value = Value::null();
      bp = bp + (task->stack_top - old_stack_top);
      if (status == VMStatus::Success) {
        last_native_function = nullptr;
      } else if (status == VMStatus::Error) {
        task->frames.back().ip = ip;
        if ((ip = throw_(accumulator)) != nullptr) {
          bp = task->frames.back().bp;
          auto f = task->frames.back().f;
          constants = f->function_info->constants.data();
          DISPATCH();
        } else {
          goto throw_end;
        }
      } else if (status == VMStatus::Suspend) {
        task->frames.back().ip = ip;
        last_native_function = nullptr;
        current_task = nullptr;
        return;
      } else {
        unreachable();
      }
    } else {
      THROW("TypeError",
            "Type " << accumulator.type_string() << " is not callable");
    }
  } else {
    THROW("TypeError",
          "Type " << accumulator.type_string() << " is not callable");
  }
  DISPATCH();
}
#endif

handler(Call, {
  callop_offset = READ(utype);
  auto n = READ(uint8_t);
  callop_actual_nargs = n;
  callop_nargs = n;
  goto callop;
});

handler(CallMethod, {
  auto object = bp[READ(utype)];
  auto member = constants[READ(utype)].as_ptr()->as<Symbol>();
  callop_offset = READ(utype);
  auto n = READ(uint8_t);

  auto class_ = get_class(object);
  auto method = class_->find_method(member);
  if (likely(method != nullptr)) {
    accumulator = Value(method);
    bp[callop_offset] = object;
    callop_actual_nargs = n + 1;
    callop_nargs = n;
    goto callop;

  } else if (object.is_ptr() && object.as_ptr()->is<Module>()) {
    auto module = object.as_ptr()->as<Module>();
    auto iter = module->module_variables.find(member);
    if (unlikely(iter == module->module_variables.end() ||
                 !iter->second.exported))
      THROW("NoModuleVariableError",
            "Module " << module->name << " does not export any variable named "
                      << static_cast<StringSlice>(*member));
    else
      accumulator = module_variables[iter->second.position];
    callop_offset++;
    callop_actual_nargs = n;
    callop_nargs = n;
    goto callop;

  } else if (object.is_ptr() && object.as_ptr()->is<Instance>()) {
    auto instance = object.as_ptr()->as<Instance>();
    auto iter = instance->properties.find(member);
    if (iter == instance->properties.end())
      THROW("NoMethodError", "object does not have any method named "
                                 << static_cast<StringSlice>(*member));
    else
      accumulator = iter->second;
    callop_offset++;
    callop_actual_nargs = n;
    callop_nargs = n;
    goto callop;

  } else {
    THROW("NoMethodError", class_->name << " does not have method named "
                                        << static_cast<StringSlice>(*member));
  }
});

handler(SuperCall, {
  auto object = bp[0];
  auto member = constants[READ(utype)].as_ptr()->as<Symbol>();
  callop_offset = READ(utype);
  auto n = READ(uint8_t);

  auto class_ = task->frames.back().f->super_class;
  auto method = class_->find_method(member);
  if (likely(method != nullptr)) {
    accumulator = Value(method);
    bp[callop_offset] = object;
    callop_actual_nargs = n + 1;
    callop_nargs = n;
    goto callop;

  } else {
    THROW("NoMethodError", class_->name << " does not have method named "
                                        << static_cast<StringSlice>(*member));
  }
});

handler(Construct, {
  callop_offset = READ(utype);
  auto n = READ(uint8_t);
  if (likely(accumulator.is_ptr() && accumulator.as_ptr()->is<Class>())) {
    auto construct_sym = builtin_symbols.construct;
    auto class_ = accumulator.as_ptr()->as<Class>();
    temp_roots.push_back(Value(class_));
    Value obj;
    if (class_->is_native) {
      obj = Value::null();
    } else {
      auto instance = allocate<Instance>();
      instance->class_ = class_;
      obj = Value(instance);
    }
    temp_roots.pop_back();
    auto iter = class_->methods.find(construct_sym);
    if (likely(iter != class_->methods.end())) {
      accumulator = Value(iter->second);
      bp[callop_offset] = obj;
      callop_actual_nargs = n + 1;
      callop_nargs = n;
      goto callop;
    } else {
      THROW("NoMethodError",
            "Class " << class_->name << " does not have a constructor");
    }
  } else {
    THROW("TypeError", "new can be called only on classes not "
                           << accumulator.type_string());
  }
});

handler(NewArray, {
  auto len = READ(utype);
  auto reg = READ(utype);

  bp[reg] = Value(allocate<Array>(len));
});

handler(LoadSubscript, {
  auto obj = bp[READ(utype)];
  if (likely(obj.is_ptr())) {
    if (obj.as_ptr()->is<Array>()) {
      if (likely(accumulator.is_int())) {
        auto i = accumulator.as_int();
        auto a = obj.as_ptr()->as<Array>();
        if (unlikely(i < 0 || static_cast<size_t>(i) >= a->inner.size()))
          THROW("IndexError", "Array index out of range");
        else
          accumulator = a->inner[static_cast<size_t>(i)];
      } else if (accumulator.is_ptr() &&
                 accumulator.as_ptr()->is<Range>()) {
        auto &r = *accumulator.as_ptr()->as<Range>();
        auto start = r.start;
        auto end = r.end;
        auto a = obj.as_ptr()->as<Array>();
        if (start < 0 || static_cast<size_t>(start) >= a->inner.size() ||
            end < 0 || static_cast<size_t>(end) > a->inner.size()) {
          THROW("IndexError", "Array index out of range");
        }
        if (start > end) {
          auto new_arr = allocate<Array>(0U);
          accumulator = Value(new_arr);
        } else {
          auto new_arr = allocate<Array>(static_cast<uint32_t>(end - start));
          for (int32_t i = start; i < end; i++) {
            new_arr->inner[static_cast<uint32_t>(i - start)] =
                a->inner[static_cast<uint32_t>(i)];
          }
          accumulator = Value(new_arr);
        }
      } else {
        THROW("TypeError", "Array indices must be Int or Range not "
                               << accumulator.type_string());
      }
    } else if (obj.as_ptr()->is<Map>()) {
      auto &m = obj.as_ptr()->as<Map>()->inner;
      auto it = m.find(accumulator);
      if (likely(it != m.end()))
        accumulator = it->second;
      else
        THROW("KeyError", "Key " << accumulator << " does not exist in map");
    } else if (obj.as_ptr()->is<String>()) {
      if (likely(accumulator.is_ptr() &&
                 accumulator.as_ptr()->is<Range>())) {
        auto str = obj.as_ptr()->as<String>();
        auto &r = *accumulator.as_ptr()->as<Range>();
        if (r.start < 0 || static_cast<size_t>(r.start) >= str->len ||
            r.end < 0 || static_cast<size_t>(r.end) > str->len) {
          THROW("IndexError", "String index out of range");
        }
        if (r.start > r.end) {
          auto new_str = allocate<String>("");
          accumulator = Value(new_str);
        } else {
          if (int8_t(str->data[r.start]) >= -0x40 &&
              (size_t(r.end) == str->len ||
               int8_t(str->data[r.end]) >= -0x40)) {
            auto bytes = StringSlice(str->data + r.start,
                                     static_cast<uint32_t>(r.end - r.start));
            auto new_str = allocate<String>(bytes);
            accumulator = Value(new_str);
          } else
            THROW("IndexError", "Index is not a character boundary");
        }
      } else {
        THROW("TypeError",
              "String indices must be Range not " << accumulator.type_string());
      }
    } else if (obj.as_ptr()->is<Instance>()) {
      if (accumulator.is_ptr() && accumulator.as_ptr()->is<Symbol>()) {
        auto &props = obj.as_ptr()->as<Instance>()->properties;
        auto it = props.find(accumulator.as_ptr()->as<Symbol>());
        if (likely(it != props.end()))
          accumulator = it->second;
        else
          THROW("PropertyError",
                "Property " << accumulator << " does not exist in object");
      } else {
        THROW("TypeError", obj.type_string() << " indices must be Symbol not "
                                             << accumulator.type_string());
      }
    } else {
      THROW("TypeError", "Cannot index type " << obj.type_string());
    }
  } else {
    THROW("TypeError", "Cannot index type " << obj.type_string());
  }
});

handler(StoreArrayUnchecked, {
  auto &array = bp[READ(utype)].as_ptr()->as<Array>()->inner;
  auto index = READ(utype);
  array[index] = accumulator;
});

handler(StoreSubscript, {
  auto obj = bp[READ(utype)];
  auto subscript = bp[READ(utype)];
  if (likely(obj.is_ptr())) {
    if (obj.as_ptr()->is<Array>()) {
      if (likely(subscript.is_int())) {
        auto i = subscript.as_int();
        auto &a = obj.as_ptr()->as<Array>()->inner;
        if (unlikely(i < 0 || static_cast<size_t>(i) >= a.size()))
          THROW("IndexError", "Array index out of range");
        else
          a[static_cast<size_t>(i)] = accumulator;
      } else {
        THROW("TypeError",
              "Array indices must be Int not" << subscript.type_string());
      }
    } else if (obj.as_ptr()->is<Map>()) {
      auto m = obj.as_ptr()->as<Map>();
      m->inner.insert({subscript, accumulator});
    } else if (obj.as_ptr()->is<Instance>()) {
      if (subscript.is_ptr() && subscript.as_ptr()->is<Symbol>()) {
        obj.as_ptr()->as<Instance>()->properties.insert(
            {subscript.as_ptr()->as<Symbol>(), accumulator});
      } else {
        THROW("TypeError", obj.type_string() << " indices must be Symbol not "
                                             << subscript.type_string());
      }
    } else {
      THROW("TypeError", "Cannot index type " << obj.type_string());
    }
  } else {
    THROW("TypeError", "Cannot index type " << obj.type_string());
  }
});

handler(NewMap, {
  auto len = READ(utype);
  auto reg = READ(utype);
  bp[reg] = Value(allocate<Map>(len));
});

handler(NewObject, {
  auto len = READ(utype);
  auto reg = READ(utype);
  auto obj = allocate<Instance>(len);
  obj->class_ = builtin_classes.Object;
  bp[reg] = Value(obj);
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
  uint32_t end = iter + 1;
  if (likely(bp[iter].is_int() && bp[end].is_int())) {
    if (bp[iter].as_int() >= bp[end].as_int()) {
      ip += (offset - (1 + 2 * sizeof(utype) + header_size<utype>()));
    }
  } else {
    THROW("TypeError",
          "Expected Int and Int for the start and end of the range got "
              << bp[iter].type_string() << " and " << bp[end].type_string()
              << " instead");
  }
});
handler(BeginForLoopConstant, {
  auto offset = static_cast<uint32_t>(constants[READ(utype)].as_int());
  auto iter = READ(utype);
  uint32_t end = iter + 1;
  if (likely(bp[iter].is_int() && bp[end].is_int())) {
    if (bp[iter].as_int() >= bp[end].as_int()) {
      ip += (offset - (1 + 2 * sizeof(utype) + header_size<utype>()));
    }
  } else {
    THROW("TypeError",
          "Expected Int and Int for the start and end of the range got "
              << bp[iter].type_string() << " and " << bp[end].type_string()
              << " instead");
  }
});

handler(MakeFunction, {
  auto function = constants[READ(utype)].as_ptr()->as<FunctionInfo>();
  accumulator = Value(make_function(bp, function));
});

handler(MakeClass, {
  auto class_ =
      allocate<Class>(*constants[READ(utype)].as_ptr()->as<Class>());
  temp_roots.push_back(Value(class_));
  if (accumulator.is_ptr() && accumulator.as_ptr()->is<Class>()) {
    auto parent = accumulator.as_ptr()->as<Class>();
    if (parent != builtin_classes.Object && parent->is_native)
      THROW("TypeError", "Cannot inherit from native class " << parent->name);
    class_->super = parent;
  } else
    THROW("TypeError",
          "Expected to inherit from Class got " << accumulator.type_string());
  for (auto &p : class_->methods)
    if (p.second->is<FunctionInfo>()) {
      p.second = make_function(bp, p.second->as<FunctionInfo>());
      p.second->as<Function>()->super_class =
          accumulator.as_ptr()->as<Class>();
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
  auto property = constants[READ(utype)].as_ptr()->as<Symbol>();
  if (likely(object.is_ptr() && object.as_ptr()->is<Instance>())) {
    auto instance = object.as_ptr()->as<Instance>();
    auto iter = instance->properties.find(property);
    if (unlikely(iter == instance->properties.end()))
      THROW("PropertyError", "object does not have any property named "
                                 << static_cast<StringSlice>(*property));
    else
      accumulator = iter->second;
  } else if (object.is_ptr() && object.as_ptr()->is<Module>()) {
    auto module = object.as_ptr()->as<Module>();
    auto iter = module->module_variables.find(property);
    if (unlikely(iter == module->module_variables.end() ||
                 !iter->second.exported))
      THROW("NoModuleVariableError",
            "Module " << module->name << " does not export any variable named "
                      << static_cast<StringSlice>(*property));
    else
      accumulator = module_variables[iter->second.position];
  } else {
    THROW("TypeError",
          "Cannot get property from type " << object.type_string());
  }
});

handler(StoreProperty, {
  auto object = bp[READ(utype)];
  auto property = constants[READ(utype)].as_ptr()->as<Symbol>();
  if (likely(object.is_ptr() && object.as_ptr()->is<Instance>())) {
    auto instance = object.as_ptr()->as<Instance>();
    instance->properties.insert({property, accumulator});
  } else {
    THROW("TypeError", "Cannot set property for type " << object.type_string());
  }
});

handler(Range, {
  auto left = bp[READ(utype)];
  auto right = accumulator;
  if (left.is_int() && right.is_int()) {
    accumulator = Value(allocate<Range>(left.as_int(), right.as_int()));
  } else {
    THROW("TypeError",
          "Expected Int and Int for the start and end of the range got "
              << left.type_string() << " and " << right.type_string()
              << " instead");
  }
});

handler(LoadModuleVariable, accumulator = module_variables[READ(utype)];);
handler(StoreModuleVariable, module_variables[READ(utype)] = accumulator;);

#define BINARY_OP_INT(opname, intfn, op)                                       \
  do {                                                                         \
    if (accumulator.is_int()) {                                                \
      int result;                                                              \
      int i = READ(itype);                                                     \
      if (unlikely(!intfn(accumulator.as_int(), i, result))) {                 \
        THROW("OverflowError",                                                 \
              "Cannot " #opname " "                                            \
                  << accumulator.as_int() << " and " << i                      \
                  << " as the result does not fit in an Int");                 \
      }                                                                        \
      accumulator = Value(result);                                             \
    } else if (accumulator.is_float()) {                                       \
      accumulator = Value(accumulator.as_float() op READ(itype));              \
    } else {                                                                   \
      THROW("TypeError", "Cannot " #opname " types "                           \
                             << accumulator.type_string() << " and Int");      \
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
      THROW("OverflowError", "Cannot mod "
                                 << accumulator.as_int() << " and " << i
                                 << " as the result does not fit in an Int");
    }
    accumulator = Value(result);
  } else if (accumulator.is_float()) {
    accumulator = Value(fmod(accumulator.as_float(), READ(itype)));
  } else {
    THROW("TypeError",
          "Cannot mod types " << accumulator.type_string() << " and Int");
  }
});

handler(JumpBack, {
  auto offset = READ(utype);
  ip -= (offset + 1 + sizeof(utype) + header_size<utype>());
});

handler(ForLoop, {
  auto offset = READ(utype);
  auto iter = READ(utype);
  uint32_t end = static_cast<uint32_t>(iter) + 1;
  bp[iter].inc();
  if (bp[iter].as_int() < bp[end].as_int()) {
    ip -= (offset + 1 + 2 * sizeof(utype) + header_size<utype>());
  }
});

handler(Switch, {
  auto &jump_table =
      task->frames.back().f->function_info->jump_tables[READ(utype)];
  auto offset_iter = jump_table.find(accumulator);
  if (offset_iter != jump_table.end())
    ip += offset_iter->second;
});
