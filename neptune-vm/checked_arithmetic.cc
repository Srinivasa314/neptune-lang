#if defined(__GNUC__) || defined(__clang__)
#include <limits>
#include <stdint.h>

static ALWAYS_INLINE bool SafeAdd(int32_t a, int32_t b, int32_t &result) {
  return !__builtin_add_overflow(a, b, &result);
}

static ALWAYS_INLINE bool SafeSubtract(int32_t a, int32_t b, int32_t &result) {
  return !__builtin_sub_overflow(a, b, &result);
}

static ALWAYS_INLINE bool SafeMultiply(int32_t a, int32_t b, int32_t &result) {
  return !__builtin_mul_overflow(a, b, &result);
}

static ALWAYS_INLINE bool SafeDivide(int32_t a, int32_t b, int32_t &result) {
  if (b == 0)
    return false;
  if (a == std::numeric_limits<int32_t>::min() && b == int32_t(-1))
    return false;
  result = a / b;
  return true;
}

static ALWAYS_INLINE bool SafeModulus(int32_t a, int32_t b, int32_t &result) {
  if (b == 0)
    return false;
  if (b == int32_t(-1)) {
    result = 0;
    return true;
  }
  result = a % b;
  return true;
}

static ALWAYS_INLINE bool SafeNegation(int32_t a, int32_t &result) {
  if (a != std::numeric_limits<int32_t>::min()) {
    result = -a;
    return true;
  }
  return false;
}
#else
#include <SafeInt/SafeInt.hpp>
#endif
