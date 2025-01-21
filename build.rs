fn main() {
    pkg_config::Config::new()
        .probe("proj")
        .expect("Could not find proj library");
    cc::Build::new()
        .file("c/proj_wrapper.c")
        .compile("proj_wrapper");
}
