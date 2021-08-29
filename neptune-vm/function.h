#pragma once
#include "gc.h"
#include "handle.h"
#include "object.h"
#include "op.h"
#include <cstdint>
#include <memory>
#include <vector>

namespace neptune_vm {
class Value;
class VM;

struct LineInfo {
  uint32_t offset;
  uint32_t line;
};

class FunctionInfo : public Object {
public:
  std::vector<uint8_t> bytecode;
  std::vector<Value> constants;
  std::vector<LineInfo> lines;

  friend class FunctionInfoWriter;
};

class FunctionInfoWriter {
  Handle<FunctionInfo> *hf;
  const VM *vm;

public:
  explicit FunctionInfoWriter(Handle<FunctionInfo> *hf_, const VM *vm_)
      : hf(hf_), vm(vm_) {}
  template <typename T> void write(T t);
  uint16_t constant(Value v);
  size_t write_op(Op op, uint32_t line);
  void write_u8(uint8_t u);
  void write_u16(uint16_t u);
  void write_u32(uint32_t u);
  uint16_t float_constant(double d);
  uint16_t string_constant(StringSlice s);
  uint16_t symbol_constant(StringSlice s);
  void shrink();
  void pop_last_op(size_t last_op_pos);
  void release(); // calls the destructor
};
} // namespace neptune_vm
