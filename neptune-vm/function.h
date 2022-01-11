#pragma once
#include "object.h"
#include "op.h"
#include <cstdint>
#include <memory>
#include <vector>

namespace neptune_vm {
template <typename O> class Handle;
class VM;
enum class VMStatus : uint8_t { Success, Error };

struct LineInfo {
  uint32_t offset;
  uint32_t line;
};

struct UpvalueInfo {
  uint16_t index;
  bool is_local;
};

struct ExceptionHandler {
  uint32_t try_begin;
  uint32_t try_end;
  uint16_t error_reg; // used for closing upvalues of try block and storing
                      // exception
  uint32_t catch_begin;
};

class FunctionInfo : public Object {
public:
  static constexpr Type type = Type::FunctionInfo;
  std::string module;
  std::string name;
  std::vector<uint8_t> bytecode;
  std::vector<Value> constants;
  std::vector<LineInfo> lines;
  uint16_t max_registers;
  uint8_t arity;
  std::vector<UpvalueInfo> upvalues;
  std::vector<ExceptionHandler> exception_handlers;
  FunctionInfo(StringSlice module, StringSlice name, uint8_t arity)
      : module(module.data, module.len), name(name.data, name.len),
        arity(arity) {}
};

class FunctionInfoWriter {
  Handle<FunctionInfo> *hf;
  VM *vm;
  std::unique_ptr<ValueMap<uint16_t>> constants;

public:
  using IsRelocatable = std::true_type;

  explicit FunctionInfoWriter(Handle<FunctionInfo> *hf_, const VM *vm_)
      : hf(hf_), vm(const_cast<VM *>(vm_)),
        constants(std::unique_ptr<ValueMap<uint16_t>>(
            new ValueMap<uint16_t>(16, Value(nullptr)))) {}
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
  uint16_t class_constant(StringSlice s);
  void add_method(uint16_t class_, StringSlice name, FunctionInfoWriter f);
  void shrink();
  void pop_last_op(size_t last_op_pos);
  void release();
  void set_max_registers(uint16_t max_registers);
  void patch_jump(size_t op_position, uint32_t jump_offset);
  size_t size() const;
  uint16_t int_constant(int32_t i);
  uint16_t reserve_constant();
  VMStatus run();
  void add_upvalue(uint16_t index, bool is_local);
  void add_exception_handler(uint32_t try_begin, uint32_t try_end,
                             uint16_t error_reg, uint32_t catch_begin);
  friend struct EFuncContext;
};

struct UpValue : public Object {
  Value *location;
  UpValue *next;
  Value closed;
  static constexpr Type type = Type::UpValue;

  UpValue(Value *v = nullptr)
      : location(v), next(nullptr), closed(Value::null()) {}
};

class Function : public Object {
public:
  static constexpr Type type = Type::Function;
  FunctionInfo *function_info;
  uint16_t num_upvalues;
  Class *super_class;
  UpValue *upvalues[];
  Function() = delete;
};
void disassemble(std::ostream &os, const FunctionInfo &f);
} // namespace neptune_vm
