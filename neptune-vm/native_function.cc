#include "neptune-vm.h"

namespace neptune_vm {
NativeFunctionStatus FunctionContext::return_value(uint16_t slot) const {
  if (slot < max_slots) {
    vm->return_value = slots[slot];
    return NativeFunctionStatus::Ok;
  } else {
    return NativeFunctionStatus::InvalidSlotError;
  }
}

NativeFunctionStatus FunctionContext::as_string(uint16_t slot,
                                                rust::String &s) const {
  if (slot < max_slots) {
    if (slots[slot].is_object() && slots[slot].as_object()->is<String>()) {
      s = static_cast<rust::String>(*slots[slot].as_object()->as<String>());
      return NativeFunctionStatus::Ok;
    } else {
      return NativeFunctionStatus::TypeError;
    }
  } else {
    return NativeFunctionStatus::InvalidSlotError;
  }
}

NativeFunctionStatus FunctionContext::to_string(uint16_t dest,
                                                uint16_t source) const {
  if (source < max_slots || dest < max_slots) {
    slots[dest] = vm->to_string(slots[source]);
    return NativeFunctionStatus::Ok;
  } else {
    return NativeFunctionStatus::InvalidSlotError;
  }
}

NativeFunctionStatus FunctionContext::null(uint16_t slot) const {
  if (slot < max_slots) {
    slots[slot] = Value::null();
    return NativeFunctionStatus::Ok;
  } else {
    return NativeFunctionStatus::InvalidSlotError;
  }
}

NativeFunctionStatus FunctionContext::string(uint16_t slot,
                                            StringSlice string) const {
  if (slot < max_slots) {
    slots[slot] = Value(vm->manage(String::from(string)));
    return NativeFunctionStatus::Ok;
  } else {
    return NativeFunctionStatus::InvalidSlotError;
  }
}

NativeFunctionStatus FunctionContext::function(uint16_t slot,
                                               FunctionInfoWriter fw) const {
  if (slot < max_slots) {
    auto function = vm->manage(new Function(fw.hf->object));
    function->num_upvalues = 0;
    slots[slot] = Value(function);
    fw.release();
    return NativeFunctionStatus::Ok;
  } else {
    return NativeFunctionStatus::InvalidSlotError;
  }
}
}; // namespace neptune_vm