#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![cfg_attr(feature = "std", feature(start))]
#![feature(stmt_expr_attributes)]
#![feature(asm_experimental_arch)]
// #![cfg_attr(
//     target_arch = "mipsel-unknown-linux-uclibc",
//     no_std,
//     no_main,
//     feature(cfg_boolean_literals),
//     feature(stmt_expr_attributes),
//     feature(asm_experimental_arch)
// )]
#![cfg_attr(
    not(debug_assertions),
    allow(non_upper_case_globals),
    allow(non_snake_case),
    allow(non_camel_case_types)
)]

pub mod bindings {
    // include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
    include!("bindings.rs");
}
#[macro_use]
extern crate alloc;
//extern crate libc_alloc;

use alloc::string::String;
use alloc::vec::Vec;

mod bgramapinfo;
mod common;
#[macro_use]
mod debug;
mod logodata_100x100_bgra;
mod osd;
pub use alloc::{boxed::Box, vec};
use core::arch::asm;
use core::fmt::Write;
use core::panic::PanicInfo; //替换rust-std 中的panic
use libc::{c_char, exit, write};
pub use libc_print::std_name::println;
pub use log::{self, LevelFilter};
use osd::sample_osd_start;

use libc_alloc::LibcAlloc;

#[global_allocator]
static ALLOCATOR: LibcAlloc = LibcAlloc;

// #[no_mangle]
// pub extern "C" fn main() -> ! {
#[start]
fn main(argc: isize, argv: *const *const u8) -> isize {
    env_logger::builder().filter_level(LevelFilter::Info).init();
    sample_osd_start().unwrap();
    unsafe { exit(0) }
}

#[allow(dead_code)]
#[cfg_attr(not(feature = "std"), panic_handler)]
fn panic(info: &PanicInfo) -> ! {
    println!("[Panic] Panic occurred!");

    if let Some(location) = info.location() {
        println!(
            "[Panic] Panic at file '{}' at line {}",
            location.file(),
            location.line()
        );
    }

    if let Some(message) = info.message().into() {
        println!("[Panic] Panic Message: {}", message);
    }
    unsafe { libc::abort() }
}

// #[cfg(not(feature = "stable"))]
// unsafe fn asm_write(fd: usize, buf: *const u8, count: usize) -> isize {
//     let ret: isize;
//     asm!(
//         "move $a0, {fd}",        // 将 fd 移动到 a0
//         "move $a1, {buf}",       // 将 buf 移动到 a1
//         "move $a2, {count}",     // 将 count 移动到 a2
//         "li $v0, 4004",          // 系统调用号 4004（write）
//         "syscall",               // 执行系统调用
//         "move {ret}, $v0",       // 将返回值放入 ret
//         fd = in(reg) fd,
//         buf = in(reg) buf,
//         count = in(reg) count,
//         ret = lateout(reg) ret,
//     );
//     ret
// }
