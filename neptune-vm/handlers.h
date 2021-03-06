
handler(LoadR0, accumulator = bp[0];);
handler(LoadR1, accumulator = bp[1];);
handler(LoadR2, accumulator = bp[2];);
handler(LoadR3, accumulator = bp[3];);
handler(LoadR4, accumulator = bp[4];);
handler(LoadR5, accumulator = bp[5];);
handler(LoadR6, accumulator = bp[6];);
handler(LoadR7, accumulator = bp[7];);
handler(LoadR8, accumulator = bp[8];);
handler(LoadR9, accumulator = bp[9];);
handler(LoadR10, accumulator = bp[10];);
handler(LoadR11, accumulator = bp[11];);
handler(LoadR12, accumulator = bp[12];);
handler(LoadR13, accumulator = bp[13];);
handler(LoadR14, accumulator = bp[14];);
handler(LoadR15, accumulator = bp[15];);
handler(LoadSmallInt, accumulator = Value(READ(int8_t)););
handler(LoadNull, accumulator = Value::null(););
handler(LoadTrue, accumulator = Value::new_true(););
handler(LoadFalse, accumulator = Value::new_false(););
handler(StoreR0, bp[0] = accumulator;);
handler(StoreR1, bp[1] = accumulator;);
handler(StoreR2, bp[2] = accumulator;);
handler(StoreR3, bp[3] = accumulator;);
handler(StoreR4, bp[4] = accumulator;);
handler(StoreR5, bp[5] = accumulator;);
handler(StoreR6, bp[6] = accumulator;);
handler(StoreR7, bp[7] = accumulator;);
handler(StoreR8, bp[8] = accumulator;);
handler(StoreR9, bp[9] = accumulator;);
handler(StoreR10, bp[10] = accumulator;);
handler(StoreR11, bp[11] = accumulator;);
handler(StoreR12, bp[12] = accumulator;);
handler(StoreR13, bp[13] = accumulator;);
handler(StoreR14, bp[14] = accumulator;);
handler(StoreR15, bp[15] = accumulator;);
handler(
    Negate,
    if (accumulator.is_int()) {
      int result;
      if (unlikely(!SafeNegation(accumulator.as_int(), result))) {
        THROW("OverflowError",
              "Cannot negate " << accumulator.as_int()
                               << " as the result cannot be stored in an Int");
      }
      accumulator = Value(result);
    } else if (accumulator.is_float()) {
      accumulator = Value(-accumulator.as_float());
    } else {
      THROW("TypeError", "Cannot negate type " << accumulator.type_string());
    });
handler(Not, {
  if (accumulator.is_null_or_false()) {
    accumulator = Value(true);
  } else {
    accumulator = Value(false);
  }
});
handler(Return, {
  CLOSE(0);
  task->frames.pop_back();
  if (unlikely(task->frames.empty())) {
    task->stack_top = task->stack.get();
    task->status = VMStatus::Success;
    goto end;
  } else {
    auto frame = task->frames.back();
    bp = frame.bp;
    ip = frame.ip;
    auto f = frame.f;
    constants = f->function_info->constants.data();
    task->stack_top = bp + f->function_info->max_registers;
  }
});

handler(Throw, {
  task->frames.back().ip = ip;
  if ((ip = throw_(accumulator)) != nullptr) {
    bp = task->frames.back().bp;
    auto f = task->frames.back().f;
    constants = f->function_info->constants.data();
  } else {
    goto throw_end;
  }
});
