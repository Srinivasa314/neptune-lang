#pragma once

namespace neptune_vm {
struct Frame {
  Value *bp; // base pointer which points to the base of the stack
};

struct Global {
  std::string name;
  Value value;
};

class VM {
  std::vector<Value> stack;
  std::vector<Frame> frames;
  mutable std::vector<Global> globals;

public:
  mutable GC gc;
  // SAFETY:must not be null bytecode must be valid
  void run(uint8_t *bytecode);
  void add_global(StringSlice name) const;
};
} // namespace neptune_vm