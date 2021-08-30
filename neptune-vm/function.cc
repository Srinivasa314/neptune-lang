#include "neptune-vm.h"
#include <algorithm>
#include <memory>
#include <stdexcept>

namespace neptune_vm {
template <typename T> void FunctionInfoWriter::write(T t) {
  hf->object->bytecode.insert(std::end(hf->object->bytecode),
                              reinterpret_cast<uint8_t *>(&t),
                              reinterpret_cast<uint8_t *>(&t) + sizeof(t));
}

// todo put proper line info
size_t FunctionInfoWriter::write_op(Op op, uint32_t line) {
  if (hf->object->lines.empty() || hf->object->lines.back().line != line)
    hf->object->lines.push_back(
        LineInfo{static_cast<uint32_t>(hf->object->bytecode.size()), line});
  write(op);
  return hf->object->bytecode.size() - 1;
}
void FunctionInfoWriter::write_u8(uint8_t u) { write(u); }
void FunctionInfoWriter::write_u16(uint16_t u) { write(u); }
void FunctionInfoWriter::write_u32(uint32_t u) { write(u); }

constexpr size_t MAX_CONSTANTS = 65535;

uint16_t FunctionInfoWriter::constant(Value v) {
  if (hf->object->constants.size() == MAX_CONSTANTS) {
    throw std::overflow_error("Cannot store more than 65535 constants");
  } else {
    auto pos = std::find(hf->object->constants.begin(),
                         hf->object->constants.end(), v);
    if (pos != hf->object->constants.end()) {
      return static_cast<uint16_t>(pos - hf->object->constants.begin());
    } else {
      hf->object->constants.push_back(v);
      return static_cast<uint16_t>(hf->object->constants.size() - 1);
    }
  }
}

uint16_t FunctionInfoWriter::float_constant(double d) {
  return constant(Value{d});
}

uint16_t FunctionInfoWriter::string_constant(StringSlice s) {
  String *p = vm->manage(String::from_string_slice(s));
  return constant(Value{static_cast<Object *>(p)});
}

uint16_t FunctionInfoWriter::symbol_constant(StringSlice s) {
  Symbol *p = vm->intern(s);
  return constant(Value{static_cast<Object *>(p)});
}

void FunctionInfoWriter::shrink() {
  hf->object->bytecode.shrink_to_fit();
  hf->object->constants.shrink_to_fit();
  hf->object->lines.shrink_to_fit();
}

void FunctionInfoWriter::pop_last_op(size_t last_op_pos) {
  hf->object->bytecode.resize(last_op_pos);
  if ((!hf->object->lines.empty()) &&
      hf->object->lines.back().offset == last_op_pos) {
    hf->object->lines.pop_back();
  }
}

void FunctionInfoWriter::release() { vm->release(hf); }

VMResult FunctionInfoWriter::run() { return vm->run(hf->object); }
} // namespace neptune_vm
