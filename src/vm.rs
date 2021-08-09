#[cxx::bridge(namespace = neptune_vm)]
mod ffi {
    unsafe extern "C++" {
        include!("neptune-lang/neptune-vm/neptune-vm.cc");
        type Value;
        fn is_int(self :&Value)->bool;
        fn as_int(self:&Value)->i32;
    }
}
