#include "neptune-vm.h"
#include <algorithm>
#include <memory>
#include <stdexcept>

namespace neptune_vm {
template <typename T> void FunctionInfo::write(T t) {
  bytecode.insert(std::end(bytecode), reinterpret_cast<uint8_t *>(&t),
                  reinterpret_cast<uint8_t *>(&t) + sizeof(t));
}

// todo put proper line info
size_t FunctionInfo::write_op(Op op, uint32_t line) {
  if (lines.empty() || lines.back().line != line)
    lines.push_back(LineInfo{static_cast<uint32_t>(bytecode.size()), line});
  write(op);
  return bytecode.size() - 1;
}
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

uint16_t FunctionInfo::float_constant(double d) { return constant(Value{d}); }

uint16_t FunctionInfo::string_constant(StringSlice s, const VM &vm) {
  String *p = vm.gc.make_constant(String::from_string_slice(s));
  return constant(Value{static_cast<Object *>(p)});
}

uint16_t FunctionInfo::symbol_constant(StringSlice s, const VM &vm) {
  Symbol *p = vm.gc.make_constant(Symbol::from_string_slice(s));
  return constant(Value{static_cast<Object *>(p)});
}

void FunctionInfo::shrink() {
  bytecode.shrink_to_fit();
  constants.shrink_to_fit();
  lines.shrink_to_fit();
}

void FunctionInfo::pop_last_op(size_t last_op_pos) {
  bytecode.resize(last_op_pos);
  if ((!lines.empty()) && lines.back().offset == last_op_pos) {
    lines.pop_back();
  }
}

std::unique_ptr<FunctionInfo> new_function_info() {
  return std::unique_ptr<FunctionInfo>(new FunctionInfo);
}
} // namespace neptune_vm
