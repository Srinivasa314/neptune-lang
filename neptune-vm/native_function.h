#pragma once
#include "object.h"
#include "rust/cxx.h"

namespace neptune_vm {
class VM;

using NativeFunctionCallback = bool(VM *vm, Value *slots);

class NativeFunction : public Object {
  uint8_t arity;
  NativeFunctionCallback *inner;

public:
  static constexpr Type type = Type::NativeFunction;
  std::string name;
  std::string module_name;
  friend class VM;
};

enum class EFuncStatus : uint8_t {
  Ok,
  TypeError,
  Underflow,
  OutOfBounds,
  PropertyError
};

class Task;
struct FunctionContext {
  VM *vm;
  Task *task;
  Value *arg;
  FunctionContext(VM *vm, Task *task, Value *arg)
      : vm(vm), task(task), arg(arg) {}
  void push(Value v);
  void push_int(int32_t i);
  void push_float(double f);
  void push_bool(bool b);
  void push_null();
  void push_string(StringSlice s);
  void push_symbol(StringSlice s);
  void push_empty_array();
  EFuncStatus push_to_array();
  void push_empty_object();
  EFuncStatus set_object_property(StringSlice s);
  EFuncStatus as_int(int32_t &i);
  EFuncStatus as_float(double &d);
  EFuncStatus as_bool(bool &b);
  EFuncStatus is_null();
  EFuncStatus as_string(rust::String &s);
  EFuncStatus as_symbol(rust::String &s);
  EFuncStatus get_array_length(size_t &len);
  EFuncStatus get_array_element(size_t pos);
  EFuncStatus is_object();
  EFuncStatus get_object_property(StringSlice prop);
  bool pop();
  Value pop_value();
  Value peek();
};

using EFuncCallback = bool (*)(FunctionContext *ctx, void *data);
using FreeDataCallback = void (*)(void *data);
struct EFunc {
  EFuncCallback callback;
  void *data;
  FreeDataCallback free_data;
};
}; // namespace neptune_vm
