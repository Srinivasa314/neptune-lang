#pragma once
#include "object.h"
#include "op.h"
#include <cstdint>
#include <memory>
#include <vector>

namespace neptune_vm {
template <typename O> class Handle;
class VM;
class VMResult;

struct LineInfo {
  uint32_t offset;
  uint32_t line;
};

class FunctionInfo : public Object {
public:
  static constexpr Type type = Type::FunctionInfo;
  std::vector<uint8_t> bytecode;
  std::vector<Value> constants;
  std::vector<LineInfo> lines;
  uint16_t max_registers;

  friend class FunctionInfoWriter;
  friend std::ostream &operator<<(std::ostream &os, const FunctionInfo &f);
};

class FunctionInfoWriter {
  Handle<FunctionInfo> *hf;
  VM *vm;

public:
  explicit FunctionInfoWriter(Handle<FunctionInfo> *hf_, const VM *vm_)
      : hf(hf_), vm(const_cast<VM *>(vm_)) {}
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
  void release();
  void set_max_registers(uint16_t max_registers);
  std::unique_ptr<VMResult> run();
  std::unique_ptr<std::string> to_cxx_string() const;
};
} // namespace neptune_vm
