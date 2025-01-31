fn main() {
    let proj = pkg_config::Config::new()
        .probe("proj")
        .expect("Could not find proj library");
    // these are not needed for `cargo run`, but are required for `cargo test` to find the library.
    for path in &proj.link_paths {
        println!("cargo:rustc-link-search=native={}", path.display());
    }
    for path in &proj.link_paths {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", path.display());
    }
    println!("cargo:rustc-link-lib=proj");


    // build the wrapper library
    cc::Build::new()
        .file("c/proj_wrapper.c")
        .compile("proj_wrapper");
}
