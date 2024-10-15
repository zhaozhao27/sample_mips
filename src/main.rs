#![no_std] //移除std使用，使用uclibc
#![no_main]
#![feature(asm_experimental_arch)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

pub mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
mod common;
mod osd;
use core::arch::asm;
use core::panic::PanicInfo; //替换rust-std 中的panic
use libc::{c_char, exit, write};

#[no_mangle]
pub extern "C" fn main() -> ! {
    let msg: &[u8] = b"Hello, world!\n";
    unsafe {
        write(1, msg.as_ptr() as *const _, msg.len());
        exit(0);
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

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
