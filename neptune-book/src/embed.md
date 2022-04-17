# Embedding API

Neptune can be embedded in any rust application. The complete API can be
browsed through docs.rs. Some key terms that must known are

## Embedder Functions

Embedder Functions aka Efuncs are functions that are created by the embedder. The EFunc can push or pop values from the stack using the EFuncContext that is passed to it. EFuncs may be synchronous or asynchronous.

The `ToNeptuneValue` trait indicates a type that can be converted to a Neptune value. The
trait is implemented for i32, String and other common data types.

It can be implemented for any type

```rust,ignore
struct Point {
     x: i32,
     y: i32
 }

 impl ToNeptuneValue for Point {
     fn to_neptune_value(self, cx: &mut EFuncContext) {
         cx.object();    // push an empty object to the stack
         cx.int(self.x); // push self.x to the stack
         cx.set_object_property("x").unwrap(); // pop self.x and set it as property x
         self.y.to_neptune_value(cx); // an alternate way to push to the stack
         cx.set_object_property("y").unwrap();
     }
 }
```

EFuncs must return `Result` where both variants satisfy the `ToNeptuneValue`
trait. The `Err` variant can be returned to throw an exception.

Methods of EFuncContext like `as_int` return `Err(EFuncError)` on error. To return
`NeptuneError` (the `Error` class of Neptune) or EFuncError we can use the
EFuncErrorOr enum.

```rust,ignore
use neptune_lang::*;
let n = VM::new(NoopModuleLoader);
n.create_efunc("inverse", |cx /*: &mut EFuncContext*/ | -> Result<f64,EFuncErrorOr<NeptuneError>> {
    // pop an int from the stack
    let i = cx.as_int()?;
    if i == 0 {
        // It would be better to create our own Error type and implement ToNeptuneValue for it.
        Err(EFuncErrorOr::Other(NeptuneError("Cannot divide by zero".into())))
    }else{
            Ok(1.0 / (i as f64))
         }
    }).unwrap();
```

EFuncs can be called using the `ecall` function in the `vm` module.

```
const {ecall} = import('vm')
ecall(@inverse,0.5)  //2.0
```

Asynchronous efuncs return a `Future<Result<T1,T2>>`.

```rust,ignore
vm.create_efunc_async("sleep", |cx| {
    let time = cx.as_int();
    async move {
        sleep(Duration::from_millis(time? as u64)).await;
        Result::<(), EFuncError>::Ok(())
    }
})
```

## Resources

Resources are opaque handles to rust values. They can be freed
using the `close()` method in neptune lang. They can be created and used only from efuncs. The
`Resource` wrapper struct can be used to return resources

```rust,ignore
use std::fs::File;
use std::io::prelude::*;
use neptune_lang::*;

 n.create_efunc(
     "file_open",
     |cx| -> Result<Resource<File>, EFuncErrorOr<NeptuneError>> {
         Ok(Resource(File::open(cx.as_string()?).or(Err(
             EFuncErrorOr::Other(NeptuneError("Error opening file".into())),
         ))?))
     },
 )
 .unwrap();
 n.create_efunc(
     "file_read_all",
     |cx| -> Result<String, EFuncErrorOr<NeptuneError>> {
         let mut contents = String::new();
         cx.as_resource::<File>()?
             .read_to_string(&mut contents)
             .or(Err(EFuncErrorOr::Other(NeptuneError(
                 "Error reading file".into(),
             ))))?;
         Ok(contents)
     },
 )
 .unwrap();
```
