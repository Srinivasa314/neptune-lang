#include "neptune-vm.h"

#define CHECK_STACK_UNDERFLOW                                                  \
  if (task->stack_top == arg + 1)                                              \
  return EFuncStatus::Underflow

namespace neptune_vm {
void FunctionContext::push(Value v) {
  if (task->stack_top == task->stack.get() + (task->stack_size / sizeof(Value)))
    arg = task->grow_stack(arg, 1 * sizeof(Value));
  *task->stack_top = v;
  task->stack_top++;
}
void FunctionContext::push_int(int32_t i) { push(Value(i)); }

bool FunctionContext::pop() {
  if (task->stack_top == task->stack.get())
    return false;
  else {
    task->stack_top--;
    return true;
  }
}
EFuncStatus FunctionContext::get_array_length(size_t &len) {
  CHECK_STACK_UNDERFLOW;
  auto v = *task->stack_top;
  if (v.is_object() && v.as_object()->is<Array>()) {
    auto &array = v.as_object()->as<Array>()->inner;
    len = array.size();
    return EFuncStatus::Ok;
  } else
    return EFuncStatus::TypeError;
}
EFuncStatus FunctionContext::get_array_element(size_t pos) {
  CHECK_STACK_UNDERFLOW;
  auto v = *task->stack_top;
  if (v.is_object() && v.as_object()->is<Array>()) {
    auto &array = v.as_object()->as<Array>()->inner;
    if (pos >= array.size())
      return EFuncStatus::Underflow;
    else {
      push(array[pos]);
      return EFuncStatus::Ok;
    }
  } else
    return EFuncStatus::TypeError;
}
}; // namespace neptune_vm