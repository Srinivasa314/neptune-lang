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
    return VMStatus::Error;                                                    \
  } while (0)

static VMStatus object_tostring(VM *vm, Value *args) {
  vm->return_value = vm->to_string(args[0]);
  return VMStatus::Success;
}

static VMStatus object_getclass(VM *vm, Value *args) {
  vm->return_value = Value(vm->get_class(args[0]));
  return VMStatus::Success;
}

static VMStatus class_name(VM *vm, Value *args) {
  vm->return_value =
      Value(vm->allocate<String>(args[0].as_object()->as<Class>()->name));
  return VMStatus::Success;
}

static VMStatus class_getsuper(VM *vm, Value *args) {
  auto super = args[0].as_object()->as<Class>()->super;
  if (super == nullptr)
    vm->return_value = Value::null();
  else
    vm->return_value = Value(super);
  return VMStatus::Success;
}

static VMStatus array_pop(VM *vm, Value *args) {
  auto &arr = args[0].as_object()->as<Array>()->inner;
  if (arr.empty()) {
    THROW("IndexError", "Cannot pop from empty array");
  }
  vm->return_value = arr.back();
  arr.pop_back();
  return VMStatus::Success;
}

static VMStatus array_push(VM *vm, Value *args) {
  args[0].as_object()->as<Array>()->inner.push_back(args[1]);
  vm->return_value = Value::null();
  return VMStatus::Success;
}

static VMStatus array_len(VM *vm, Value *args) {
  vm->return_value = Value(
      static_cast<int32_t>(args[0].as_object()->as<Array>()->inner.size()));
  return VMStatus::Success;
}

static VMStatus array_insert(VM *vm, Value *args) {
  auto &arr = args[0].as_object()->as<Array>()->inner;
  if (args[1].is_int()) {
    auto index = args[1].as_int();
    if (index < 0 || static_cast<size_t>(index) > arr.size())
      THROW("IndexError", "Array index out of range");
    arr.insert(arr.begin() + index, args[2]);
    vm->return_value = Value::null();
    return VMStatus::Success;
  } else
    THROW("TypeError",
          "Expected Int for array index got " << args[1].type_string());
}

static VMStatus array_remove(VM *vm, Value *args) {
  auto &arr = args[0].as_object()->as<Array>()->inner;
  if (args[1].is_int()) {
    auto index = args[1].as_int();
    if (index < 0 || static_cast<size_t>(index) >= arr.size())
      THROW("IndexError", "Array index out of range");
    arr.erase(arr.begin() + index);
    vm->return_value = Value::null();
    return VMStatus::Success;
  } else
    THROW("TypeError",
          "Expected Int for array index got " << args[1].type_string());
}

static VMStatus array_clear(VM *vm, Value *args) {
  args[0].as_object()->as<Array>()->inner.clear();
  vm->return_value = Value::null();
  return VMStatus::Success;
}

static VMStatus int_construct(VM *vm, Value *) {
  vm->return_value = Value(0);
  return VMStatus::Success;
}

static VMStatus float_construct(VM *vm, Value *) {
  vm->return_value = Value(0.0);
  return VMStatus::Success;
}

static VMStatus bool_construct(VM *vm, Value *) {
  vm->return_value = Value(false);
  return VMStatus::Success;
}

static VMStatus null_construct(VM *vm, Value *) {
  vm->return_value = Value::null();
  return VMStatus::Success;
}

static VMStatus string_construct(VM *vm, Value *) {
  vm->return_value = Value(vm->allocate<String>(""));
  return VMStatus::Success;
}

static VMStatus string_find(VM *vm, Value *args) {
  if (args[1].is_object() && args[1].as_object()->is<String>()) {
    auto haystack = args[0].as_object()->as<String>();
    auto needle = args[1].as_object()->as<String>();
    auto pos = String::find(haystack, needle, 0);
    if (pos == haystack->get_len())
      vm->return_value = Value(-1);
    else
      vm->return_value = Value(static_cast<int32_t>(pos));
    return VMStatus::Success;
  } else
    THROW("TypeError",
          "The first argument must be a String, not " << args[1].type_string());
}

