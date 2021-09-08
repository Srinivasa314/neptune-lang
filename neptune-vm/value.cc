#include "neptune-vm.h"
#include <cstring>
#include <iomanip>
#include <ostream>

namespace neptune_vm {
#ifdef NANBOX

/*
  On x86_64 and aarch64 the following scheme is used to represent values.

  Empty   0x0000 0000 0000 0000
  Null    0x0000 0000 0000 0001
  True    0x0000 0000 0000 0002
  False   0x0000 0000 0000 0003
  Pointer 0x0000 XXXX XXXX XXXX [due to alignment we can use the last 2bits]
  Integer 0x0001 0000 XXXX XXXX
  Double  0x0002 0000 0000 0000
                  to
          0xFFFA 0000 0000 0000

  Doubles lie from 0x0000000000000000 to 0xFFF8000000000000. On adding 2<<48
  they lie in the range listed above.
*/

constexpr uint64_t INT_ENCODING_OFFSET = (1llu << 48);
constexpr uint64_t DOUBLE_ENCODING_OFFSET = (2llu << 48);
constexpr uint64_t OBJECT_MASK = 0xFFFF000000000003llu;

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
  assert(is_int());
  return static_cast<int32_t>(inner);
}

bool Value::is_float() const { return inner >= DOUBLE_ENCODING_OFFSET; }

double Value::as_float() const {
  assert(is_float());
  double d;
  uint64_t u = inner - DOUBLE_ENCODING_OFFSET;
  memcpy(&d, &u, sizeof(u));
  return d;
}

bool Value::is_null_or_false() const {
  return (inner == VALUE_NULL) || (inner == VALUE_FALSE);
}

bool Value::is_object() const {
  // return ((inner >> 48) == 0) && ((inner % 4) == 0);
  return !(inner & OBJECT_MASK);
}

Object *Value::as_object() const {
  assert(is_object());
  return reinterpret_cast<Object *>(inner);
}

bool Value::is_null() const { return inner == VALUE_NULL; }

bool Value::is_empty() const { return inner == 0; }

bool Value::is_bool() const {
  return inner == VALUE_TRUE || inner == VALUE_FALSE;
}

bool Value::is_true() const { return inner == VALUE_TRUE; }

bool Value::is_false() const { return inner == VALUE_FALSE; }

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
  assert(is_int());
  return value.as_int;
}

bool Value::is_float() const { return tag == Tag::Float; }

double Value::as_float() const {
  assert(is_float());
  return value.as_float;
}

bool Value::is_null_or_false() const {
  return (tag == Tag::Null) || (tag == Tag::False);
}

bool Value::is_object() const { return tag == Tag::Object; }

Object *Value::as_object() const {
  assert(is_object());
  return value.as_object;
}

bool Value::is_null() const { return tag == Tag::Null; }

bool Value::is_empty() const { return tag == Tag::Empty; }

bool Value::is_bool() const { return tag == Tag::True || tag == Tag::False; }

bool Value::is_true() const { return tag == Tag::True; }

bool Value::is_false() const { return tag == Tag::False; }
#endif

bool Value::operator==(Value rhs) const {
  if (is_int()) {
    if (rhs.is_int())
      return as_int() == rhs.as_int();
    else if (rhs.is_float())
      return double(as_int()) == rhs.as_float();
    else
      return false;
  } else if (is_float()) {
    if (rhs.is_float())
      return as_float() == rhs.as_float();
    else if (rhs.is_int())
      return as_float() == double(rhs.as_int());
    else
      return false;
  } else if (is_object() && as_object()->is<String>() && rhs.is_object() &&
             rhs.as_object()->is<String>()) {
    return StringEquality{}(as_object()->as<String>(),
                            rhs.as_object()->as<String>());
#ifdef NANBOX
  } else if (inner == rhs.inner) {
    return true;
  } else {
    return false;
  }
#else
  } else if (is_object() && rhs.is_object()) {
    return as_object() == rhs.as_object();
  } else {
    return tag == rhs.tag;
  }
