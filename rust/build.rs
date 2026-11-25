fn main() {
    // cxx emits the C++ half of the bridge under target/cxxbridge; CMake adds
    // that to the include path. That is the whole coupling between the two.
    cxx_build::bridge("src/bridge.rs").std("c++17").compile("loadout_bridge");
    println!("cargo:rerun-if-changed=src/bridge.rs");
}
