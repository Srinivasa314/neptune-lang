# Embedding API
Neptune lang can be embedded in any rust application. The complete API can be browsed through docs.rs. Some key terms that must known are

## Embedder Functions
Embedder Functions aka Efuncs are functions that are created by the embedder. They may be synchronous or asynchronous. The EFunc can push or pop values from the stack using the EFuncContext that is passed to it. EFuncs can be called using the `ecall` function in the `vm` module. EFuncs must return `Result` where both variants satisfy the `ToNeptuneValue` trait. The `Err` variant can be returned to throw an exception. `ToNeptuneValue` indicates a type that can be converted to a Neptune value. Asynchronous efuncs return a `Future<Result<T1,T2>>`. Examples for creating efuncs, implementing `ToNeptuneValue` can be seen in the documentation.

## Resources
Resources are opaque handles to rust values. They can be freed using  the `close()` method. They can be created and used only from efuncs using the `resource()` and `as_resource()` methods of `EFuncContext`.
