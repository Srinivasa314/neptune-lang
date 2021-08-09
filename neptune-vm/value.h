#pragma once
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "object.h"

#if (defined(__x86_64__) || defined(_M_X64) || defined(__aarch64__) || defined(_M_ARM64))
#define NANBOX
#endif

#define ASSERT(x)                            \
    do                                       \
    {                                        \
        if (!(x))                            \
        {                                    \
            puts("Assertion " #x " failed"); \
            abort();                         \
        }                                    \
    } while (0)

namespace neptune_vm
{
    namespace value
    {
        using object::Object;
#ifdef NANBOX
        class Value
        {
            uint64_t inner;
            static constexpr uint64_t VALUE_NULL = 1;
            static constexpr uint64_t VALUE_TRUE = 2;
            static constexpr uint64_t VALUE_FALSE = 3;
            Value(uint64_t u)
            {
                inner = u;
            }

        public:
            explicit Value(int32_t i)
            {
                inner = (1llu << 48) | static_cast<uint32_t>(i);
            }

            explicit Value(double d)
            {
                uint64_t u;
                memcpy(&u, &d, sizeof(u));
                inner = u + (2llu << 48);
            }

            explicit Value(Object o)
            {
                inner = (uint64_t)o;
            }

            explicit Value(bool b)
            {
                if (b)
                {
                    inner = VALUE_TRUE;
                }
                else
                {
                    inner = VALUE_FALSE;
                }
            }

            static Value new_true()
            {
                return Value(VALUE_TRUE);
            }

            static Value new_false()
            {
                return Value(VALUE_FALSE);
            }

            static Value null()
            {
                return Value(VALUE_NULL);
            }

            static Value empty()
            {
                return Value((uint64_t)0);
            }

            bool is_int() const
            {
                return (inner >> 48) == 1llu;
            }

            int32_t as_int() const
            {
                ASSERT(is_int());
                return static_cast<int32_t>(inner);
            }

            bool is_float() const
            {
                return inner >= (2llu << 48);
            }

            double as_float() const
            {
                ASSERT(is_float());
                double d;
                uint64_t u = inner - (2llu << 48);
                memcpy(&d, &u, sizeof(u));
                return d;
            }

            bool is_null_or_false() const
            {
                return (inner == VALUE_NULL) || (inner == VALUE_FALSE);
            }

            bool is_object() const
            {
                return ((inner >> 48) == 0) && inner > VALUE_FALSE;
            }

            Object as_object() const
            {
                ASSERT(is_object());
                return (Object)inner;
            }

            bool is_null() const
            {
                return inner == VALUE_NULL;
            }

            bool is_empty() const
            {
                return inner == 0;
            }
        };
#else
        enum class Tag : int8_t
        {
            Empty,
            Int,
            Float,
            Object,
            True,
            False,
            Null,
        };

        class Value
        {
            Tag tag;
            union
            {
                int32_t as_int;
                double as_float;
                Object as_object;
            } value;

            Value(Tag t)
            {
                tag = t;
            }

        public:
            explicit Value(int32_t i)
            {
                tag = Tag::Int;
                value.as_int = i;
            }

            explicit Value(double d)
            {
                tag = Tag::Float;
                value.as_float = d;
            }

            explicit Value(Object o)
            {
                tag = Tag::Object;
                value.as_object = o;
            }

            explicit Value(bool b)
            {
                if (b)
                {
                    tag = Tag::True;
                }
                else
                {
                    tag = Tag::False;
                }
            }

            static Value new_true()
            {
                return Value(Tag::True);
            }

            static Value new_false()
            {
                return Value(Tag::False);
            }

            static Value null()
            {
                return Value(Tag::Null);
            }

            static Value empty()
            {
                return Value(Tag::Empty);
            }

            bool is_int() const
            {
                return tag == Tag::Int;
            }

            int32_t as_int() const
            {
                ASSERT(is_int());
                return value.as_int;
            }

            bool is_float() const
            {
                tag == Tag::Float;
            }

            double as_float() const
            {
                ASSERT(is_float());
                return value.as_float;
            }

            bool is_null_or_false() const
            {
                return (tag == Tag::Null) || (tag == Tag::False);
            }

            bool is_object() const
            {
                return tag == Tag::Object;
            }

            Object as_object() const
            {
                ASSERT(is_object());
                return value.as_object;
            }

            bool is_null() const
            {
                return tag == Tag::Null;
            }

            bool is_empty() const
            {
                return tag == Tag::Empty;
            }
        };
#endif
    }
}

#undef ASSERT