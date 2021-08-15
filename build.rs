fn main() {
    cxx_build::bridge("src/vm.rs")
        .include("vendor/github.com/Tessil/robin-map/include")
        .file("neptune-vm/neptune-vm.cc")
        .compile("neptune-vm");
}
