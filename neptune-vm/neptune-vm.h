#pragma once
#include <memory>
#include <stdint.h>
#include <vector>

#if (defined(__x86_64__) || defined(_M_X64) || defined(__aarch64__) ||         \
     defined(_M_ARM64))
#define NANBOX
#endif

namespace neptune_vm {
class Object {};

class Value {
#ifdef NANBOX
  uint64_t inner;
  static constexpr uint64_t VALUE_NULL = 1;
  static constexpr uint64_t VALUE_TRUE = 2;
  static constexpr uint64_t VALUE_FALSE = 3;
  Value(uint64_t u) { inner = u; }

public:
  static Value new_true() { return Value(VALUE_TRUE); }

  static Value new_false() { return Value(VALUE_FALSE); }

  static Value null() { return Value(VALUE_NULL); }

  static Value empty() { return Value((uint64_t)0); }

#else
  enum class Tag : uint8_t {
    Empty,
    Int,
    Float,
    Object,
    True,
    False,
    Null,
  };

  Tag tag;
  union {
    int32_t as_int;
    double as_float;
    Object *as_object;
  } value;

  Value(Tag t) { tag = t; }

public:
  static Value new_true() { return Value(Tag::True); }

  static Value new_false() { return Value(Tag::False); }

  static Value null() { return Value(Tag::Null); }

  static Value empty() { return Value(Tag::Empty); }

#endif
public:
  explicit Value(int32_t i);
  explicit Value(double d);
  explicit Value(Object *o);
  explicit Value(bool b);

  bool is_int() const;
  int32_t as_int() const;
  bool is_float() const;
  double as_float() const;
  bool is_null_or_false() const;
  bool is_object() const;
  Object *as_object() const;
  bool is_null() const;
  bool is_empty() const;
};

enum class Op : uint8_t {
  Wide,
  ExtraWide,
  LoadRegister,
  LoadInt,
  LoadNull,
  LoadTrue,
  LoadFalse,
  LoadConstant,
  StoreRegister,
  Move,
  LoadGlobal,
  StoreGlobal,
  AddRegister,
  SubtractRegister,
  MultiplyRegister,
  DivideRegister,
  ConcatRegister,
  AddInt,
  SubtractInt,
  MultiplyInt,
  DivideInt,
  Negate,
  Call,
  Call0Argument,
  Call1Argument,
  Call2Argument,
  Less,
  ToString,
  Jump,
  JumpBack,
  JumpIfFalse,
  Return,
  Exit,
  StoreR0,
  StoreR1,
  StoreR2,
  StoreR3,
  StoreR4,
  StoreR5,
  StoreR6,
  StoreR7,
  StoreR8,
  StoreR9,
  StoreR10,
  StoreR11,
  StoreR12,
  StoreR13,
  StoreR14,
  StoreR15,
};

struct FunctionInfo : Object {
  struct LineInfo {
    uint32_t offset;
    uint32_t line;
  };

  std::vector<uint8_t> bytecode;
  // the lifetime of this is the lifetime of the VM as they are constants
  std::vector<Value> constants;
  std::vector<LineInfo> lines;

public:
  template <typename T> void write(T t);
  void write_op(Op op, uint32_t line);
  void write_u8(uint8_t u);
  void write_u16(uint16_t u);
  void write_u32(uint32_t u);
  void write_i8(int8_t i);
  void write_i16(int16_t i);
  void write_i32(int32_t i);
  uint16_t float_constant();
  uint16_t string_constant(const char *s, size_t len);
  uint16_t symbol_constant(const char *s, size_t len);
  void shrink();
  void shrink_to(size_t);
};
} // namespace neptune_vm
