# Neptune Language Internals

## VM Design
Like V8, the VM is a register-based VM that has a special accumulator register. The accumulator register is the implicit input/output register for many ops. This reduces the number of arguments needed.
The VM also has many dedicated ops to speed up integer operations like `AddInt`,`LoadSmallInt` and `ForLoop`.
The bytecode generated for a function can be viewed by the disassemble function in the `vm` module.

## Value representation
On x86_64 and aarch64 the following scheme is used to represent values.

Empty   0x0000 0000 0000 0000 (nullptr)
Null    0x0000 0000 0000 0001
True    0x0000 0000 0000 0002
False   0x0000 0000 0000 0003
Pointer 0x0000 XXXX XXXX XXXX [due to alignment we can use the last 2bits]
Int     0x0001 0000 XXXX XXXX
Float   0x0002 0000 0000 0000
                  to
        0xFFFA 0000 0000 0000

Doubles lie from 0x0000000000000000 to 0xFFF8000000000000. On adding 2<<48
they lie in the range listed above.

## ForLoop op
Many for loops are of the form
```
for i in a..b {
    do something
}
```
If `hasNext` and `next` methods are called it would be very slow. So two specialized ops exist for for loops of this form. 
* `BeginForLoop`: It checks whether both the start and end are integers and whether the start is lesser than the end.It is only called once.
* `ForLoop`: It just increments the integer loop variable and compares it so it is much faster than other for loops.

## Wide and Extrawide arguments
To reduce bytecode size Neptune lang uses the strategy that V8 does. An op can have arguments of any size. 8 bit arguments are used normally but prefix bytecodes are used for 16 bit(wide) and 32 bit(extrawide) arguments. The `Wide` and `Extrawide` ops precede instructions with these arguments. These ops read the op next to it and dispatch to the wide and extrawide variants of the ops. The wide and extrawide handlers are assigned entries in the bytecode dispatch table that have a fixed offset from the normal variants. Macros are used to generate the wide and extrawide bytecode handlers. This scheme has the problem that the number of bytes to reserve for jump offsets is not known. To resolve this problem `JumpConstant`, `JumpIfFalseOrNullConstant` and similar ops exist. The jump offset is contained in the constants table. If later it is found that enough space exists to store the jump offset directly in the bytecode then they are converted to the non-constant variants like `Jump` and `JumpIfFalseOrNull` and the bytecode is patched. If enough space is not available then the constant table must be patched.
```
|   AddInt  | |     5     |
|    Wide   | |   AddInt  | |         300          |
| Extrawide | |   AddInt  | |                10_000                      |
```

## Maybe something related to tasks,channels or async?