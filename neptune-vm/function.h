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
  std::string name;
  std::vector<uint8_t> bytecode;
  std::vector<Value> constants;
  std::vector<LineInfo> lines;
  uint16_t max_registers;

public:
  friend class FunctionInfoWriter;
  FunctionInfo(StringSlice name_) : name(name_.data, name_.len) {}
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
  uint16_t fun_constant(FunctionInfoWriter f);
  void shrink();
  void pop_last_op(size_t last_op_pos);
  void release();
  void set_max_registers(uint16_t max_registers);
  void patch_jump(size_t op_position, uint32_t jump_offset);
  size_t size() const;
  uint16_t int_constant(int32_t i);
  std::unique_ptr<VMResult> run();
  std::unique_ptr<std::string> to_cxx_string() const;
};
} // namespace neptune_vm
