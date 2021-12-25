fn main() {
    let mut build = cxx_build::bridge("src/vm.rs");

    let profile = std::env::var("PROFILE").unwrap();
    match profile.as_str() {
        "debug" => {
            build.define("MI_DEBUG", "3");
        }
        "release" => {
            build.define("MI_DEBUG", "0").define("NDEBUG", None);
        }
        _ => {}
    };

    build
        .include("vendor/github.com/Tessil/robin-map/include")
        .include("vendor/github.com/dcleblanc")
        .include("vendor/github.com/microsoft/mimalloc/src")
        .include("vendor/github.com/microsoft/mimalloc/include")
        .file("neptune-vm/neptune-vm.cc")
        .file("vendor/github.com/microsoft/mimalloc/src/static.c")
        .compile("neptune-vm");
}