static VMStatus string_replace(VM *vm, Value *args) {
  if (args[1].is_object() && args[1].as_object()->is<String>() &&
      args[2].is_object() && args[2].as_object()->is<String>()) {
    vm->return_value = Value(args[0].as_object()->as<String>()->replace(
        vm, args[1].as_object()->as<String>(),
        args[2].as_object()->as<String>()));
    return VMStatus::Success;
  } else
    THROW("TypeError",
          "The first and second argument must be a String and String, not "
              << args[1].type_string() << " and " << args[2].type_string());
}

static VMStatus array_construct(VM *vm, Value *args) {
  if (args[1].is_int()) {
    if (args[1].as_int() < 0)
      THROW("Error", "The array size must be non negative");
    vm->return_value = Value(vm->allocate<Array>(args[1].as_int(), args[2]));
    return VMStatus::Success;
  } else {
    THROW("TypeError",
          "The first argument must be a Int, not " << args[1].type_string());
  }
}

static VMStatus map_construct(VM *vm, Value *) {
  vm->return_value = Value(vm->allocate<Map>());
  return VMStatus::Success;
}

static VMStatus object_construct(VM *vm, Value *) {
  auto obj = vm->allocate<Instance>();
  obj->class_ = vm->builtin_classes.Object;
  vm->return_value = Value(obj);
  return VMStatus::Success;
}

static VMStatus range_construct(VM *vm, Value *args) {
  if (args[1].is_int() && args[2].is_int()) {
    vm->return_value =
        Value(vm->allocate<Range>(args[1].as_int(), args[2].as_int()));
    return VMStatus::Success;
  } else {
    THROW("TypeError",
          "Expected Int and Int for the start and end of the range got "
              << args[1].type_string() << " and " << args[2].type_string()
              << " instead");
  }
}

static VMStatus symbol_construct(VM *vm, Value *args) {
  if (args[1].is_object() && args[1].as_object()->is<String>()) {
    vm->return_value = Value(vm->intern(*args[1].as_object()->as<String>()));
    return VMStatus::Success;
  } else {
    THROW("TypeError",
          "The first argument must be a String, not " << args[1].type_string());
  }
}

static VMStatus range_next(VM *vm, Value *args) {
  auto &range = *args[0].as_object()->as<Range>();
  vm->return_value = Value(range.start);
  if (range.start != range.end)
    range.start++;
  return VMStatus::Success;
}

static VMStatus range_hasnext(VM *vm, Value *args) {
  auto &range = *args[0].as_object()->as<Range>();
  vm->return_value = Value(range.start < range.end);
  return VMStatus::Success;
}

static VMStatus array_iter(VM *vm, Value *args) {
  vm->return_value =
      Value(vm->allocate<ArrayIterator>(args[0].as_object()->as<Array>()));
  return VMStatus::Success;
}

static VMStatus map_keys(VM *vm, Value *args) {
  vm->return_value =
      Value(vm->allocate<MapIterator>(args[0].as_object()->as<Map>()));
  return VMStatus::Success;
}

static VMStatus string_chars(VM *vm, Value *args) {
  vm->return_value =
      Value(vm->allocate<StringIterator>(args[0].as_object()->as<String>()));
  return VMStatus::Success;
}

static VMStatus mapiterator_hasnext(VM *vm, Value *args) {
  vm->return_value =
      Value(!args[0].as_object()->as<MapIterator>()->last_key.is_empty());
  return VMStatus::Success;
}

static VMStatus mapiterator_next(VM *vm, Value *args) {
  auto mi = args[0].as_object()->as<MapIterator>();
  if (mi->last_key.is_empty())
    vm->return_value = Value::null();
  else {
    vm->return_value = mi->last_key;
    auto iter = mi->map->inner.find(mi->last_key);
    if (iter == mi->map->inner.end())
      mi->last_key = Value(nullptr);
    else {
      ++iter;
      if (iter == mi->map->inner.end())
        mi->last_key = Value(nullptr);
      else
        mi->last_key = iter->first;
    }
  }
  return VMStatus::Success;
}

static VMStatus arrayiterator_hasnext(VM *vm, Value *args) {
  auto ai = args[0].as_object()->as<ArrayIterator>();
  vm->return_value = Value(ai->position < ai->array->inner.size());
  return VMStatus::Success;
}

static VMStatus arrayiterator_next(VM *vm, Value *args) {
  auto ai = args[0].as_object()->as<ArrayIterator>();
  if (ai->position < ai->array->inner.size()) {
    vm->return_value = ai->array->inner[ai->position];
    ai->position++;
  } else {
    vm->return_value = Value::null();
  }
  return VMStatus::Success;
}

