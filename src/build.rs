#![allow(unused_imports)]
use bindgen::{AliasVariation, EnumVariation, MacroTypeVariation, NonCopyUnionStyle};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() {
    let target = std::env::var("TARGET").unwrap();
    if true {
        println!("cargo:rustc-link-search=native=./lib/uclibc");
        println!("cargo:rustc-link-search=native=/home/zhaozhao/WorkSpace/ISVP-T23-1.1.2-20240204/software/zh/Ingenic-SDK-T23-1.1.2-20240204-zh/resource/toolchain/gcc_540/mips-gcc540-glibc222-64bit-r3.3.0.smaller/mips-linux-gnu/libc/uclibc/soft-float/lib/");

        //println!("cargo:rustc-link-search=native=/home/zhaozhao/WorkSpace/ISVP-T23-1.1.2-20240204/software/zh/Ingenic-SDK-T23-1.1.2-20240204-zh/resource/toolchain/gcc_540/mips-gcc540-glibc222-64bit-r3.3.0.smaller/mips-linux-gnu/libc/lib/");
        // println!("cargo:rustc-link-lib=static=imp");
        println!("cargo:rustc-link-lib=dylib=imp");
        //println!("cargo:rustc-link-lib=static=alog");
        println!("cargo:rustc-link-lib=dylib=alog");
        println!("cargo:rustc-link-lib=dylib=uClibc-0.9.33.2");
        //println!("cargo:rustc-link-lib=dylib=libc-2.22");
        // mipsel-gcc-9 debug
        // println!("cargo:rustc-link-lib=static=uClibc-0.9.33.2");

        println!("cargo:include=/usr/mipsel-linux-gnu/include/");
        println!("cargo:rustc-link-arg=-lm -lrt -lpthread");
        // println!("cargo:rustc-link-lib=m");
        // println!("cargo:rustc-link-lib=rt");
        // println!("cargo:rustc-link-lib=pthread");
        // 指定在设备上需要链接的动态库的路径，目前只是暂时修改
        println!("cargo:rustc-link-arg=-Wl,-rpath,/system/nfs");
        // mipsel-gcc-9 debug
        // println!("cargo:rustc-link-arg=-Wl,-rpath,/home/zhaozhao/WorkSpace/ISVP-T23-1.1.2-20240204/software/zh/Ingenic-SDK-T23-1.1.2-20240204-zh/resource/toolchain/gcc_540/mips-gcc540-glibc222-64bit-r3.3.0.smaller/mips-linux-gnu/libc/uclibc/soft-float/lib/");

        let include_dir = "../include";
        let mut builder = bindgen::Builder::default();
        for entry in WalkDir::new(include_dir).into_iter().filter_map(Result::ok) {
            if entry.path().is_file() && entry.path().extension().map(|s| s == "h").unwrap_or(false)
            {
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

        let bindings_path = PathBuf::from("bindings.rs");
        //let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
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
    } else if target.contains("mipsel-unknown-linux-gnu") {
        println!("cargo:rustc-link-search=native=../lib/glibc");
        println!("cargo:rustc-link-search=native=/opt/mips-gcc540-glibc222-64bit-r3.3.0/mips-linux-gnu/libc/lib/");

        //println!("cargo:rustc-link-search=native=/home/zhaozhao/WorkSpace/ISVP-T23-1.1.2-20240204/software/zh/Ingenic-SDK-T23-1.1.2-20240204-zh/resource/toolchain/gcc_540/mips-gcc540-glibc222-64bit-r3.3.0.smaller/mips-linux-gnu/libc/lib/");
        // println!("cargo:rustc-link-lib=static=imp");
        println!("cargo:rustc-link-lib=dylib=imp");
        //println!("cargo:rustc-link-lib=static=alog");
        println!("cargo:rustc-link-lib=dylib=alog");
        //println!("cargo:rustc-link-lib=dylib=uClibc-0.9.33.2");
        //println!("cargo:rustc-link-lib=dylib=libc-2.22");
        println!("cargo:rustc-link-lib=dylib=dl");
        // mipsel-gcc-9 debug
        // println!("cargo:rustc-link-lib=static=uClibc-0.9.33.2");

        println!("cargo:include=/usr/mipsel-linux-gnu/include/");
        //println!("cargo:rustc-link-arg=-lm -lrt -lpthread");
        // println!("cargo:rustc-link-lib=m");
        // println!("cargo:rustc-link-lib=rt");
        // println!("cargo:rustc-link-lib=pthread");

        // 指定在设备上需要链接的动态库的路径，目前只是暂时修改
        println!("cargo:rustc-link-arg=-Wl,-rpath,/system/nfs");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/lib");
        // mipsel-gcc-9 debug
        // println!("cargo:rustc-link-arg=-Wl,-rpath,/home/zhaozhao/WorkSpace/ISVP-T23-1.1.2-20240204/software/zh/Ingenic-SDK-T23-1.1.2-20240204-zh/resource/toolchain/gcc_540/mips-gcc540-glibc222-64bit-r3.3.0.smaller/mips-linux-gnu/libc/uclibc/soft-float/lib/");

        let include_dir = "../include";
        let mut builder = bindgen::Builder::default();
        for entry in WalkDir::new(include_dir).into_iter().filter_map(Result::ok) {
            if entry.path().is_file() && entry.path().extension().map(|s| s == "h").unwrap_or(false)
            {
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
}
