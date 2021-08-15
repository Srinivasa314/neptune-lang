#pragma once
#include "function.h"
#include "gc.h"
#include "object.h"
#include "op.h"
#include "stddef.h"
#include "value.h"
#include "vm.h"
namespace neptune_vm {
struct StringSlice {
  char *data;
  size_t len;
};
} // namespace neptune_vm