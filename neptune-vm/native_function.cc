#include "neptune-vm.h"

namespace neptune_vm {
NativeFunctionStatus FunctionContext::return_value(uint16_t slot) {
  if (slot < max_slots) {
    vm->return_value = slots[slot];
    return NativeFunctionStatus::Ok;
  } else {
    return NativeFunctionStatus::InvalidSlotError;
  }
}
}; // namespace neptune_vm