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
  std::string name;
  bool (*inner)(FunctionContext ctx);
  friend class VM;
};
}; // namespace neptune_vm
