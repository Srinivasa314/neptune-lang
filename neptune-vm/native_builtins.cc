#include "neptune-vm.h"
#include <algorithm>
#include <cmath>
#define MATH_FNS                                                               \
  FN(acos)                                                                     \
  FN(asin)                                                                     \
  FN(atan)                                                                     \
  FN(cbrt)                                                                     \
  FN(ceil)                                                                     \
  FN(cos)                                                                      \
  FN(floor)                                                                    \
  FN(round)                                                                    \
  FN(sin)                                                                      \
  FN(sqrt)                                                                     \
  FN(tan)                                                                      \
  FN(log)                                                                      \
  FN(log2)                                                                     \
  FN(exp)

namespace neptune_vm {
namespace native_builtins {

#define THROW(class, message)                                                  \
  do {                                                                         \
    std::ostringstream os;                                                     \
    os << message;                                                             \
    vm->return_value = vm->create_error(class, os.str());                      \
    return false;                                                              \
  } while (0)

static bool object_tostring(VM *vm, Value *slots) {
  vm->return_value = vm->to_string(slots[0]);
  return true;
}

static bool object_getclass(VM *vm, Value *slots) {
  vm->return_value = Value(vm->get_class(slots[0]));
  return true;
}

static bool class_name(VM *vm, Value *slots) {
  vm->return_value =
      Value(vm->allocate<String>(slots[0].as_object()->as<Class>()->name));
  return true;
}

static bool class_getsuper(VM *vm, Value *slots) {
  auto super = slots[0].as_object()->as<Class>()->super;
  if (super == nullptr)
    vm->return_value = Value::null();
  else
    vm->return_value = Value(super);
  return true;
}

static bool array_pop(VM *vm, Value *slots) {
  auto &arr = slots[0].as_object()->as<Array>()->inner;
  if (arr.empty()) {
    THROW("IndexError", "Cannot pop from empty array");
  }
  vm->return_value = arr.back();
  arr.pop_back();
  return true;
}

static bool array_push(VM *vm, Value *slots) {
  slots[0].as_object()->as<Array>()->inner.push_back(slots[1]);
  vm->return_value = Value::null();
  return true;
}

static bool array_len(VM *vm, Value *slots) {
  vm->return_value = Value(
      static_cast<int32_t>(slots[0].as_object()->as<Array>()->inner.size()));
  return true;
}

static bool array_insert(VM *vm, Value *slots) {
  auto &arr = slots[0].as_object()->as<Array>()->inner;
  if (slots[1].is_int()) {
    auto index = slots[1].as_int();
    if (index < 0 || static_cast<size_t>(index) > arr.size())
      THROW("IndexError", "Array index out of range");
    arr.insert(arr.begin() + index, slots[2]);
    return true;
  } else
    THROW("TypeError",
          "Expected Int for array index got " << slots[1].type_string());
}

static bool array_remove(VM *vm, Value *slots) {
  auto &arr = slots[0].as_object()->as<Array>()->inner;
  if (slots[1].is_int()) {
    auto index = slots[1].as_int();
    if (index < 0 || static_cast<size_t>(index) >= arr.size())
      THROW("IndexError", "Array index out of range");
    arr.erase(arr.begin() + index);
    return true;
  } else
    THROW("TypeError",
          "Expected Int for array index got " << slots[1].type_string());
}

static bool array_clear(VM *, Value *slots) {
  slots[0].as_object()->as<Array>()->inner.clear();
  return true;
}

static bool int_construct(VM *vm, Value *) {
  vm->return_value = Value(0);
  return true;
}

static bool float_construct(VM *vm, Value *) {
  vm->return_value = Value(0.0);
  return true;
}

static bool bool_construct(VM *vm, Value *) {
  vm->return_value = Value(false);
  return true;
}

static bool null_construct(VM *vm, Value *) {
  vm->return_value = Value::null();
  return true;
}

static bool string_construct(VM *vm, Value *) {
  vm->return_value = Value(vm->allocate<String>(""));
  return true;
}

static bool string_find(VM *vm, Value *slots) {
  if (slots[1].is_object() && slots[1].as_object()->is<String>()) {
    auto haystack = slots[0].as_object()->as<String>();
    auto needle = slots[1].as_object()->as<String>();
    auto pos = String::find(haystack, needle, 0);
    if (pos == haystack->get_len())
      vm->return_value = Value(-1);
    else
      vm->return_value = Value(static_cast<int32_t>(pos));
    return true;
  } else
    THROW("TypeError", "Expected String got " << slots[1].type_string());
}

static bool array_construct(VM *vm, Value *slots) {
  if (slots[1].is_int()) {
    if (slots[1].as_int() < 0)
      THROW("Error", "The array size must be non negative");
    vm->return_value = Value(vm->allocate<Array>(slots[1].as_int(), slots[2]));
    return true;
  } else {
    THROW("TypeError",
          "The first argument must be a Int, not " << slots[1].type_string());
  }
}

static bool map_construct(VM *vm, Value *) {
  vm->return_value = Value(vm->allocate<Map>());
  return true;
}

static bool object_construct(VM *vm, Value *) {
  auto obj = vm->allocate<Instance>();
  obj->class_ = vm->builtin_classes.Object;
  vm->return_value = Value(obj);
  return true;
}

static bool range_construct(VM *vm, Value *slots) {
  if (slots[1].is_int() && slots[2].is_int()) {
    vm->return_value =
        Value(vm->allocate<Range>(slots[1].as_int(), slots[2].as_int()));
    return true;
  } else {
    THROW("TypeError",
          "Expected Int and Int for the start and end of the range got "
              << slots[1].type_string() << " and " << slots[2].type_string()
              << " instead");
  }
}

static bool symbol_construct(VM *vm, Value *slots) {
  if (slots[1].is_object() && slots[1].as_object()->is<String>()) {
    vm->return_value = Value(vm->intern(*slots[1].as_object()->as<String>()));
    return true;
  } else {
    THROW("TypeError", "The first argument must be a String, not "
                           << slots[1].type_string());
  }
}

static bool range_next(VM *vm, Value *slots) {
  auto &range = *slots[0].as_object()->as<Range>();
  vm->return_value = Value(range.start);
  if (range.start != range.end)
    range.start++;
  return true;
}

static bool range_hasnext(VM *vm, Value *slots) {
  auto &range = *slots[0].as_object()->as<Range>();
  vm->return_value = Value(range.start < range.end);
  return true;
}

static bool array_iter(VM *vm, Value *slots) {
  vm->return_value =
      Value(vm->allocate<ArrayIterator>(slots[0].as_object()->as<Array>()));
  return true;
}

static bool map_keys(VM *vm, Value *slots) {
  vm->return_value =
      Value(vm->allocate<MapIterator>(slots[0].as_object()->as<Map>()));
  return true;
}

static bool string_chars(VM *vm, Value *slots) {
  vm->return_value =
      Value(vm->allocate<StringIterator>(slots[0].as_object()->as<String>()));
  return true;
}

static bool mapiterator_hasnext(VM *vm, Value *slots) {
  vm->return_value =
      Value(!slots[0].as_object()->as<MapIterator>()->last_key.is_empty());
  return true;
}

static bool mapiterator_next(VM *vm, Value *slots) {
  auto mi = slots[0].as_object()->as<MapIterator>();
  if (mi->last_key.is_empty())
    vm->return_value = Value::null();
  else {
    vm->return_value = mi->last_key;
    auto iter = mi->map->inner.find(mi->last_key);
    if (iter == mi->map->inner.end())
      mi->last_key = Value(nullptr);
    else {
      iter++;
      if (iter == mi->map->inner.end())
        mi->last_key = Value(nullptr);
      else
        mi->last_key = iter->first;
    }
  }
  return true;
}

static bool arrayiterator_hasnext(VM *vm, Value *slots) {
  auto ai = slots[0].as_object()->as<ArrayIterator>();
  vm->return_value = Value(ai->position < ai->array->inner.size());
  return true;
}

static bool arrayiterator_next(VM *vm, Value *slots) {
  auto ai = slots[0].as_object()->as<ArrayIterator>();
  if (ai->position < ai->array->inner.size()) {
    vm->return_value = ai->array->inner[ai->position];
    ai->position++;
  } else {
    vm->return_value = Value::null();
  }
  return true;
}

static bool stringiterator_hasnext(VM *vm, Value *slots) {
  auto si = slots[0].as_object()->as<StringIterator>();
  auto str = static_cast<StringSlice>(*si->string);
  vm->return_value = Value(si->position < str.len);
  return true;
}

static bool stringiterator_next(VM *vm, Value *slots) {
  auto si = slots[0].as_object()->as<StringIterator>();
  auto str = static_cast<StringSlice>(*si->string);
  if (si->position < str.len) {
    auto old_pos = si->position;
    do {
      si->position++;
    } while (((uint8_t)str.data[si->position] & 0xc0) == 0x80);
    vm->return_value = Value(vm->allocate<String>(
        StringSlice(str.data + old_pos, si->position - old_pos)));
  } else {
    vm->return_value = Value::null();
  }
  return true;
}

#define FN(x)                                                                  \
  bool x(VM *vm, Value *slots) {                                               \
    auto num = slots[0];                                                       \
    if (num.is_int()) {                                                        \
      vm->return_value = Value(std::x(num.as_int()));                          \
      return true;                                                             \
    } else if (num.is_float()) {                                               \
      vm->return_value = Value(std::x(num.as_float()));                        \
      return true;                                                             \
    } else {                                                                   \
      THROW("TypeError", "The first argument must be a Int or Float, not "     \
                             << slots[0].type_string());                       \
    }                                                                          \
  }
MATH_FNS
#undef FN

bool pow(VM *vm, Value *slots) {
  if (slots[0].is_float() && slots[1].is_float()) {
    vm->return_value =
        Value(std::pow(slots[0].as_float(), slots[1].as_float()));
    return true;
  } else if (slots[0].is_int() && slots[1].is_int()) {
    vm->return_value = Value(std::pow(slots[0].as_int(), slots[1].as_int()));
    return true;
  } else if (slots[0].is_float() && slots[1].is_int()) {
    vm->return_value = Value(std::pow(slots[0].as_float(), slots[1].as_int()));
    return true;
  } else if (slots[0].is_int() && slots[1].is_float()) {
    vm->return_value = Value(std::pow(slots[0].as_int(), slots[1].as_float()));
    return true;
  } else {
    THROW("TypeError", "The two arguments must be a Int or Float, not "
                           << slots[0].type_string() << " and "
                           << slots[1].type_string());
  }
}

static bool abs(VM *vm, Value *slots) {
  auto num = slots[0];
  if (num.is_int()) {
    if (num.as_int() == std::numeric_limits<int32_t>::min())
      THROW("OverflowError",
            "abs of " << num.as_int() << " does not fit in an Int");
    vm->return_value = Value(std::abs(num.as_int()));
    return true;
  } else if (num.is_float()) {
    vm->return_value = Value(std::fabs(num.as_float()));
    return true;
  } else {
    THROW("TypeError", "The first argument must be a Int or Float, not "
                           << slots[1].type_string());
  }
}

static bool disassemble(VM *vm, Value *slots) {
  auto fn = slots[0];
  if (fn.is_object() && fn.as_object()->is<Function>()) {
    std::ostringstream os;
    neptune_vm::disassemble(os, *fn.as_object()->as<Function>()->function_info);
    vm->return_value = Value(vm->allocate<String>(os.str()));
    return true;
  } else if (fn.is_object() && fn.as_object()->is<NativeFunction>()) {
    THROW("TypeError", "Cannot disassemble native function "
                           << fn.as_object()->as<NativeFunction>()->name);
  } else {
    THROW("TypeError", "The first argument must be a Function, not "
                           << slots[0].type_string());
  }
}

static bool gc(VM *vm, Value *) {
  vm->collect();
  vm->return_value = Value::null();
  return true;
}

static bool _getModule(VM *vm, Value *slots) {
  if (slots[0].is_object() && slots[0].as_object()->is<String>()) {
    auto module =
        vm->get_module(StringSlice(*slots[0].as_object()->as<String>()));
    if (module == nullptr)
      vm->return_value = Value::null();
    else
      vm->return_value = Value(module);
    return true;
  } else {
    THROW("TypeError", "The first argument must be a Function, not "
                           << slots[0].type_string());
  }
}

static bool _getCallerModule(VM *vm, Value *) {
  if (vm->current_task->frames.size() < 2) {
    THROW("Error", "Function doesnt have caller");
  } else {
    vm->return_value = Value(vm->allocate<String>(
        vm->current_task->frames[vm->current_task->frames.size() - 2]
            .f->function_info->module));
    return true;
  }
}

static bool ecall(VM *vm, Value *slots) {
  if (slots[0].is_object() && slots[0].as_object()->is<Symbol>()) {
    auto efunc_iter = vm->efuncs.find(slots[0].as_object()->as<Symbol>());
    if (efunc_iter == vm->efuncs.end()) {
      THROW("Error", "Cannot find EFunc "
                         << StringSlice(*slots[0].as_object()->as<Symbol>()));

    } else {
      auto task = vm->current_task;
      auto efunc = efunc_iter->second;
      auto old_stack_top = task->stack_top;
      task->stack_top = slots + 2;
      bool result =
          efunc.callback(EFuncContext(vm, task, slots + 1), efunc.data);
      if (task->stack_top == slots + 1)
        vm->return_value = Value::null();
      else {
        vm->return_value = *(task->stack_top - 1);
      }
      task->stack_top = old_stack_top;
      return result;
    }
  } else {
    THROW("TypeError", "The first argument must be a Symbol, not "
                           << slots[0].type_string());
  }
}

static bool generateStackTrace(VM *vm, Value *slots) {
  if (slots[0].is_int()) {
    vm->return_value = Value(vm->allocate<String>(
        vm->generate_stack_trace(false, slots[0].as_int())));
    return true;
  } else {
    THROW("TypeError",
          "The first argument must be a Int, not " << slots[0].type_string());
  }
}

static bool _extendClass(VM *vm, Value *slots) {
  if (slots[0].is_object() && slots[0].as_object()->is<Class>() &&
      slots[1].is_object() && slots[1].as_object()->is<Class>()) {
    auto class0 = slots[0].as_object()->as<Class>();
    auto class1 = slots[1].as_object()->as<Class>();
    if (class1->is_native && class1 != vm->builtin_classes.Object)
      THROW("TypeError", "Cannot inherit from native class");
    class0->super = class1;
    vm->return_value = Value::null();
    return true;
  } else {
    THROW("TypeError", "Expected Class and Class for  got "
                           << slots[0].type_string() << " and "
                           << slots[1].type_string() << " instead");
  }
}

static bool _copyMethods(VM *vm, Value *slots) {
  if (slots[0].is_object() && slots[0].as_object()->is<Class>() &&
      slots[1].is_object() && slots[1].as_object()->is<Class>()) {
    auto class0 = slots[0].as_object()->as<Class>();
    auto class1 = slots[1].as_object()->as<Class>();
    if (class1->is_native)
      THROW("TypeError", "Cannot copy methods from native class");
    class0->copy_methods(*class1);
    vm->return_value = Value::null();
    return true;
  } else {
    THROW("TypeError", "Expected Class and Class for  got "
                           << slots[0].type_string() << " and "
                           << slots[1].type_string() << " instead");
  }
}

static bool random(VM *vm, Value *) {
  std::uniform_real_distribution<double> dist(0.0, 1.0);
  vm->return_value = Value(dist(vm->rng));
  return true;
}

static bool shuffle(VM *vm, Value *slots) {
  if (slots[0].is_object() && slots[0].as_object()->is<Array>()) {
    auto &array = slots[0].as_object()->as<Array>()->inner;
    std::shuffle(array.begin(), array.end(), vm->rng);
    vm->return_value = Value::null();
    return true;
  } else {
    THROW("TypeError", "The first argument must be an Array, not "
                           << slots[0].type_string());
  }
}

static bool random_range(VM *vm, Value *slots) {
  if (slots[0].is_int() && slots[1].is_int()) {
    std::uniform_int_distribution<int32_t> dist(slots[0].as_int(),
                                                slots[1].as_int());
    vm->return_value = Value(dist(vm->rng));
    return true;
  } else {
    THROW("TypeError",
          "Expected Int and Int for the start and end of the range got "
              << slots[0].type_string() << " and " << slots[1].type_string()
              << " instead");
  }
}

static bool map_clear(VM *vm, Value *slots) {
  slots[0].as_object()->as<Map>()->inner.clear();
  vm->return_value = Value::null();
  return true;
}

static bool map_len(VM *vm, Value *slots) {
  vm->return_value =
      Value((int32_t)slots[0].as_object()->as<Map>()->inner.size());
  return true;
}

static bool map_contains(VM *vm, Value *slots) {
  vm->return_value =
      Value(slots[0].as_object()->as<Map>()->inner.contains(slots[1]));
  return true;
}

static bool map_remove(VM *vm, Value *slots) {
  if (!slots[0].as_object()->as<Map>()->inner.erase(slots[1]))
    THROW("KeyError", "Key " << slots[1] << " does not exist in map.");
  vm->return_value = Value::null();
  return true;
}

static bool range_start(VM *vm, Value *slots) {
  vm->return_value = Value(slots[0].as_object()->as<Range>()->start);
  return true;
}

static bool range_end(VM *vm, Value *slots) {
  vm->return_value = Value(slots[0].as_object()->as<Range>()->end);
  return true;
}
static bool float_toint(VM *vm, Value *slots) {
  auto f = slots[0].as_float();
  if (std::isnan(f) || f > std::numeric_limits<int32_t>::max() ||
      f < std::numeric_limits<int32_t>::min())
    THROW("OverflowError", slots[0].as_float() << " does not fit in an Int");
  vm->return_value = Value(int(f));
  return true;
}

static bool int_tofloat(VM *vm, Value *slots) {
  vm->return_value = Value(double(slots[0].as_int()));
  return true;
}

static bool float_isnan(VM *vm, Value *slots) {
  vm->return_value = Value(bool(std::isnan(slots[0].as_float())));
  return true;
}

#undef THROW
} // namespace native_builtins

void VM::declare_native_builtins() {
#define DEFCLASS(Name)                                                         \
  builtin_classes.Name = allocate<Class>();                                    \
  builtin_classes.Name->name = #Name;                                          \
  builtin_classes.Name->is_native = true;                                      \
  builtin_classes.Name->super = builtin_classes.Object;                        \
  add_module_variable("<prelude>", StringSlice(#Name), false, true);           \
  module_variables[module_variables.size() - 1] = Value(builtin_classes.Name);

  DEFCLASS(Object)
  builtin_classes.Object->super = nullptr;
  builtin_classes.Class_ = allocate<Class>();
  builtin_classes.Class_->name = "Class";
  builtin_classes.Class_->super = builtin_classes.Object;
  builtin_classes.Class_->is_native = true;
  add_module_variable("<prelude>", "Class", false, true);
  module_variables[module_variables.size() - 1] = Value(builtin_classes.Class_);

  DEFCLASS(Int)
  DEFCLASS(Float)
  DEFCLASS(Bool)
  DEFCLASS(Null)
  DEFCLASS(String)
  DEFCLASS(Symbol)
  DEFCLASS(Array)
  DEFCLASS(Map)
  DEFCLASS(Function)
  DEFCLASS(Module)
  DEFCLASS(Task)
  DEFCLASS(Range)
  DEFCLASS(ArrayIterator)
  DEFCLASS(MapIterator)
  DEFCLASS(StringIterator)

#undef DEFCLASS

#define DECL_NATIVE_METHOD(class, method, arity, fn)                           \
  do {                                                                         \
    auto method_sym = intern(StringSlice(#method));                            \
    temp_roots.push_back(Value(method_sym));                                   \
    builtin_classes.class->methods.insert(                                     \
        {method_sym, allocate<NativeFunction>(native_builtins::fn, #method,    \
                                              "<prelude>", arity)});           \
    temp_roots.pop_back();                                                     \
  } while (0)

  DECL_NATIVE_METHOD(Object, toString, 0, object_tostring);
  DECL_NATIVE_METHOD(Object, getClass, 0, object_getclass);
  DECL_NATIVE_METHOD(Array, push, 1, array_push);
  DECL_NATIVE_METHOD(Array, pop, 0, array_pop);
  DECL_NATIVE_METHOD(Array, len, 0, array_len);
  DECL_NATIVE_METHOD(Array, insert, 2, array_insert);
  DECL_NATIVE_METHOD(Array, remove, 1, array_remove);
  DECL_NATIVE_METHOD(Array, clear, 0, array_clear);
  DECL_NATIVE_METHOD(String, find, 1, string_find);
  DECL_NATIVE_METHOD(Int, construct, 0, int_construct);
  DECL_NATIVE_METHOD(Float, construct, 0, float_construct);
  DECL_NATIVE_METHOD(Bool, construct, 0, bool_construct);
  DECL_NATIVE_METHOD(Null, construct, 0, null_construct);
  DECL_NATIVE_METHOD(String, construct, 0, string_construct);
  DECL_NATIVE_METHOD(Array, construct, 2, array_construct);
  DECL_NATIVE_METHOD(Map, construct, 0, map_construct);
  DECL_NATIVE_METHOD(Object, construct, 0, object_construct);
  DECL_NATIVE_METHOD(Range, construct, 2, range_construct);
  DECL_NATIVE_METHOD(Symbol, construct, 1, symbol_construct);
  DECL_NATIVE_METHOD(Range, hasNext, 0, range_hasnext);
  DECL_NATIVE_METHOD(Range, next, 0, range_next);
  DECL_NATIVE_METHOD(Array, iter, 0, array_iter);
  DECL_NATIVE_METHOD(Map, keys, 0, map_keys);
  DECL_NATIVE_METHOD(String, chars, 0, string_chars);
  DECL_NATIVE_METHOD(Array, iter, 0, array_iter);
  DECL_NATIVE_METHOD(MapIterator, hasNext, 0, mapiterator_hasnext);
  DECL_NATIVE_METHOD(MapIterator, next, 0, mapiterator_next);
  DECL_NATIVE_METHOD(ArrayIterator, hasNext, 0, arrayiterator_hasnext);
  DECL_NATIVE_METHOD(ArrayIterator, next, 0, arrayiterator_next);
  DECL_NATIVE_METHOD(StringIterator, hasNext, 0, stringiterator_hasnext);
  DECL_NATIVE_METHOD(StringIterator, next, 0, stringiterator_next);
  DECL_NATIVE_METHOD(Class_, getSuper, 0, class_getsuper);
  DECL_NATIVE_METHOD(Class_, name, 0, class_name);
  DECL_NATIVE_METHOD(Map, clear, 0, map_clear);
  DECL_NATIVE_METHOD(Map, len, 0, map_len);
  DECL_NATIVE_METHOD(Map, contains, 1, map_contains);
  DECL_NATIVE_METHOD(Map, remove, 1, map_remove);
  DECL_NATIVE_METHOD(Range, start, 0, range_start);
  DECL_NATIVE_METHOD(Range, end, 0, range_end);
  DECL_NATIVE_METHOD(Float, toInt, 0, float_toint);
  DECL_NATIVE_METHOD(Int, toFloat, 0, int_tofloat);
  DECL_NATIVE_METHOD(Float, isNaN, 0, float_isnan);

  create_module("vm");
  create_module("math");
  create_module("random");
  declare_native_function("vm", "disassemble", true, 1,
                          native_builtins::disassemble);
  declare_native_function("vm", "gc", true, 0, native_builtins::gc);
  declare_native_function("vm", "ecall", true, 2, native_builtins::ecall);
  declare_native_function("vm", "generateStackTrace", true, 1,
                          native_builtins::generateStackTrace);

#define FN(x) declare_native_function("math", #x, true, 1, native_builtins::x);

  MATH_FNS
#undef FN
  declare_native_function("math", "abs", true, 1, native_builtins::abs);
  declare_native_function("math", "pow", true, 2, native_builtins::pow);

  declare_native_function("<prelude>", "_getModule", false, 1,
                          native_builtins::_getModule);
  declare_native_function("<prelude>", "_getCallerModule", false, 0,
                          native_builtins::_getCallerModule);
  declare_native_function("<prelude>", "_extendClass", false, 2,
                          native_builtins::_extendClass);
  declare_native_function("<prelude>", "_copyMethods", false, 2,
                          native_builtins::_copyMethods);

  declare_native_function("random", "random", true, 0, native_builtins::random);
  declare_native_function("random", "shuffle", true, 1,
                          native_builtins::shuffle);
  declare_native_function("random", "range", true, 2,
                          native_builtins::random_range);
#define DEF_MATH_CONSTANT(name, value)                                         \
  add_module_variable("math", name, false, true);                              \
  module_variables[module_variables.size() - 1] = Value(value);

  DEF_MATH_CONSTANT("NaN", NAN)
  DEF_MATH_CONSTANT("Infinity", INFINITY)
  DEF_MATH_CONSTANT("E", M_E)
  DEF_MATH_CONSTANT("LN2", M_LN2)
  DEF_MATH_CONSTANT("LOG2E", M_LOG2E)
  DEF_MATH_CONSTANT("SQRT1_2", M_SQRT1_2)
  DEF_MATH_CONSTANT("LN10", M_LN10)
  DEF_MATH_CONSTANT("LOG10E", M_LOG10E)
  DEF_MATH_CONSTANT("PI", M_PI)
  DEF_MATH_CONSTANT("SQRT2", M_SQRT2)

#undef DEF_MATH_CONSTANT
}
} // namespace neptune_vm