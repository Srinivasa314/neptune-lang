#include "neptune-vm.h"

#define CHECK_STACK_UNDERFLOW                                                  \
  if (task->stack_top == arg)                                                  \
  return EFuncStatus::Underflow

namespace neptune_vm {
EFuncContext::EFuncContext(VM *vm, Value *arg, Task *task)
    : vm(vm), task(task), arg(arg) {}
void EFuncContext::push(Value v) {
  if (task->stack_top == task->stack.get() + task->stack_size)
    arg = task->grow_stack(arg, 1);
  *task->stack_top = v;
  task->stack_top++;
}

Value EFuncContext::pop_value() {
  task->stack_top--;
  return *task->stack_top;
}

Value EFuncContext::peek() const { return task->stack_top[-1]; }

void EFuncContext::push_int(int32_t i) { push(Value(i)); }

void EFuncContext::push_float(double d) { push(Value(d)); }

void EFuncContext::push_bool(bool b) { push(Value(b)); }

void EFuncContext::push_null() { push(Value::null()); }

void EFuncContext::push_string(StringSlice s) {
  push(Value(vm->allocate<String>(s)));
}

void EFuncContext::push_symbol(StringSlice s) { push(Value(vm->intern(s))); }

void EFuncContext::push_empty_array() { push(Value(vm->allocate<Array>())); }

void EFuncContext::push_function(FunctionInfoWriter fw) {
  auto function = vm->make_function(nullptr, fw.hf->object);
  push(Value(function));
  fw.release();
}

EFuncStatus EFuncContext::push_to_array() {
  CHECK_STACK_UNDERFLOW;
  auto elem = pop_value();
  CHECK_STACK_UNDERFLOW;
  auto v = peek();
  if (v.is_ptr() && v.as_ptr()->is<Array>()) {
    auto &array = v.as_ptr()->as<Array>()->inner;
    array.push_back(elem);
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus EFuncContext::set_object_property(StringSlice s) {
  CHECK_STACK_UNDERFLOW;
  auto elem = pop_value();
  vm->temp_roots.push_back(elem);
  CHECK_STACK_UNDERFLOW;
  auto obj = peek();
  if (obj.is_ptr() && obj.as_ptr()->is<Instance>()) {
    auto &map = obj.as_ptr()->as<Instance>()->properties;
    map.insert({vm->intern(s), elem});
    vm->temp_roots.pop_back();
    return EFuncStatus::Ok;
  } else {
    vm->temp_roots.pop_back();
    return EFuncStatus::TypeError;
  }
}

void EFuncContext::push_empty_object() {
  auto obj = vm->allocate<Instance>();
  obj->class_ = vm->builtin_classes.Object;
  push(Value(obj));
}

void EFuncContext::push_empty_map() { push(Value(vm->allocate<Map>())); }

EFuncStatus EFuncContext::push_error(StringSlice module,
                                     StringSlice error_class,
                                     StringSlice message) {
  auto error = vm->create_error(module, error_class, message);
  if (error.is_null())
    return EFuncStatus::TypeError;
  else {
    push(error);
    return EFuncStatus::Ok;
  }
}

EFuncStatus EFuncContext::insert_in_map() {
  CHECK_STACK_UNDERFLOW;
  auto value = pop_value();
  CHECK_STACK_UNDERFLOW;
  auto key = pop_value();
  CHECK_STACK_UNDERFLOW;
  auto m = peek();
  if (m.is_ptr() && m.as_ptr()->is<Map>()) {
    auto &map = m.as_ptr()->as<Map>()->inner;
    map.insert({key, value});
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus EFuncContext::as_int(int32_t &i) {
  CHECK_STACK_UNDERFLOW;
  Value v = pop_value();
  if (v.is_int()) {
    i = v.as_int();
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus EFuncContext::as_float(double &d) {
  CHECK_STACK_UNDERFLOW;
  Value v = pop_value();
  if (v.is_float()) {
    d = v.as_float();
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus EFuncContext::as_bool(bool &b) {
  CHECK_STACK_UNDERFLOW;
  Value v = pop_value();
  if (v.is_bool()) {
    b = v.is_true();
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus EFuncContext::is_null() {
  CHECK_STACK_UNDERFLOW;
  Value v = pop_value();
  if (v.is_null()) {
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus EFuncContext::as_string(StringSlice &s) {
  CHECK_STACK_UNDERFLOW;
  Value v = pop_value();
  if (v.is_ptr() && v.as_ptr()->is<String>()) {
    s = StringSlice(*v.as_ptr()->as<String>());
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus EFuncContext::as_symbol(StringSlice &s) {
  CHECK_STACK_UNDERFLOW;
  Value v = pop_value();
  if (v.is_ptr() && v.as_ptr()->is<Symbol>()) {
    s = StringSlice(*v.as_ptr()->as<Symbol>());
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

bool EFuncContext::pop() {
  if (task->stack_top == arg)
    return false;
  else {
    task->stack_top--;
    return true;
  }
}
EFuncStatus EFuncContext::get_array_length(size_t &len) const {
  CHECK_STACK_UNDERFLOW;
  auto v = peek();
  if (v.is_ptr() && v.as_ptr()->is<Array>()) {
    auto &array = v.as_ptr()->as<Array>()->inner;
    len = array.size();
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus EFuncContext::get_array_element(size_t pos) {
  CHECK_STACK_UNDERFLOW;
  auto v = peek();
  if (v.is_ptr() && v.as_ptr()->is<Array>()) {
    auto &array = v.as_ptr()->as<Array>()->inner;
    if (pos >= array.size())
      return EFuncStatus::OutOfBoundsError;
    else {
      push(array[pos]);
      return EFuncStatus::Ok;
    }
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus EFuncContext::get_object_property(StringSlice prop) {
  CHECK_STACK_UNDERFLOW;
  auto obj = peek();
  if (obj.is_ptr() && obj.as_ptr()->is<Instance>()) {
    auto &map = obj.as_ptr()->as<Instance>()->properties;
    auto key = vm->intern(prop);
    auto iter = map.find(key);
    if (iter == map.end())
      return EFuncStatus::PropertyError;
    else {
      push(iter->second);
      return EFuncStatus::Ok;
    }
  } else
    return EFuncStatus::TypeError;
}

void EFuncContext::push_resource(Data *data, FreeDataCallback *free_data) {
  push(Value(vm->allocate<Resource>(data, free_data)));
}

Data *EFuncContext::as_resource(EFuncStatus &status) {
  if (task->stack_top == arg) {
    status = EFuncStatus::Underflow;
    return nullptr;
  }
  Value v = pop_value();
  if (v.is_ptr() && v.as_ptr()->is<Resource>()) {
    status = EFuncStatus::Ok;
    return v.as_ptr()->as<Resource>()->data;
  } else {
    status = EFuncStatus::TypeError;
    return nullptr;
  }
}
}; // namespace neptune_vm
