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

ALWAYS_INLINE Value::Value(int32_t i) {
  inner = INT_ENCODING_OFFSET | static_cast<uint32_t>(i);
}

ALWAYS_INLINE Value::Value(double d) {
  uint64_t u;
  memcpy(&u, &d, sizeof(u));
  inner = u + DOUBLE_ENCODING_OFFSET;
}

ALWAYS_INLINE Value::Value(Object *o) { inner = reinterpret_cast<uint64_t>(o); }

ALWAYS_INLINE Value::Value(bool b) {
  if (b) {
    inner = VALUE_TRUE;
  } else {
    inner = VALUE_FALSE;
  }
}

ALWAYS_INLINE bool Value::is_int() const { return (inner >> 48) == 1llu; }

ALWAYS_INLINE int32_t Value::as_int() const {
  assert(is_int());
  return static_cast<int32_t>(inner);
}

ALWAYS_INLINE bool Value::is_float() const {
  return inner >= DOUBLE_ENCODING_OFFSET;
}

ALWAYS_INLINE double Value::as_float() const {
  assert(is_float());
  double d;
  uint64_t u = inner - DOUBLE_ENCODING_OFFSET;
  memcpy(&d, &u, sizeof(u));
  return d;
}

ALWAYS_INLINE bool Value::is_null_or_false() const {
  return (inner == VALUE_NULL) || (inner == VALUE_FALSE);
}

ALWAYS_INLINE bool Value::is_object() const {
  // return ((inner >> 48) == 0) && ((inner % 4) == 0);
  return !(inner & OBJECT_MASK);
}

ALWAYS_INLINE Object *Value::as_object() const {
  assert(is_object());
  return reinterpret_cast<Object *>(inner);
}

ALWAYS_INLINE bool Value::is_null() const { return inner == VALUE_NULL; }

ALWAYS_INLINE bool Value::is_empty() const { return inner == 0; }

ALWAYS_INLINE bool Value::is_bool() const {
  return inner == VALUE_TRUE || inner == VALUE_FALSE;
}

ALWAYS_INLINE bool Value::is_true() const { return inner == VALUE_TRUE; }

ALWAYS_INLINE bool Value::is_false() const { return inner == VALUE_FALSE; }

ALWAYS_INLINE void Value::inc() {
  assert(is_int());
  inner++; // there is no need to check for overflow because it is impossible
}

#else
ALWAYS_INLINE Value::Value(int32_t i) {
  tag = Tag::Int;
  value.as_int = i;
}

ALWAYS_INLINE Value::Value(double d) {
  tag = Tag::Float;
  value.as_float = d;
}

ALWAYS_INLINE Value::Value(Object *o) {
  tag = Tag::Object;
  value.as_object = o;
}

ALWAYS_INLINE Value::Value(bool b) {
  if (b) {
    tag = Tag::True;
  } else {
    tag = Tag::False;
  }
}

ALWAYS_INLINE bool Value::is_int() const { return tag == Tag::Int; }

ALWAYS_INLINE int32_t Value::as_int() const {
  assert(is_int());
  return value.as_int;
}

ALWAYS_INLINE bool Value::is_float() const { return tag == Tag::Float; }

ALWAYS_INLINE double Value::as_float() const {
  assert(is_float());
  return value.as_float;
}

ALWAYS_INLINE bool Value::is_null_or_false() const {
  return (tag == Tag::Null) || (tag == Tag::False);
}

ALWAYS_INLINE bool Value::is_object() const { return tag == Tag::Object; }

ALWAYS_INLINE Object *Value::as_object() const {
  assert(is_object());
  return value.as_object;
}

ALWAYS_INLINE bool Value::is_null() const { return tag == Tag::Null; }

ALWAYS_INLINE bool Value::is_empty() const { return tag == Tag::Empty; }

ALWAYS_INLINE bool Value::is_bool() const {
  return tag == Tag::True || tag == Tag::False;
}

ALWAYS_INLINE bool Value::is_true() const { return tag == Tag::True; }

ALWAYS_INLINE bool Value::is_false() const { return tag == Tag::False; }

ALWAYS_INLINE void Value::inc() {
  assert(is_int());
  value.as_int++; // there is no need to check for overflow because it is
                  // impossible
}
#endif

ALWAYS_INLINE bool Value::operator==(Value rhs) const {
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
ALWAYS_INLINE const char *Value::type_string() const {
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
void operator<<(ValueFormatter vf, Value v) {
  if (v.is_int())
    vf.os << v.as_int();
  else if (v.is_float()) {
    auto f = v.as_float();
    if (std::isnan(f)) {
      if (std::signbit(f))
        vf.os << "-nan";
      else
        vf.os << "nan";
    } else {
      vf.os << std::setprecision(14) << f;
      if (fmod(f, 1.0) == 0)
        vf.os << ".0";
    }
  } else if (v.is_null())
    vf.os << "null";
  else if (v.is_true())
    vf.os << "true";
  else if (v.is_false())
    vf.os << "false";
  else if (v.is_object())
    vf << v.as_object();
  else
    unreachable();
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
    double d1 = a.as_float(), d2 = b.as_float();
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

std::ostream &operator<<(std::ostream &os, Value v) {
  ValueFormatter vf{os};
  vf << v;
  return os;
}

std::ostream &operator<<(std::ostream &os, Object &o) {
  ValueFormatter vf{os};
  vf << &o;
  return os;
}
} // namespace neptune_vm
