#pragma once

#define OPS                                                                    \
  OP(Wide)                                                                     \
  OP(ExtraWide)                                                                \
  OP(LoadRegister)                                                             \
  OP(LoadR0)                                                                   \
  OP(LoadR1)                                                                   \
  OP(LoadR2)                                                                   \
  OP(LoadR3)                                                                   \
  OP(LoadR4)                                                                   \
  OP(LoadR5)                                                                   \
  OP(LoadR6)                                                                   \
  OP(LoadR7)                                                                   \
  OP(LoadR8)                                                                   \
  OP(LoadR9)                                                                   \
  OP(LoadR10)                                                                  \
  OP(LoadR11)                                                                  \
  OP(LoadR12)                                                                  \
  OP(LoadR13)                                                                  \
  OP(LoadR14)                                                                  \
  OP(LoadR15)                                                                  \
  OP(LoadSmallInt)                                                                  \
  OP(LoadNull)                                                                 \
  OP(LoadTrue)                                                                 \
  OP(LoadFalse)                                                                \
  OP(LoadConstant)                                                             \
  OP(StoreRegister)                                                            \
  OP(StoreR0)                                                                  \
  OP(StoreR1)                                                                  \
  OP(StoreR2)                                                                  \
  OP(StoreR3)                                                                  \
  OP(StoreR4)                                                                  \
  OP(StoreR5)                                                                  \
  OP(StoreR6)                                                                  \
  OP(StoreR7)                                                                  \
  OP(StoreR8)                                                                  \
  OP(StoreR9)                                                                  \
  OP(StoreR10)                                                                 \
  OP(StoreR11)                                                                 \
  OP(StoreR12)                                                                 \
  OP(StoreR13)                                                                 \
  OP(StoreR14)                                                                 \
  OP(StoreR15)                                                                 \
  OP(Move)                                                                     \
  OP(LoadGlobal)                                                               \
  OP(StoreGlobal)                                                              \
  OP(LoadSubscript)                                                            \
  OP(StoreArrayUnchecked)                                                      \
  OP(StoreSubscript)                                                           \
  OP(AddRegister)                                                              \
  OP(SubtractRegister)                                                         \
  OP(MultiplyRegister)                                                         \
  OP(DivideRegister)                                                           \
  OP(ConcatRegister)                                                           \
  OP(AddInt)                                                                   \
  OP(SubtractInt)                                                              \
  OP(MultiplyInt)                                                              \
  OP(DivideInt)                                                                \
  OP(Negate)                                                                   \
  OP(Equal)                                                                    \
  OP(NotEqual)                                                                 \
  OP(StrictEqual)                                                              \
  OP(StrictNotEqual)                                                           \
  OP(GreaterThan)                                                              \
  OP(LesserThan)                                                               \
  OP(GreaterThanOrEqual)                                                       \
  OP(LesserThanOrEqual)                                                        \
  OP(Call)                                                                     \
  OP(Call0Argument)                                                            \
  OP(Call1Argument)                                                            \
  OP(Call2Argument)                                                            \
  OP(ToString)                                                                 \
  OP(NewArray)                                                                 \
  OP(NewMap)                                                                   \
  OP(EmptyArray)                                                               \
  OP(EmptyMap)                                                                 \
  OP(Jump)                                                                     \
  OP(JumpIfFalseOrNull)                                                              \
  OP(JumpBack)                                                                 \
  OP(JumpConstant)                                                             \
  OP(JumpIfFalseOrNullConstant)                                                      \
  OP(Return)                                                                   \
  OP(Exit)

#define OP(x) x,
namespace neptune_vm {
enum class Op : uint8_t { OPS };
#undef OP
} // namespace neptune_vm
