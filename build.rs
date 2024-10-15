use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() {
    println!("cargo:rustc-link-search=native=./lib/uclibc");
    println!("cargo:rustc-link-search=native=/home/zhaozhao/WorkSpace/ISVP-T23-1.1.2-20240204/software/zh/Ingenic-SDK-T23-1.1.2-20240204-zh/resource/toolchain/gcc_540/mips-gcc540-glibc222-64bit-r3.3.0.smaller/mips-linux-gnu/libc/uclibc/soft-float/lib/");
    println!("cargo:rustc-link-lib=static=imp");
    println!("cargo:rustc-link-lib=static=alog");
    println!("cargo:rustc-link-lib=dylib=uClibc-0.9.33.2");
    println!("cargo:include=/usr/mipsel-linux-gnu/include/");

    let include_dir = "./include";
    let mut builder = bindgen::Builder::default();
    for entry in WalkDir::new(include_dir).into_iter().filter_map(Result::ok) {
        if entry.path().is_file() && entry.path().extension().map(|s| s == "h").unwrap_or(false) {
            // 添加每个找到的头文件
            println!("Found header: {:?}", entry.path());
            builder = builder.header(entry.path().to_str().unwrap());
        }
    }

    // 添加包含目录和目标
    builder = builder
        .clang_arg(format!("-I{}", include_dir))
        .clang_arg("-I/usr/mipsel-linux-gnu/include/")
        .clang_arg("-target")
        .clang_arg("mipsel-unknown-linux-uclibc");
    let bindings = builder
        .ctypes_prefix("libc")
        //.clang_arg("-nostdinc")
        //.clang_arg("-isystem")
        .blocklist_type("_bindgen_ty_12")
        .blocklist_type("_bindgen_ty_16")
        .derive_default(true)
        .use_core()
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs")) // 生成的文件名
        .expect("Couldn't write bindings!");

    let bindings_file_path = out_path.join("bindings.rs");
    // 读取并修改生成的 bindings 文件
    let mut bindings_content =
        std::fs::read_to_string(&bindings_file_path).expect("Couldn't read bindings file");

    // 在文件顶部添加禁用警告的注释
    bindings_content.insert_str(0, "#[allow(non_upper_case_globals)]\n");

    // 将修改后的内容写回文件
    std::fs::write(&bindings_file_path, bindings_content)
        .expect("Couldn't write updated bindings file!");
}
