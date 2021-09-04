#pragma once

#if (defined(__x86_64__) || defined(_M_X64) || defined(__aarch64__) ||         \
     defined(_M_ARM64))
#define NANBOX
#endif

namespace neptune_vm {
class Value {
#ifdef NANBOX
  uint64_t inner;
  static constexpr uint64_t VALUE_NULL = 1;
  static constexpr uint64_t VALUE_TRUE = 2;
  static constexpr uint64_t VALUE_FALSE = 3;
  explicit Value(uint64_t u) { inner = u; }

public:
  static Value new_true() { return Value(VALUE_TRUE); }

  static Value new_false() { return Value(VALUE_FALSE); }

  static Value null() { return Value(VALUE_NULL); }

  static Value empty() { return Value(static_cast<uint64_t>(0)); }

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

  explicit Value(Tag t) { tag = t; }

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
  bool is_bool() const;
  bool is_true() const;
  bool is_false() const;
  bool operator==(Value rhs) const;
  const char *type_string() const;
  friend std::ostream &operator<<(std::ostream &os, const Value &v);
};
} // namespace neptune_vm
