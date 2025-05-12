use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rustc-link-search=/usr/local/lib")
}