#endif
}
const char *Value::type_string() const {
  if (is_int())
    return "int";
  else if (is_float())
    return "float";
  else if (is_null())
    return "null";
  else if (is_bool())
    return "bool";
  else if (is_object())
    return as_object()->type_string();
  else
    unreachable();
}
std::ostream &operator<<(std::ostream &os, const Value &v) {
  if (v.is_int())
    os << v.as_int();
  else if (v.is_float())
    os << std::setprecision(14) << v.as_float();
  else if (v.is_null())
    os << "null";
  else if (v.is_true())
    os << "true";
  else if (v.is_false())
    os << "false";
  else if (v.is_object())
    os << *v.as_object();
  else
    unreachable();
  return os;
}

#ifndef NANBOX
// Thomas Wang's hash function
static uint32_t intHash(uint32_t key) {
  key += ~(key << 15);
  key ^= (key >> 10);
  key += (key << 3);
  key ^= (key >> 6);
  key += ~(key << 11);
  key ^= (key >> 16);
  return key;
}
#endif

// Thomas Wang's hash function
static uint32_t intHash(uint64_t key) {
  key += ~(key << 32);
  key ^= (key >> 22);
  key += ~(key << 13);
  key ^= (key >> 8);
  key += (key << 3);
  key ^= (key >> 15);
  key += ~(key << 27);
  key ^= (key >> 31);
  return static_cast<uint32_t>(key);
}

uint32_t ValueHasher::operator()(Value v) const {
#ifdef NANBOX
  if (v.is_object()) {
    auto o = v.as_object();
    if (o->is<Symbol>())
      return StringHasher{}(o->as<Symbol>());
    else if (o->is<String>())
      return StringHasher{}(static_cast<StringSlice>(*o->as<String>()));
    else
      return intHash(v.inner);
  } else {
    return intHash(v.inner);
  }
#else
  using Tag = Value::Tag;
  switch (v.tag) {
  case Tag::Int:
    return intHash(static_cast<uint32_t>(v.as_int()));
  case Tag::Float: {
    uint64_t u;
    auto f = v.as_float();
    memcpy(&u, &f, sizeof(u));
    return intHash(u);
  }
  case Tag::Object: {
    auto o = v.as_object();
    if (o->is<Symbol>())
      return StringHasher{}(o->as<Symbol>());
    else if (o->is<String>())
      return StringHasher{}(static_cast<StringSlice>(*o->as<String>()));
    else
      return intHash(reinterpret_cast<uintptr_t>(o));
  }
  default:
    return static_cast<uint32_t>(v.tag);
  }
#endif
}

#ifdef NANBOX
bool ValueStrictEquality::operator()(Value a, Value b) const {
  if (a.is_object() && a.as_object()->is<String>() && b.is_object() &&
      b.as_object()->is<String>()) {
    return StringEquality{}(a.as_object()->as<String>(),
                            b.as_object()->as<String>());
  } else {
    return a.inner == b.inner;
  }
}
#else
bool ValueStrictEquality::operator()(Value a, Value b) const {
  if (a.is_int() && b.is_int()) {
    return a.as_int() == b.as_int();
  } else if (a.is_float() && b.is_float()) {
    uint64_t u1, u2;
    double d1 = a.as_float(), d2 = a.as_float();
    memcpy(&u1, &d1, sizeof(u1));
    memcpy(&u2, &d2, sizeof(u2));
    return u1 == u2;
  } else if (a.is_object() && b.is_object()) {
    auto o1 = a.as_object();
    auto o2 = b.as_object();
    if (o1->is<String>() && o2->is<String>())
      return StringEquality{}(o1->as<String>(), o2->as<String>());
    else
      return reinterpret_cast<uintptr_t>(o1) == reinterpret_cast<uintptr_t>(o2);
  } else {
    return a.tag == b.tag;
  }
}
#endif
} // namespace neptune_vm
