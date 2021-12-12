#pragma once
#include "object.h"

namespace neptune_vm {
class VM;

using NativeFunctionCallback = bool(VM *vm, Value *slots);

class NativeFunction : public Object {
  uint8_t arity;
  NativeFunctionCallback *inner;

public:
  NativeFunction() {}
  NativeFunction(NativeFunctionCallback *function, std::string name,
                 uint8_t arity)
      : inner(function), name(name), arity(arity) {}
  static constexpr Type type = Type::NativeFunction;
  std::string name;
  std::string module_name;
  friend class VM;
};

enum class EFuncStatus : uint8_t {
  Ok,
  TypeError,
  Underflow,
  OutOfBoundsError,
  PropertyError
};

class Task;
struct EFuncContext {
  VM *vm;
  Task *task;
  Value *arg;
  EFuncContext(VM *vm, Task *task, Value *arg)
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
  void push_empty_map();
  EFuncStatus set_object_property(StringSlice s);
  EFuncStatus insert_in_map();
  EFuncStatus as_int(int32_t &i);
  EFuncStatus as_float(double &d);
  EFuncStatus as_bool(bool &b);
  EFuncStatus is_null();
  EFuncStatus as_string(StringSlice &s);
  EFuncStatus as_symbol(StringSlice &s);
  EFuncStatus get_array_length(size_t &len) const;
  EFuncStatus get_array_element(size_t pos);
  EFuncStatus get_object_property(StringSlice prop);
  bool pop();
  Value pop_value();
  Value peek() const;
};

using Data = void; // Can be any type
using EFuncCallback = bool(EFuncContext cx, Data *data);
using FreeDataCallback = void(Data *data);
struct EFunc {
  EFuncCallback *callback;
  Data *data;
  FreeDataCallback *free_data;
};
}; // namespace neptune_vm