static VMStatus stringiterator_hasnext(VM *vm, Value *args) {
  auto si = args[0].as_object()->as<StringIterator>();
  auto str = static_cast<StringSlice>(*si->string);
  vm->return_value = Value(si->position < str.len);
  return VMStatus::Success;
}

static VMStatus stringiterator_next(VM *vm, Value *args) {
  auto si = args[0].as_object()->as<StringIterator>();
  auto str = static_cast<StringSlice>(*si->string);
  if (si->position < str.len) {
    auto old_pos = si->position;
    do {
      si->position++;
    } while (si->position < str.len &&
             ((uint8_t)str.data[si->position] & 0xc0) == 0x80);
    vm->return_value = Value(vm->allocate<String>(
        StringSlice(str.data + old_pos, si->position - old_pos)));
  } else {
    vm->return_value = Value::null();
  }
  return VMStatus::Success;
}

#define FN(x)                                                                  \
  VMStatus x(VM *vm, Value *args) {                                            \
    auto num = args[0];                                                        \
    if (num.is_int()) {                                                        \
      vm->return_value = Value(std::x(num.as_int()));                          \
      return VMStatus::Success;                                                \
    } else if (num.is_float()) {                                               \
      vm->return_value = Value(std::x(num.as_float()));                        \
      return VMStatus::Success;                                                \
    } else {                                                                   \
      THROW("TypeError", "The first argument must be a Int or Float, not "     \
                             << args[0].type_string());                        \
    }                                                                          \
  }
MATH_FNS
#undef FN

VMStatus pow(VM *vm, Value *args) {
  if (args[0].is_float() && args[1].is_float()) {
    vm->return_value = Value(std::pow(args[0].as_float(), args[1].as_float()));
    return VMStatus::Success;
  } else if (args[0].is_int() && args[1].is_int()) {
    vm->return_value = Value(std::pow(args[0].as_int(), args[1].as_int()));
    return VMStatus::Success;
  } else if (args[0].is_float() && args[1].is_int()) {
    vm->return_value = Value(std::pow(args[0].as_float(), args[1].as_int()));
    return VMStatus::Success;
  } else if (args[0].is_int() && args[1].is_float()) {
    vm->return_value = Value(std::pow(args[0].as_int(), args[1].as_float()));
    return VMStatus::Success;
  } else {
    THROW("TypeError", "The two arguments must be a Int or Float, not "
                           << args[0].type_string() << " and "
                           << args[1].type_string());
  }
}

static VMStatus abs(VM *vm, Value *args) {
  auto num = args[0];
  if (num.is_int()) {
    if (num.as_int() == std::numeric_limits<int32_t>::min())
      THROW("OverflowError",
            "abs of " << num.as_int() << " does not fit in an Int");
    vm->return_value = Value(std::abs(num.as_int()));
    return VMStatus::Success;
  } else if (num.is_float()) {
    vm->return_value = Value(std::fabs(num.as_float()));
    return VMStatus::Success;
  } else {
    THROW("TypeError", "The first argument must be a Int or Float, not "
                           << args[1].type_string());
  }
}

static VMStatus disassemble(VM *vm, Value *args) {
  auto fn = args[0];
  if (fn.is_object() && fn.as_object()->is<Function>()) {
    std::ostringstream os;
    neptune_vm::disassemble(os, *fn.as_object()->as<Function>()->function_info);
    vm->return_value = Value(vm->allocate<String>(os.str()));
    return VMStatus::Success;
  } else if (fn.is_object() && fn.as_object()->is<NativeFunction>()) {
    THROW("TypeError", "Cannot disassemble native function "
                           << fn.as_object()->as<NativeFunction>()->name);
  } else {
    THROW("TypeError", "The first argument must be a Function, not "
                           << args[0].type_string());
  }
}

static VMStatus gc(VM *vm, Value *) {
  vm->collect();
  vm->return_value = Value::null();
  return VMStatus::Success;
}

static VMStatus _getModule(VM *vm, Value *args) {
  if (args[0].is_object() && args[0].as_object()->is<String>()) {
    auto module =
        vm->get_module(StringSlice(*args[0].as_object()->as<String>()));
    if (module == nullptr)
      vm->return_value = Value::null();
    else
      vm->return_value = Value(module);
    return VMStatus::Success;
  } else {
    THROW("TypeError", "The first argument must be a Function, not "
                           << args[0].type_string());
  }
}

