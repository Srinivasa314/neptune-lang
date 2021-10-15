#pragma once
#include "object.h"
#include "op.h"
#include <cstdint>
#include <memory>
#include <vector>

namespace neptune_vm {
template <typename O> class Handle;
class VM;
struct VMResult;

struct LineInfo {
  uint32_t offset;
  uint32_t line;
};

struct UpvalueInfo {
  uint16_t index;
  bool is_local;
};

class FunctionInfo : public Object {
public:
  static constexpr Type type = Type::FunctionInfo;
  std::string name;
  std::vector<uint8_t> bytecode;
  std::vector<Value> constants;
  std::vector<LineInfo> lines;
  uint16_t max_registers;
  uint8_t arity;
  std::vector<UpvalueInfo> upvalues;
  FunctionInfo(StringSlice name_, uint8_t arity_)
      : name(name_.data, name_.len), arity(arity_) {}
};

class FunctionInfoWriter {
  Handle<FunctionInfo> *hf;
  VM *vm;
  std::unique_ptr<ValueMap<uint16_t>> constants;

public:
  using IsRelocatable = std::true_type;

  explicit FunctionInfoWriter(Handle<FunctionInfo> *hf_, const VM *vm_)
      : hf(hf_), vm(const_cast<VM *>(vm_)),
        constants(std::unique_ptr<ValueMap<uint16_t>>(new ValueMap<uint16_t>)) {
  }
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
  uint16_t reserve_constant();
  std::unique_ptr<VMResult> run(bool eval);
  std::unique_ptr<std::string> to_cxx_string() const;
  void add_upvalue(uint16_t index, bool is_local);
};

struct UpValue : public Object {
  Value *location;
  UpValue *next;
  Value closed;
  static constexpr Type type = Type::UpValue;

public:
  UpValue(Value *v = nullptr)
      : location(v), next(nullptr), closed(Value::empty()) {}
};

class Function : public Object {
public:
  static constexpr Type type = Type::Function;
  FunctionInfo *function_info;
  uint16_t num_upvalues;
  UpValue *upvalues[];
  Function(FunctionInfo *f) : function_info(f) {}
};
} // namespace neptune_vm
