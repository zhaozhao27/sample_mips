use bindgen::{AliasVariation, EnumVariation, MacroTypeVariation, NonCopyUnionStyle};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(CONFIG_PRINTK)");
    let include_dir = "./include";
    let mut builder = bindgen::Builder::default();
    for entry in WalkDir::new(include_dir).into_iter().filter_map(Result::ok) {
        if entry.path().is_file() && entry.path().extension().map(|s| s == "h").unwrap_or(false) {
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
        .bindgen_wrapper_union("")
        .default_enum_style(EnumVariation::Rust {
            non_exhaustive: (false),
        })
        .default_alias_style(AliasVariation::TypeAlias)
        .default_non_copy_union_style(NonCopyUnionStyle::ManuallyDrop)
        .default_macro_constant_type(MacroTypeVariation::Unsigned)
        .anon_fields_prefix("__anon")
        .use_core()
        .generate()
        .expect("Unable to generate bindings");

    let bindings_path = PathBuf::from("bindings/bindings_generated.rs");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    //let bindings_path = out_path.join("bindings_generated.rs");
    bindings
        .write_to_file(&bindings_path) // 生成的文件名
        .expect("Couldn't write bindings!");

    // 读取并修改生成的 bindings 文件
    let mut bindings_content =
        std::fs::read_to_string(&bindings_path).expect("Couldn't read bindings file");

    // 在文件顶部添加禁用警告的注释
    bindings_content.insert_str(0, "#[allow(non_upper_case_globals)]\n");

    // 将修改后的内容写回文件
    std::fs::write(&bindings_path, bindings_content)
        .expect("Couldn't write updated bindings file!");

    let include_dir = "./include";
    let mut builder = bindgen::Builder::default();
    for entry in WalkDir::new(include_dir).into_iter().filter_map(Result::ok) {
        if entry.path().is_file() && entry.path().extension().map(|s| s == "h").unwrap_or(false) {
            println!("Found header: {:?}", entry.path());
            builder = builder.header(entry.path().to_str().unwrap());
        }
    }

    //helper
    let include_dir = "./bindings";
    let linux_include = "./include";
    let mut builder = bindgen::Builder::default();
    for entry in WalkDir::new(include_dir).into_iter().filter_map(Result::ok) {
        if entry.path().is_file() && entry.path().extension().map(|s| s == "h").unwrap_or(false) {
            println!("Found header: {:?}", entry.path());
            builder = builder.header(entry.path().to_str().unwrap());
        }
    }
    // 添加包含目录和目标
    builder = builder
        .clang_arg(format!("-I{}", linux_include))
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
        .bindgen_wrapper_union("")
        .default_enum_style(EnumVariation::Rust {
            non_exhaustive: (false),
        })
        .default_alias_style(AliasVariation::TypeAlias)
        .default_non_copy_union_style(NonCopyUnionStyle::ManuallyDrop)
        .default_macro_constant_type(MacroTypeVariation::Unsigned)
        .anon_fields_prefix("__anon")
        .use_core()
        .generate()
        .expect("Unable to generate bindings");

    let bindings_path = PathBuf::from("bindings/bindings_helpers_generated.rs");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    //let bindings_path = out_path.join("bindings_generated.rs");
    bindings
        .write_to_file(&bindings_path) // 生成的文件名
        .expect("Couldn't write bindings!");

    // 读取并修改生成的 bindings 文件
    let mut bindings_content =
        std::fs::read_to_string(&bindings_path).expect("Couldn't read bindings file");

    // 在文件顶部添加禁用警告的注释
    bindings_content.insert_str(0, "#[allow(non_upper_case_globals)]\n");

    // 将修改后的内容写回文件
    std::fs::write(&bindings_path, bindings_content)
        .expect("Couldn't write updated bindings file!");
}
