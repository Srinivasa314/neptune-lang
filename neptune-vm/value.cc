#include "neptune-vm.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ASSERT(x)                                                              \
  do {                                                                         \
    if (!(x)) {                                                                \
      puts("Assertion " #x " failed");                                         \
      abort();                                                                 \
    }                                                                          \
  } while (0)

namespace neptune_vm {
#ifdef NANBOX

Value::Value(int32_t i) { inner = (1llu << 48) | static_cast<uint32_t>(i); }

Value::Value(double d) {
  uint64_t u;
  memcpy(&u, &d, sizeof(u));
  inner = u + (2llu << 48);
}

Value::Value(Object *o) { inner = (uint64_t)o; }

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

bool Value::is_float() const { return inner >= (2llu << 48); }

double Value::as_float() const {
  ASSERT(is_float());
  double d;
  uint64_t u = inner - (2llu << 48);
  memcpy(&d, &u, sizeof(u));
  return d;
}

bool Value::is_null_or_false() const {
  return (inner == VALUE_NULL) || (inner == VALUE_FALSE);
}

bool Value::is_object() const {
  return ((inner >> 48) == 0) && inner > VALUE_FALSE;
}

Object *Value::as_object() const {
  ASSERT(is_object());
  return reinterpret_cast<Object *>(inner);
}

bool Value::is_null() const { return inner == VALUE_NULL; }

bool Value::is_empty() const { return inner == 0; }

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
#endif
} // namespace neptune_vm

#undef ASSERT