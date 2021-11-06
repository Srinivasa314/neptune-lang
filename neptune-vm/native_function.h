#pragma once
#include "object.h"
#include "rust/cxx.h"

namespace neptune_vm {
class VM;

enum class NativeFunctionStatus : uint8_t {
  Ok,
  InvalidSlotError,
  TypeError,
};

struct FunctionContext {
  VM *vm;
  Value *slots;
  uint16_t max_slots;
  NativeFunctionStatus return_value(uint16_t slot) const;
  NativeFunctionStatus as_string(uint16_t slot, rust::String &s) const;
  NativeFunctionStatus to_string(uint16_t dest, uint16_t source) const;
  NativeFunctionStatus null(uint16_t slot) const;
  NativeFunctionStatus function(uint16_t slot, FunctionInfoWriter fw) const;
  NativeFunctionStatus string(uint16_t slot, StringSlice string) const;
};

using Data = void; // Can be anything
using NativeFunctionCallback = bool(FunctionContext ctx, void *data);
using FreeDataCallback = void(Data *data);

class NativeFunction : public Object {
  uint8_t arity;
  uint16_t max_slots;
  NativeFunctionCallback *inner;
  Data *data;
  FreeDataCallback *free_data;

public:
  static constexpr Type type = Type::NativeFunction;
  std::string name;
  std::string module_name;
  friend class VM;
};
}; // namespace neptune_vm
