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

NativeFunctionStatus FunctionContext::as_string(uint16_t slot,
                                                StringSlice &s) const {
  if (slot < max_slots) {
    if (slots[slot].is_object() && slots[slot].as_object()->is<String>()) {
      s = static_cast<StringSlice>(*slots[slot].as_object()->as<String>());
      return NativeFunctionStatus::Ok;
    } else {
      return NativeFunctionStatus::TypeError;
    }
  } else {
    return NativeFunctionStatus::InvalidSlotError;
  }
}

NativeFunctionStatus FunctionContext::to_string(uint16_t dest,
                                                uint16_t source) {
  if (source < max_slots || dest < max_slots) {
    slots[dest] = vm->to_string(slots[source]);
    return NativeFunctionStatus::Ok;
  } else {
    return NativeFunctionStatus::InvalidSlotError;
  }
}

void FunctionContext::null(uint16_t slot) { slots[slot] = Value::null(); }
}; // namespace neptune_vm