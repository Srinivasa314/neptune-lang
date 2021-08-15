#pragma once
#include "gc.h"
#include "object.h"
#include "op.h"
#include <cstdint>
#include <memory>
#include <vector>

namespace neptune_vm {
class Value;
class VM;

class FunctionInfo : public Object {
  struct LineInfo {
    uint32_t offset;
    uint32_t line;
  };

  std::vector<uint8_t> bytecode;
  // they live as long as the VM as they are constants
  std::vector<Value> constants;
  std::vector<LineInfo> lines;

  uint16_t constant(Value v);

public:
  template <typename T> void write(T t);
  size_t write_op(Op op, uint32_t line);
  void write_u8(uint8_t u);
  void write_u16(uint16_t u);
  void write_u32(uint32_t u);
  void write_i8(int8_t i);
  void write_i16(int16_t i);
  void write_i32(int32_t i);
  uint16_t float_constant(double d);
  uint16_t string_constant(StringSlice s, const VM &vm);
  uint16_t symbol_constant(StringSlice s, const VM &vm);
  void shrink();
  void pop_last_op(size_t last_op_pos);
};
class FunctionInfoHandle {
  Handle<FunctionInfo *> *inner;

public:
  FunctionInfoHandle(Handle<FunctionInfo *> *inner) : inner(inner) {}
  size_t write_op(Op op, uint32_t line) const {
    return inner->object->write_op(op, line);
  }
  void write_u8(uint8_t u) const { return inner->object->write_u8(u); }
  void write_u16(uint16_t u) const { return inner->object->write_u16(u); }
  void write_u32(uint32_t u) const { return inner->object->write_u32(u); }
  void write_i8(int8_t i) const { return inner->object->write_i8(i); }
  void write_i16(int16_t i) const { return inner->object->write_i16(i); }
  void write_i32(int32_t i) const { return inner->object->write_i32(i); }
  uint16_t float_constant(double d) const {
    return inner->object->float_constant(d);
  }
  uint16_t string_constant(StringSlice s, const VM &vm) const {
    return inner->object->string_constant(s, vm);
  }
  uint16_t symbol_constant(StringSlice s, const VM &vm) const {
    return inner->object->symbol_constant(s, vm);
  }
  void shrink() const { return inner->object->shrink(); }
  void pop_last_op(size_t last_op_pos) const {
    return inner->object->pop_last_op(last_op_pos);
  }
};
} // namespace neptune_vm