static VMStatus _getCallerModule(VM *vm, Value *) {
  if (vm->current_task->frames.size() < 2) {
    THROW("Error", "Function doesnt have caller");
  } else {
    vm->return_value = Value(vm->allocate<String>(
        vm->current_task->frames[vm->current_task->frames.size() - 2]
            .f->function_info->module));
    return VMStatus::Success;
  }
}

static VMStatus ecall(VM *vm, Value *args) {
  if (args[0].is_object() && args[0].as_object()->is<Symbol>()) {
    auto efunc_iter = vm->efuncs.find(args[0].as_object()->as<Symbol>());
    if (efunc_iter == vm->efuncs.end()) {
      THROW("Error", "Cannot find EFunc "
                         << StringSlice(*args[0].as_object()->as<Symbol>()));

    } else {
      auto task = vm->current_task;
      auto efunc = efunc_iter->second;
      auto old_stack_top = task->stack_top;
      task->stack_top = args + 2;
      VMStatus result = efunc.callback(
          EFuncContext(vm, args + 1, vm->current_task), efunc.data);
      if (result == VMStatus::Suspend) {
        task->waiting_for_rust_future = true;
        return VMStatus::Suspend;
      }
      if (task->stack_top == args + 1)
        vm->return_value = Value::null();
      else {
        vm->return_value = *(task->stack_top - 1);
      }
      task->stack_top = old_stack_top;
      return result;
    }
  } else {
    THROW("TypeError",
          "The first argument must be a Symbol, not " << args[0].type_string());
  }
}

static VMStatus generateStackTrace(VM *vm, Value *args) {
  if (args[0].is_int()) {
    vm->return_value = Value(vm->allocate<String>(
        vm->generate_stack_trace(false, args[0].as_int())));
    return VMStatus::Success;
  } else {
    THROW("TypeError",
          "The first argument must be a Int, not " << args[0].type_string());
  }
}

static VMStatus _extendClass(VM *vm, Value *args) {
  if (args[0].is_object() && args[0].as_object()->is<Class>() &&
      args[1].is_object() && args[1].as_object()->is<Class>()) {
    auto class0 = args[0].as_object()->as<Class>();
    auto class1 = args[1].as_object()->as<Class>();
    if (class1->is_native && class1 != vm->builtin_classes.Object)
      THROW("TypeError", "Cannot inherit from native class");
    class0->super = class1;
    vm->return_value = Value::null();
    return VMStatus::Success;
  } else {
    THROW("TypeError", "Expected Class and Class for  got "
                           << args[0].type_string() << " and "
                           << args[1].type_string() << " instead");
  }
}

static VMStatus _copyMethods(VM *vm, Value *args) {
  if (args[0].is_object() && args[0].as_object()->is<Class>() &&
      args[1].is_object() && args[1].as_object()->is<Class>()) {
    auto class0 = args[0].as_object()->as<Class>();
    auto class1 = args[1].as_object()->as<Class>();
    if (class1->is_native)
      THROW("TypeError", "Cannot copy methods from native class");
    class0->copy_methods(*class1);
    vm->return_value = Value::null();
    return VMStatus::Success;
  } else {
    THROW("TypeError", "Expected Class and Class for  got "
                           << args[0].type_string() << " and "
                           << args[1].type_string() << " instead");
  }
}

static VMStatus random(VM *vm, Value *) {
  std::uniform_real_distribution<double> dist(0.0, 1.0);
  vm->return_value = Value(dist(vm->rng));
  return VMStatus::Success;
}

static VMStatus shuffle(VM *vm, Value *args) {
  if (args[0].is_object() && args[0].as_object()->is<Array>()) {
    auto &array = args[0].as_object()->as<Array>()->inner;
    std::shuffle(array.begin(), array.end(), vm->rng);
    vm->return_value = Value::null();
    return VMStatus::Success;
  } else {
    THROW("TypeError",
          "The first argument must be an Array, not " << args[0].type_string());
  }
}

static VMStatus random_range(VM *vm, Value *args) {
  if (args[0].is_int() && args[1].is_int()) {
    std::uniform_int_distribution<int32_t> dist(args[0].as_int(),
                                                args[1].as_int());
    vm->return_value = Value(dist(vm->rng));
    return VMStatus::Success;
  } else {
    THROW("TypeError",
          "Expected Int and Int for the start and end of the range got "
              << args[0].type_string() << " and " << args[1].type_string()
              << " instead");
  }
}

