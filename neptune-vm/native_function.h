#pragma once
#include "object.h"

namespace neptune_vm {
class VM;
struct FunctionContext {
  VM *vm;
  Value *slots;
  uint16_t max_slots;
};
class NativeFunction : public Object {
  uint8_t arity;
  uint16_t max_slots;
  bool (*inner)(FunctionContext ctx, void *data);
  void *data;
  void (*free_data)(void *data);

public:
  static constexpr Type type = Type::NativeFunction;
  std::string name;
  friend class VM;
};
}; // namespace neptune_vm
