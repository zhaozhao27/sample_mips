#![no_std]
#![no_main]
#![feature(cfg_boolean_literals)]
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
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

mod common;
#[macro_use]
mod debug;
mod bgramapinfo;
mod logodata_100x100_bgra;
mod osd;
use core::arch::asm;
use core::fmt::Write;
use core::panic::PanicInfo; //替换rust-std 中的panic
use libc::{c_char, exit, write};
use osd::sample_osd_start;

struct Writer;

impl Writer {
    fn new() -> Self {
        Writer
    }

    fn write_bytes(&self, bytes: &[u8]) {
        unsafe {
            write(1, bytes.as_ptr() as *const core::ffi::c_void, bytes.len());
        }
    }
}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_bytes(s.as_bytes());
        Ok(())
    }
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    let msg: &[u8] = b"Hello, world!\n";
    unsafe {
        write(1, msg.as_ptr() as *const _, msg.len());
    }
    //let result: Result<i32, i32> = Err(-1);
    //result.unwrap();
    sample_osd_start().unwrap();
    unsafe { exit(0) }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut writer = Writer::new();

    // 打印简单的 panic 信息
    let _ = writeln!(writer, "[Panic Info] Panic occurred!");

    // 打印详细信息（如果有）
    if let Some(location) = info.location() {
        let _ = writeln!(
            writer,
            "[Panic Info] Panic at file '{}' at line {}",
            location.file(),
            location.line()
        );
    }

    // 打印可选的 panic 消息
    if let Some(message) = info.message().into() {
        let _ = writeln!(writer, "Message: {}", message);
    }

    // 退出程序
    unsafe {
        exit(1);
    }
}
/*
#[cfg(not(feature = "stable"))]
unsafe fn asm_write(fd: usize, buf: *const u8, count: usize) -> isize {
    let ret: isize;
    asm!(
        "move $a0, {fd}",        // 将 fd 移动到 a0
        "move $a1, {buf}",       // 将 buf 移动到 a1
        "move $a2, {count}",     // 将 count 移动到 a2
        "li $v0, 4004",          // 系统调用号 4004（write）
        "syscall",               // 执行系统调用
        "move {ret}, $v0",       // 将返回值放入 ret
        fd = in(reg) fd,
        buf = in(reg) buf,
        count = in(reg) count,
        ret = lateout(reg) ret,
    );
    ret
}
*/
