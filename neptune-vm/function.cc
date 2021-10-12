#include "neptune-vm.h"
#include <algorithm>
#include <memory>
#include <ostream>
#include <sstream>

namespace neptune_vm {
template <typename T> void FunctionInfoWriter::write(T t) {
  hf->object->bytecode.insert(std::end(hf->object->bytecode),
                              reinterpret_cast<uint8_t *>(&t),
                              reinterpret_cast<uint8_t *>(&t) + sizeof(t));
}

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
    auto pos = constants->find(v);
    if (pos != constants->end()) {
      return pos->second;
    } else {
      hf->object->constants.push_back(v);
      uint16_t pos = hf->object->constants.size() - 1;
      (*constants)[v] = pos;
      return pos;
    }
  }
}

uint16_t FunctionInfoWriter::reserve_constant() {
  if (hf->object->constants.size() == MAX_CONSTANTS) {
    throw std::overflow_error("Cannot store more than 65535 constants");
  } else {
    hf->object->constants.push_back(Value::null());
    return static_cast<uint16_t>(hf->object->constants.size() - 1);
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

uint16_t FunctionInfoWriter::fun_constant(FunctionInfoWriter f) {
  auto c = constant(Value{static_cast<Object *>(f.hf->object)});
  f.release();
  return c;
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

void FunctionInfoWriter::release() {
  vm->release(hf);
  constants.reset();
}

std::unique_ptr<VMResult> FunctionInfoWriter::run(bool eval) {
  return std::unique_ptr<VMResult>(new VMResult(vm->run(hf->object, eval)));
}

void FunctionInfoWriter::set_max_registers(uint16_t max_registers) {
  hf->object->max_registers = max_registers;
}

static void assert_in_range(size_t index, size_t len) {
  if (index >= len)
    throw std::overflow_error("Index out of bounds");
}

void FunctionInfoWriter::patch_jump(size_t op_position, uint32_t jump_offset) {
  constexpr uint8_t PATCH_OFFSET =
      static_cast<uint8_t>(Op::JumpConstant) - static_cast<uint8_t>(Op::Jump);
  auto len = hf->object->bytecode.size();
  auto bytecode = hf->object->bytecode.data();
  assert_in_range(op_position, len);
  if (bytecode[op_position] == static_cast<uint8_t>(Op::Wide)) {
    assert_in_range(op_position + 3, len);
    if (jump_offset < 65536) {
      bytecode[op_position + 1] -= PATCH_OFFSET;
      write_unaligned<uint16_t>(bytecode + op_position + 2,
                                static_cast<uint16_t>(jump_offset));
    } else {
      hf->object
          ->constants[read_unaligned<uint16_t>(bytecode + op_position + 2)] =
          Value(static_cast<int32_t>(jump_offset));
    }
  } else {
    assert_in_range(op_position + 1, len);
    if (jump_offset < 256) {
      bytecode[op_position] -= PATCH_OFFSET;
      bytecode[op_position + 1] = static_cast<uint8_t>(jump_offset);
    } else {
      hf->object->constants[bytecode[op_position + 1]] =
          Value(static_cast<int32_t>(jump_offset));
    }
  }
}

size_t FunctionInfoWriter::size() const { return hf->object->bytecode.size(); }

uint16_t FunctionInfoWriter::int_constant(int32_t i) {
  return constant(Value(i));
}

#define CASE(x)                                                                \
  case Op::x:                                                                  \
    os << #x " "

#define REG(type) "r" << READ(type)

namespace numerical_chars {
static std::ostream &operator<<(std::ostream &os, int8_t i) {
  return os << static_cast<int>(i);
}

static std::ostream &operator<<(std::ostream &os, uint8_t i) {
  return os << static_cast<unsigned int>(i);
}
} // namespace numerical_chars

#define READ(type) checked_read<type>(ip, end)
static void disassemble(std::ostream &os, const FunctionInfo &f) {
  using namespace numerical_chars;
  os << "Bytecode for " << f.name << '\n';
  auto ip = f.bytecode.data();
  auto end = f.bytecode.data() + f.bytecode.size();
  auto curr_line = f.lines.begin();
  while (ip != end) {
    if (curr_line != f.lines.end() &&
        ip - f.bytecode.data() == curr_line->offset) {
      os << curr_line->line << "> ";
      curr_line++;
    }
    os << ip - f.bytecode.data() << ' ';
    switch (READ(Op)) {
    case Op::Wide: {
      os << "Wide ";
      switch (READ(Op)) {
        CASE(LoadRegister) << REG(uint16_t);
        break;

        CASE(LoadConstant) << f.constants[READ(uint16_t)];
        break;

        CASE(StoreRegister) << REG(uint16_t);
        break;

        CASE(Move) << REG(uint16_t) << ' ' << REG(uint16_t);
        break;

        CASE(LoadGlobal) << READ(uint16_t);
        break;
        CASE(StoreGlobal) << READ(uint16_t);
        break;

        CASE(AddRegister) << REG(uint16_t);
        break;
        CASE(SubtractRegister) << REG(uint16_t);
        break;
        CASE(MultiplyRegister) << REG(uint16_t);
        break;
        CASE(DivideRegister) << REG(uint16_t);
        break;
        CASE(ModRegister) << REG(uint16_t);
        break;
        CASE(ConcatRegister) << REG(uint16_t);
        break;

        CASE(AddInt) << READ(int16_t);
        break;
        CASE(SubtractInt) << READ(int16_t);
        break;
        CASE(MultiplyInt) << READ(int16_t);
        break;
        CASE(DivideInt) << READ(int16_t);
        break;
        CASE(ModInt) << READ(int16_t);
        break;

        CASE(Equal) << REG(uint16_t);
        break;
        CASE(NotEqual) << REG(uint16_t);
        break;
        CASE(StrictEqual) << REG(uint16_t);
        break;
        CASE(StrictNotEqual) << REG(uint16_t);
        break;
        CASE(GreaterThan) << REG(uint16_t);
        break;
        CASE(LesserThan) << REG(uint16_t);
        break;
        CASE(GreaterThanOrEqual) << REG(uint16_t);
        break;
        CASE(LesserThanOrEqual) << REG(uint16_t);
        break;

        CASE(Call) << REG(uint16_t) << ' ' << READ(uint8_t);
        break;

        CASE(NewArray) << READ(uint16_t) << ' ' << REG(uint16_t);
        break;
        CASE(StoreSubscript) << REG(uint16_t) << ' ' << REG(uint16_t);
        break;
        CASE(StoreArrayUnchecked) << REG(uint16_t) << ' ' << READ(uint16_t);
        break;
        CASE(LoadSubscript) << REG(uint16_t);
        break;
        CASE(NewMap) << READ(uint16_t) << ' ' << REG(uint16_t);
        break;

        CASE(Jump) << READ(uint16_t);
        break;
        CASE(JumpIfFalseOrNull) << READ(uint16_t);
        break;
        CASE(JumpIfNotFalseOrNull) << READ(uint16_t);
        break;
        CASE(JumpConstant) << f.constants[READ(uint16_t)];
        break;
        CASE(JumpIfFalseOrNullConstant) << f.constants[READ(uint16_t)];
        break;
        CASE(JumpIfNotFalseOrNullConstant) << f.constants[READ(uint16_t)];
        break;
        CASE(JumpBack) << READ(uint16_t);
        break;
        CASE(BeginForLoop) << READ(uint16_t) << ' ' << REG(uint16_t);
        break;
        CASE(BeginForLoopConstant)
            << f.constants[READ(uint16_t)] << ' ' << REG(uint16_t);
        break;
        CASE(ForLoop) << READ(uint16_t) << ' ' << REG(uint16_t);
        break;
        CASE(Call0Argument) << REG(uint16_t);
        break;
        CASE(Call1Argument) << REG(uint16_t);
        break;
        CASE(Call2Argument) << REG(uint16_t);
        break;
        CASE(Call3Argument) << REG(uint16_t);
        break;

      default:
        os << "An op that doesnt have a wide variant is here!";
      }
    } break;

    case Op::ExtraWide: {
      os << "ExtraWide ";
      switch (READ(Op)) {
        CASE(LoadGlobal) << READ(uint32_t);
        break;
        CASE(StoreGlobal) << READ(uint32_t);
        break;
        CASE(AddInt) << READ(int32_t);
        break;
        CASE(SubtractInt) << READ(int32_t);
        break;
        CASE(MultiplyInt) << READ(int32_t);
        break;
        CASE(DivideInt) << READ(int32_t);
        break;
        CASE(ModInt) << READ(int32_t);
        break;
        CASE(JumpBack) << READ(uint32_t);
        break;
        CASE(ForLoop) << READ(uint32_t) << ' ' << REG(uint32_t);
        break;

      default:
        os << "An op that doesnt have an extrawide variant is here!";
      }
    } break;

      CASE(LoadRegister) << REG(uint8_t);
      break;
      CASE(LoadSmallInt) << READ(int8_t);
      break;
      CASE(LoadNull);
      break;
      CASE(LoadTrue);
      break;
      CASE(LoadFalse);
      break;

      CASE(LoadConstant) << f.constants[READ(uint8_t)];
      break;
      CASE(StoreRegister) << REG(uint8_t);
      break;
      CASE(Move) << REG(uint8_t) << ' ' << REG(uint8_t);
      break;
      CASE(LoadGlobal) << READ(uint8_t);
      break;
      CASE(StoreGlobal) << READ(uint8_t);
      break;

      CASE(AddRegister) << REG(uint8_t);
      break;
      CASE(SubtractRegister) << REG(uint8_t);
      break;
      CASE(MultiplyRegister) << REG(uint8_t);
      break;
      CASE(DivideRegister) << REG(uint8_t);
      break;
      CASE(ModRegister) << REG(uint8_t);
      break;
      CASE(ConcatRegister) << REG(uint8_t);
      break;

      CASE(AddInt) << READ(int8_t);
      break;
      CASE(SubtractInt) << READ(int8_t);
      break;
      CASE(MultiplyInt) << READ(int8_t);
      break;
      CASE(DivideInt) << READ(int8_t);
      break;
      CASE(ModInt) << READ(int8_t);
      break;
      CASE(Negate);
      break;
      CASE(Not);
      break;

      CASE(Equal) << REG(uint8_t);
      break;
      CASE(NotEqual) << REG(uint8_t);
      break;
      CASE(StrictEqual) << REG(uint8_t);
      break;
      CASE(StrictNotEqual) << REG(uint8_t);
      break;

      CASE(GreaterThan) << REG(uint8_t);
      break;
      CASE(LesserThan) << REG(uint8_t);
      break;
      CASE(GreaterThanOrEqual) << REG(uint8_t);
      break;
      CASE(LesserThanOrEqual) << REG(uint8_t);
      break;

      CASE(Call) << REG(uint8_t) << ' ' << READ(uint8_t);
      break;

      CASE(ToString);
      break;
      CASE(NewArray) << READ(uint8_t) << ' ' << REG(uint8_t);
      break;
      CASE(StoreSubscript) << REG(uint8_t) << ' ' << REG(uint8_t);
      break;
      CASE(StoreArrayUnchecked) << REG(uint8_t) << ' ' << READ(uint8_t);
      break;
      CASE(LoadSubscript) << REG(uint8_t);
      break;
      CASE(NewMap) << READ(uint8_t) << ' ' << REG(uint8_t);
      break;
      CASE(EmptyArray);
      break;
      CASE(EmptyMap);
      break;
      CASE(Jump) << READ(uint8_t);
      break;
      CASE(JumpIfFalseOrNull) << READ(uint8_t);
      break;
      CASE(JumpIfNotFalseOrNull) << READ(uint8_t);
      break;
      CASE(JumpConstant) << f.constants[READ(uint8_t)];
      break;
      CASE(JumpIfFalseOrNullConstant) << f.constants[READ(uint8_t)];
      break;
      CASE(JumpIfNotFalseOrNullConstant) << f.constants[READ(uint8_t)];
      break;
      CASE(JumpBack) << READ(uint8_t);
      break;
      CASE(BeginForLoop) << READ(uint8_t) << ' ' << REG(uint8_t);
      break;
      CASE(BeginForLoopConstant)
          << f.constants[READ(uint8_t)] << ' ' << REG(uint8_t);
      break;
      CASE(ForLoop) << READ(uint8_t) << ' ' << REG(uint8_t);
      break;
      CASE(Return);
      break;
      CASE(Exit);
      break;

      CASE(LoadR0);
      break;
      CASE(LoadR1);
      break;
      CASE(LoadR2);
      break;
      CASE(LoadR3);
      break;
      CASE(LoadR4);
      break;
      CASE(LoadR5);
      break;
      CASE(LoadR6);
      break;
      CASE(LoadR7);
      break;
      CASE(LoadR8);
      break;
      CASE(LoadR9);
      break;
      CASE(LoadR10);
      break;
      CASE(LoadR11);
      break;
      CASE(LoadR12);
      break;
      CASE(LoadR13);
      break;
      CASE(LoadR14);
      break;
      CASE(LoadR15);
      break;

      CASE(StoreR0);
      break;
      CASE(StoreR1);
      break;
      CASE(StoreR2);
      break;
      CASE(StoreR3);
      break;
      CASE(StoreR4);
      break;
      CASE(StoreR5);
      break;
      CASE(StoreR6);
      break;
      CASE(StoreR7);
      break;
      CASE(StoreR8);
      break;
      CASE(StoreR9);
      break;
      CASE(StoreR10);
      break;
      CASE(StoreR11);
      break;
      CASE(StoreR12);
      break;
      CASE(StoreR13);
      break;
      CASE(StoreR14);
      break;
      CASE(StoreR15);
      break;
      CASE(Call0Argument) << REG(uint8_t);
      break;
      CASE(Call1Argument) << REG(uint8_t);
      break;
      CASE(Call2Argument) << REG(uint8_t);
      break;
      CASE(Call3Argument) << REG(uint8_t);
      break;
    default:
      os << "Invalid op here!";
    }
    os << '\n';
  }
  for (auto i : f.constants) {
    if (i.is_object() && i.as_object()->is<FunctionInfo>()) {
      os << '\n';
      disassemble(os, *i.as_object()->as<FunctionInfo>());
    }
  }
}
#undef CASE
#undef READ
#undef REG

std::unique_ptr<std::string> FunctionInfoWriter::to_cxx_string() const {
  std::ostringstream os;
  disassemble(os, *this->hf->object);
  return std::unique_ptr<std::string>(new std::string(os.str()));
}
} // namespace neptune_vm
