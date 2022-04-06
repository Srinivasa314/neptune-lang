#pragma once
#include "object.h"

namespace neptune_vm {
class VM;

using NativeFunctionCallback = VMStatus(VM *vm, Value *args);

class NativeFunction : public Object {
  uint8_t arity;
  NativeFunctionCallback *inner;

public:
  NativeFunction(NativeFunctionCallback *function, std::string name,
                 std::string module_name, uint8_t arity)
      : arity(arity), inner(function), name(name), module_name(module_name) {}
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
class TaskHandle;
struct EFuncContext {
  VM *vm;
  Task *task;
  Value *arg;
  EFuncContext(VM *vm, Value *arg, Task *task);
  void push(Value v);
  void push_int(int32_t i);
  void push_float(double d);
  void push_bool(bool b);
  void push_null();
  void push_string(StringSlice s);
  void push_symbol(StringSlice s);
  void push_empty_array();
  EFuncStatus push_to_array();
  void push_empty_object();
  void push_empty_map();
  EFuncStatus push_error(StringSlice module, StringSlice error_class,
                         StringSlice message);
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
  void push_function(FunctionInfoWriter fw);
  bool pop();
  Value pop_value();
  Value peek() const;
  const VM& get_vm() const{return *vm;}
};

using Data = void; // Can be any type
using EFuncCallback = VMStatus(EFuncContext cx, Data *data);
using FreeDataCallback = void(Data *data);
struct EFunc {
  EFuncCallback *callback;
  Data *data;
  FreeDataCallback *free_data;
};

}; // namespace neptune_vm
