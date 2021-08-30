fn main() {
    cxx_build::bridge("src/vm.rs")
        .include("vendor/github.com/Tessil/robin-map/include")
        .include("vendor/github.com/dcleblanc")
        .file("neptune-vm/neptune-vm.cc")
        .compile("neptune-vm");
}
