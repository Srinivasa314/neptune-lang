#pragma once
#include <cstring>
#include <iostream>
#include <stdexcept>
#include <string>

#ifndef NDEBUG
#define unreachable()                                                          \
  do {                                                                         \
    std::cout << "unreachable at: " << __FILE__ << ":" << __LINE__             \
              << std::endl;                                                    \
    exit(1);                                                                   \
  } while (0)
#elif defined(__GNUC__) || defined(__clang__) // gcc or clang
#define unreachable() __builtin_unreachable()
#elif defined(_MSC_VER) // MSVC
#define unreachable() __assume(false)
#else
#define unreachable() abort()
#endif

#if defined(__GNUC__) || defined(__clang__)
#define ALWAYS_INLINE inline __attribute__((__always_inline__))
#elif defined(_MSC_VER)
#define ALWAYS_INLINE __forceinline
#else
#define ALWAYS_INLINE inline
#endif

template <typename T> static ALWAYS_INLINE T read_unaligned(const void *ptr) {
  T ret;
  memcpy(&ret, ptr, sizeof(T));
  return ret;
}

template <typename T>
static ALWAYS_INLINE void write_unaligned(void *dest, T t) {
  memcpy(dest, &t, sizeof(T));
}

template <typename T> static ALWAYS_INLINE T read(const uint8_t *&ip) {
  auto ret = read_unaligned<T>(ip);
  ip += sizeof(T);
  return ret;
}

template <typename T>
static T checked_read(const uint8_t *&ip, const uint8_t *end) {
  if (ip + sizeof(T) > end) {
    throw std::overflow_error("Attempt to read out of bounds");
  }
  auto ret = read_unaligned<T>(ip);
  ip += sizeof(T);
  return ret;
}

// Size of Wide/Extrawide header if it exists
template <typename T> static size_t header_size() {
  if (sizeof(T) == 1)
    return 0;
  else
    return 1;
}

#define TODO()                                                                 \
  do {                                                                         \
    std::cout << "TODO at: " << __FILE__ << ":" << __LINE__ << std::endl;      \
    exit(1);                                                                   \
  } while (0)

#if defined(__GNUC__) || defined(__clang__)
#define likely(x) __builtin_expect((x), 1)
#define unlikely(x) __builtin_expect((x), 0)
#else
#define likely(x) x
#define unlikely(x) x
#endif

#ifdef __clang__
#define IF_CLANG(x) x
#else
#define IF_CLANG(x)
#endif