static VMStatus map_clear(VM *vm, Value *args) {
  args[0].as_object()->as<Map>()->inner.clear();
  vm->return_value = Value::null();
  return VMStatus::Success;
}

static VMStatus map_len(VM *vm, Value *args) {
  vm->return_value =
      Value((int32_t)args[0].as_object()->as<Map>()->inner.size());
  return VMStatus::Success;
}

static VMStatus map_contains(VM *vm, Value *args) {
  vm->return_value =
      Value(args[0].as_object()->as<Map>()->inner.count(args[1]) == 1);
  return VMStatus::Success;
}

static VMStatus map_remove(VM *vm, Value *args) {
  if (!args[0].as_object()->as<Map>()->inner.erase(args[1]))
    THROW("KeyError", "Key " << args[1] << " does not exist in map.");
  vm->return_value = Value::null();
  return VMStatus::Success;
}

static VMStatus range_start(VM *vm, Value *args) {
  vm->return_value = Value(args[0].as_object()->as<Range>()->start);
  return VMStatus::Success;
}

static VMStatus range_end(VM *vm, Value *args) {
  vm->return_value = Value(args[0].as_object()->as<Range>()->end);
  return VMStatus::Success;
}
static VMStatus float_toint(VM *vm, Value *args) {
  auto f = args[0].as_float();
  if (std::isnan(f) || f > std::numeric_limits<int32_t>::max() ||
      f < std::numeric_limits<int32_t>::min())
    THROW("OverflowError", args[0].as_float() << " does not fit in an Int");
  vm->return_value = Value(int(f));
  return VMStatus::Success;
}

static VMStatus int_tofloat(VM *vm, Value *args) {
  vm->return_value = Value(double(args[0].as_int()));
  return VMStatus::Success;
}

static VMStatus float_isnan(VM *vm, Value *args) {
  vm->return_value = Value(bool(std::isnan(args[0].as_float())));
  return VMStatus::Success;
}

static VMStatus string_len(VM *vm, Value *args) {
  vm->return_value =
      Value(int32_t(args[0].as_object()->as<String>()->get_len()));
  return VMStatus::Success;
}

static VMStatus suspendCurrentTask(VM *vm, Value *) {
  vm->tasks_queue.push_back({vm->current_task, Value::null(), false});
  return VMStatus::Suspend;
}

static VMStatus spawn(VM *vm, Value *args) {
  if (args[0].is_object() && args[0].as_object()->is<Function>()) {
    Task *t = vm->allocate<Task>(args[0].as_object()->as<Function>());
    vm->return_value = Value(t);
    vm->tasks_queue.push_back({t, Value::null(), false});
    vm->main_task->links.insert(t);
    return VMStatus::Success;
  } else
    THROW("TypeError", "The first argument must be a Function, not "
                           << args[0].type_string());
}

static VMStatus spawn_link(VM *vm, Value *args) {
  auto status = spawn(vm, args);
  if (status == VMStatus::Success) {
    auto task = vm->return_value.as_object()->as<Task>();
    task->links.insert(vm->current_task);
    vm->current_task->links.insert(task);
  }
  return status;
}

static VMStatus task_kill(VM *vm, Value *args) {
  vm->kill(args[0].as_object()->as<Task>(), args[1]);
  if (vm->current_task->status == VMStatus::Error) {
    vm->current_task->status = VMStatus::Suspend;
    vm->return_value = vm->current_task->uncaught_exception;
    vm->current_task->uncaught_exception = Value::null();
    return VMStatus::Error;
  }
  return VMStatus::Success;
}

static VMStatus channel_construct(VM *vm, Value *) {
  vm->return_value = Value(vm->allocate<Channel>());
  return VMStatus::Success;
}

static VMStatus channel_send(VM *vm, Value *args) {
  args[0].as_object()->as<Channel>()->send(args[1], vm);
  vm->return_value = Value::null();
  return VMStatus::Success;
}

static VMStatus channel_recv(VM *vm, Value *args) {
  auto chan = args[0].as_object()->as<Channel>();
  if (chan->queue.empty()) {
    chan->wait_list.push_back(vm->current_task);
    return VMStatus::Suspend;
  } else {
    vm->return_value = chan->queue.front();
    chan->queue.pop_front();
    return VMStatus::Success;
  }
}

