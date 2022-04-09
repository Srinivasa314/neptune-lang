fn main() {
    println!("cargo:rerun-if-changed=neptune-vm");
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

    if cfg!(feature = "mimalloc") {
        build.file("vendor/github.com/microsoft/mimalloc/src/static.c");
        build.define("MI_MALLOC", "1");
    }

    build
        .include("vendor/github.com/dcleblanc")
        .include("vendor/github.com/microsoft/mimalloc/src")
        .include("vendor/github.com/microsoft/mimalloc/include")
        .file("neptune-vm/neptune-vm.cc")
        .compile("neptune-vm");
}
