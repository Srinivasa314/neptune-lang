#include "neptune-vm.h"

#define CHECK_STACK_UNDERFLOW                                                  \
  if (task->stack_top == arg)                                                  \
  return EFuncStatus::Underflow

namespace neptune_vm {
void FunctionContext::push(Value v) {
  if (task->stack_top == task->stack.get() + (task->stack_size / sizeof(Value)))
    arg = task->grow_stack(arg, 1 * sizeof(Value));
  *task->stack_top = v;
  task->stack_top++;
}

Value FunctionContext::pop_value() {
  task->stack_top--;
  return *task->stack_top;
}

Value FunctionContext::peek() { return task->stack_top[-1]; }

void FunctionContext::push_int(int32_t i) { push(Value(i)); }

void FunctionContext::push_float(double d) { push(Value(d)); }

void FunctionContext::push_bool(bool b) { push(Value(b)); }

void FunctionContext::push_null() { push(Value::null()); }

void FunctionContext::push_string(StringSlice s) {
  push(Value(vm->manage(String::from(s))));
}

void FunctionContext::push_symbol(StringSlice s) { push(Value(vm->intern(s))); }

void FunctionContext::push_empty_array() {
  push(Value(vm->manage(new Array())));
}

EFuncStatus FunctionContext::push_to_array() {
  CHECK_STACK_UNDERFLOW;
  auto elem = pop_value();
  auto v = peek();
  if (v.is_object() && v.as_object()->is<Array>()) {
    auto &array = v.as_object()->as<Array>()->inner;
    array.push_back(elem);
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus FunctionContext::set_object_property(StringSlice s) {
  CHECK_STACK_UNDERFLOW;
  auto elem = pop_value();
  vm->temp_roots.push_back(elem);
  auto obj = peek();
  if (obj.is_object() && obj.as_object()->is<Instance>()) {
    auto &map = obj.as_object()->as<Instance>()->properties;
    map.insert({vm->intern(s), elem});
    vm->temp_roots.pop_back();
    return EFuncStatus::Ok;
  } else {
    vm->temp_roots.pop_back();
    return EFuncStatus::TypeError;
  }
}

void FunctionContext::push_empty_object() {
  push(Value(vm->manage(new Instance())));
}

EFuncStatus FunctionContext::as_int(int32_t &i) {
  CHECK_STACK_UNDERFLOW;
  Value v = pop_value();
  if (v.is_int()) {
    i = v.as_int();
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus FunctionContext::as_float(double &d) {
  CHECK_STACK_UNDERFLOW;
  Value v = pop_value();
  if (v.is_float()) {
    d = v.as_float();
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus FunctionContext::as_bool(bool &b) {
  CHECK_STACK_UNDERFLOW;
  Value v = pop_value();
  if (v.is_bool()) {
    b = v.is_true();
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus FunctionContext::is_null() {
  CHECK_STACK_UNDERFLOW;
  Value v = pop_value();
  if (v.is_null()) {
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus FunctionContext::is_object() {
  if (peek().is_object()) {
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus FunctionContext::as_string(rust::String &s) {
  CHECK_STACK_UNDERFLOW;
  Value v = pop_value();
  if (v.is_object() && v.as_object()->is<String>()) {
    s = rust::String(*v.as_object()->as<String>());
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus FunctionContext::as_symbol(rust::String &s) {
  CHECK_STACK_UNDERFLOW;
  Value v = pop_value();
  if (v.is_object() && v.as_object()->is<Symbol>()) {
    s = rust::String(*v.as_object()->as<Symbol>());
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

bool FunctionContext::pop() {
  if (task->stack_top == arg)
    return false;
  else {
    task->stack_top--;
    return true;
  }
}
EFuncStatus FunctionContext::get_array_length(size_t &len) {
  auto v = peek();
  if (v.is_object() && v.as_object()->is<Array>()) {
    auto &array = v.as_object()->as<Array>()->inner;
    len = array.size();
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus FunctionContext::get_array_element(size_t pos) {
  auto v = peek();
  if (v.is_object() && v.as_object()->is<Array>()) {
    auto &array = v.as_object()->as<Array>()->inner;
    if (pos >= array.size())
      return EFuncStatus::OutOfBounds;
    else {
      push(array[pos]);
      return EFuncStatus::Ok;
    }
  } else
    return EFuncStatus::TypeError;
}

EFuncStatus FunctionContext::get_object_property(StringSlice prop) {
  auto obj = peek();
  if (obj.is_object() && obj.as_object()->is<Instance>()) {
    auto &map = obj.as_object()->as<Instance>()->properties;
    auto key = vm->intern(prop);
    if (map.find(key) == map.end())
      return EFuncStatus::PropertyError;
    else {
      push(map[key]);
      return EFuncStatus::Ok;
    }
  } else
    return EFuncStatus::TypeError;
}
}; // namespace neptune_vm