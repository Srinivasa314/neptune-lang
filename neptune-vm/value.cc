#include "neptune-vm.h"
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#define ASSERT(x)                                                              \
  do {                                                                         \
    if (!(x)) {                                                                \
      puts("Assertion " #x " failed");                                         \
      abort();                                                                 \
    }                                                                          \
  } while (0)

namespace neptune_vm {
#ifdef NANBOX

/*
  On x86_64 and aarch64 the following scheme is used to represent values.

  Empty   0x0000 0000 0000 0000
  Null    0x0000 0000 0000 0001
  True    0x0000 0000 0000 0002
  False   0x0000 0000 0000 0003
  Pointer 0x0000 XXXX XXXX XXXX [due to pointer alignment we can use the last 2
  bits] Integer 0x0001 0000 XXXX XXXX 0x0002 0000 0000 0000 Double to 0xFFFA
  0000 0000 0000

  Doubles lie from 0x0000000000000000 to 0xFFF8000000000000. On adding 2<<48
  they lie in the range listed above.
*/

constexpr uint64_t INT_ENCODING_OFFSET = (1llu << 48);
constexpr uint64_t DOUBLE_ENCODING_OFFSET = (2llu << 48);

Value::Value(int32_t i) {
  inner = INT_ENCODING_OFFSET | static_cast<uint32_t>(i);
}

Value::Value(double d) {
  uint64_t u;
  memcpy(&u, &d, sizeof(u));
  inner = u + DOUBLE_ENCODING_OFFSET;
}

Value::Value(Object *o) { inner = reinterpret_cast<uint64_t>(o); }

Value::Value(bool b) {
  if (b) {
    inner = VALUE_TRUE;
  } else {
    inner = VALUE_FALSE;
  }
}

bool Value::is_int() const { return (inner >> 48) == 1llu; }

int32_t Value::as_int() const {
  ASSERT(is_int());
  return static_cast<int32_t>(inner);
}

bool Value::is_float() const { return inner >= DOUBLE_ENCODING_OFFSET; }

double Value::as_float() const {
  ASSERT(is_float());
  double d;
  uint64_t u = inner - DOUBLE_ENCODING_OFFSET;
  memcpy(&d, &u, sizeof(u));
  return d;
}

bool Value::is_null_or_false() const {
  return (inner == VALUE_NULL) || (inner == VALUE_FALSE);
}

bool Value::is_object() const {
  return ((inner >> 48) == 0) && ((inner % 4) == 0);
}

Object *Value::as_object() const {
  ASSERT(is_object());
  return reinterpret_cast<Object *>(inner);
}

bool Value::is_null() const { return inner == VALUE_NULL; }

bool Value::is_empty() const { return inner == 0; }

bool Value::operator==(Value rhs) const {
  // todo
  return true;
}

#else
Value::Value(int32_t i) {
  tag = Tag::Int;
  value.as_int = i;
}

Value::Value(double d) {
  tag = Tag::Float;
  value.as_float = d;
}

Value::Value(Object *o) {
  tag = Tag::Object;
  value.as_object = o;
}

Value::Value(bool b) {
  if (b) {
    tag = Tag::True;
  } else {
    tag = Tag::False;
  }
}

bool Value::is_int() const { return tag == Tag::Int; }

int32_t Value::as_int() const {
  ASSERT(is_int());
  return value.as_int;
}

bool Value::is_float() const { return tag == Tag::Float; }

double Value::as_float() const {
  ASSERT(is_float());
  return value.as_float;
}

bool Value::is_null_or_false() const {
  return (tag == Tag::Null) || (tag == Tag::False);
}

bool Value::is_object() const { return tag == Tag::Object; }

Object *Value::as_object() const {
  ASSERT(is_object());
  return value.as_object;
}

bool Value::is_null() const { return tag == Tag::Null; }

bool Value::is_empty() const { return tag == Tag::Empty; }

bool Value::operator==(Value rhs) {
  // todo
  return true;
}

#endif
} // namespace neptune_vm

#undef ASSERT
