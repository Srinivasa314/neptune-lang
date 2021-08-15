#include "neptune-vm.h"
namespace neptune_vm {
void VM::run(uint8_t *bytecode) {
  /*#define WIDE(x) ((x) + 10)
  #define EXTRAWIDE(x) ((x) + 20)
  #ifdef COMPUTED_GOTO
    static void *dispatch_table[] = {};
  #define CASE(x) x
  #define DISPATCH() goto *dispatch_table[*bytecode++]
  #define DISPATCH_WIDE() goto *dispatch_table[WIDE(*bytecode++)]
  #define DISPATCH_EXTRAWIDE() goto *dispatch_table[EXTRAWIDE(*bytecode++)]
  #define INTERPRET_LOOP DISPATCH();
  #else
  #define CASE(x) case Op::x
  #define INTERPRET_LOOP \
    uint8_t op; \
    DISPATCH(); \
    loop: \ switch (static_cast<Op>(op))
  #define DISPATCH() \
    op = EXTRAWIDE(*bytecode++); \ goto loop
  #define DISPATCH_WIDE() \
    op = WIDE(*bytecode++); \ goto loop
  #define DISPATCH_EXTRAWIDE() \
    op = EXTRAWIDE(*bytecode++); \ goto loop #endif INTERPRET_LOOP { CASE(Wide)
  : DISPATCH_WIDE(); CASE(ExtraWide) : DISPATCH_EXTRAWIDE();
    }*/
}

void VM::add_global(StringSlice name) const {
  globals.push_back(Global{std::string(name.data, name.len), Value::empty()});
}
} // namespace neptune_vm
