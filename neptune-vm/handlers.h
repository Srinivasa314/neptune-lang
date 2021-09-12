
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
      if (!SafeNegation(accumulator.as_int(), result)) {
        PANIC("Cannot negate " << accumulator.as_int()
                               << " as the result cannot be stored in an int");
      }
      accumulator = Value(result);
    } else if (accumulator.is_float()) {
      accumulator = Value(-accumulator.as_float());
    } else { PANIC("Cannot negate type " << accumulator.type_string()); });

handler(ToString, {
  if (accumulator.is_int()) {
    char buffer[12];
    size_t len =
        static_cast<size_t>(sprintf(buffer, "%d", accumulator.as_int()));
    accumulator =
        Value(manage(String::from_string_slice(StringSlice{buffer, len})));
  } else if (accumulator.is_float()) {
    auto f = accumulator.as_float();
    if (std::isnan(f)) {
      const char *result = std::signbit(f) ? "-nan" : "nan";
      accumulator = Value(manage(
          String::from_string_slice(StringSlice{result, strlen(result)})));
    } else {
      char buffer[24];
      size_t len = static_cast<size_t>(sprintf(buffer, "%.14g", f));
      if (strspn(buffer, "0123456789-") == len) {
        buffer[len] = '.';
        buffer[len + 1] = '0';
        len += 2;
      }
      accumulator =
          Value(manage(String::from_string_slice(StringSlice{buffer, len})));
    }
  } else if (accumulator.is_object()) {
    if (accumulator.as_object()->is<String>()) {
    } else if (accumulator.as_object()->is<Symbol>()) {
      accumulator = Value(manage(String::from_string_slice(
          static_cast<StringSlice>(*accumulator.as_object()->as<Symbol>()))));
    }
  } else if (accumulator.is_true()) {
    accumulator = Value(
        manage(String::from_string_slice(StringSlice{"true", strlen("true")})));
  } else if (accumulator.is_false()) {
    accumulator = Value(manage(
        String::from_string_slice(StringSlice{"false", strlen("false")})));
  } else if (accumulator.is_null()) {
    accumulator = Value(
        manage(String::from_string_slice(StringSlice{"null", strlen("null")})));
  } else {
    std::ostringstream os;
    os << accumulator;
    auto s = os.str();
    accumulator = Value(
        manage(String::from_string_slice(StringSlice{s.data(), s.length()})));
  }
});
handler(EmptyArray, accumulator = Value{manage(new Array)};);
handler(EmptyMap, accumulator = Value{manage(new Map)};);
handler(Return, TODO(););
handler(Exit, goto end;);
