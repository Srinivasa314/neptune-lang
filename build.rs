fn main() {
    cxx_build::bridge("src/vm.rs")
        .file("neptune-vm/neptune-vm.cc")
        .compile("neptune-vm");
}
