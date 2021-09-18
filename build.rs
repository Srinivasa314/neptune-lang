fn main() {
    let mut build = cxx_build::bridge("src/vm.rs");
    if !cfg!(debug_assertions) {
        build.define("NDEBUG", None);
    }
    build
        .include("vendor/github.com/Tessil/robin-map/include")
        .include("vendor/github.com/dcleblanc")
        .file("neptune-vm/neptune-vm.cc")
        .compile("neptune-vm");
}
