#pragma once

namespace neptune_vm {
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
  LoadR0,
  LoadR1,
  LoadR2,
  LoadR3,
  LoadR4,
  LoadR5,
  LoadR6,
  LoadR7,
  LoadR8,
  LoadR9,
  LoadR10,
  LoadR11,
  LoadR12,
  LoadR13,
  LoadR14,
  LoadR15,

};
}