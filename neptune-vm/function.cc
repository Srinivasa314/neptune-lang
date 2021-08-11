#include "neptune-vm.h"
#include <algorithm>
#include <stdexcept>

namespace neptune_vm {
template <typename T> void FunctionInfo::write(T t) {
  bytecode.insert(std::end(bytecode), reinterpret_cast<uint8_t *>(&t),
                  reinterpret_cast<uint8_t *>(&t) + sizeof(t));
}

void FunctionInfo::write_op(Op op, uint32_t line) { write(op); }
void FunctionInfo::write_u8(uint8_t u) { write(u); }
void FunctionInfo::write_u16(uint16_t u) { write(u); }
void FunctionInfo::write_u32(uint32_t u) { write(u); }
void FunctionInfo::write_i8(int8_t i) { write(i); }
void FunctionInfo::write_i16(int16_t i) { write(i); }
void FunctionInfo::write_i32(int32_t i) { write(i); }

constexpr size_t MAX_CONSTANTS = 65535;

uint16_t FunctionInfo::constant(Value v) {
  if (constants.size() == MAX_CONSTANTS) {
    throw std::overflow_error("Cannot store more than 65535 constants");
  } else {
    auto pos = std::find(constants.begin(), constants.end(), v);
    if (pos != constants.end()) {
      return static_cast<uint16_t>(pos - constants.begin());
    } else {
      constants.push_back(v);
      return constants.size() - 1;
    }
  }
}

uint16_t FunctionInfo::float_constant(double d) {
  return constant(Value{d});
}
uint16_t FunctionInfo::string_constant(StringSlice s) {
}
uint16_t FunctionInfo::symbol_constant(StringSlice s) {}
void FunctionInfo::shrink() {
  bytecode.shrink_to_fit();
  constants.shrink_to_fit();
  lines.shrink_to_fit();
}
void FunctionInfo::shrink_to(size_t size) { bytecode.resize(size); }

} // namespace neptune_vm