static VMStatus task_name(VM *vm, Value *args) {
  auto name = args[0].as_object()->as<Task>()->name;
  if (name == nullptr)
    vm->return_value = Value::null();
  else
    vm->return_value = Value(name);
  return VMStatus::Success;
}

static VMStatus task_setname(VM *vm, Value *args) {
  if (args[1].is_object() && args[1].as_object()->is<String>()) {
    args[0].as_object()->as<Task>()->name = args[1].as_object()->as<String>();
    return VMStatus::Success;
  } else
    THROW("TypeError",
          "The first argument must be a String, not " << args[1].type_string());
}

static VMStatus task_monitor(VM *vm, Value *args) {
  if (args[1].is_object() && args[1].as_object()->is<Channel>()) {
    auto task = args[0].as_object()->as<Task>();
    auto chan = args[1].as_object()->as<Channel>();
    if (task->status == VMStatus::Suspend)
      args[0].as_object()->as<Task>()->monitors.push_back(chan);
    else
      chan->send(Value(task), vm);
    return VMStatus::Success;
  } else
    THROW("TypeError", "The first argument must be a Channel, not "
                           << args[1].type_string());
}

static VMStatus task_link(VM *vm, Value *args) {
  if (args[1].is_object() && args[1].as_object()->is<Task>()) {
    auto task0 = args[0].as_object()->as<Task>();
    auto task1 = args[1].as_object()->as<Task>();
    task0->links.insert(task1);
    task1->links.insert(task0);
    return VMStatus::Success;
  } else
    THROW("TypeError",
          "The first argument must be a Task, not " << args[1].type_string());
}

static VMStatus currentTask(VM *vm, Value *) {
  vm->return_value = Value(vm->current_task);
  return VMStatus::Success;
}

static VMStatus task_status(VM *vm, Value *args) {
  switch (args[0].as_object()->as<Task>()->status) {
  case VMStatus::Suspend:
    vm->return_value = Value(vm->builtin_symbols.running);
    break;
  case VMStatus::Success:
    vm->return_value = Value(vm->builtin_symbols.finished);
    break;
  case VMStatus::Error:
    vm->return_value = Value(vm->builtin_symbols.killed);
  }
  return VMStatus::Success;
}

static VMStatus task_get_uncaught_exception(VM *vm, Value *args) {
  auto task = args[0].as_object()->as<Task>();
  if (task->status == VMStatus::Error)
    vm->return_value = task->uncaught_exception;
  else
    vm->return_value = Value::null();
  return VMStatus::Success;
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
  DEFCLASS(Channel)

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
  DECL_NATIVE_METHOD(String, replace, 2, string_replace);
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
  DECL_NATIVE_METHOD(String, len, 0, string_len);
  DECL_NATIVE_METHOD(Task, kill, 1, task_kill);
  DECL_NATIVE_METHOD(Channel, construct, 0, channel_construct);
  DECL_NATIVE_METHOD(Channel, send, 1, channel_send);
  DECL_NATIVE_METHOD(Channel, recv, 0, channel_recv);
  DECL_NATIVE_METHOD(Task, setName, 1, task_setname);
  DECL_NATIVE_METHOD(Task, name, 0, task_name);
  DECL_NATIVE_METHOD(Task, monitor, 1, task_monitor);
  DECL_NATIVE_METHOD(Task, link, 1, task_link);
  DECL_NATIVE_METHOD(Task, status, 0, task_status);
  DECL_NATIVE_METHOD(Task, getUncaughtException, 0,
                     task_get_uncaught_exception);

  create_module("vm");
  create_module("math");
  create_module("random");
  declare_native_function("vm", "disassemble", true, 1,
                          native_builtins::disassemble);
  declare_native_function("vm", "gc", true, 0, native_builtins::gc);
  declare_native_function("vm", "ecall", true, 2, native_builtins::ecall);
  declare_native_function("vm", "generateStackTrace", true, 1,
                          native_builtins::generateStackTrace);
  declare_native_function("vm", "suspendCurrentTask", true, 0,
                          native_builtins::suspendCurrentTask);
  declare_native_function("vm", "currentTask", true, 0,
                          native_builtins::currentTask);
  declare_native_function("<prelude>", "spawn", true, 1,
                          native_builtins::spawn);
  declare_native_function("<prelude>", "spawn_link", true, 1,
                          native_builtins::spawn_link);

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