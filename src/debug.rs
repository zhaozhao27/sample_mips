// debug.rs
// 日志模块
use log::{debug, error, info};

#[macro_export]
macro_rules! imp_log_err {
    ($tag:expr, $fmt:expr) => {{
        log::error!("[{}]:{}",$tag, format!($fmt));
    }};
    ($tag:expr, $fmt:expr,$($arg:tt)*) => {{
        log::error!("[{}]:{}",$tag, format!($fmt,$($arg)*));
    }};
}

#[macro_export]
macro_rules! imp_log_info {
    ($tag:expr,$fmt:expr) => {{
        log::info!("[{}]:{}",$tag,format!($fmt));
    }};
    ($tag:expr,$fmt:expr,$($arg:tt)*) => {{
        log::info!("[{}]:{}",$tag,format!($fmt,$($arg)*));
    }};
}

#[macro_export]
macro_rules! imp_log_dbg {
    ($tag:expr,$fmt:expr) => {{
        log::debug!("[{}]:{}",$tag,format!($fmt));
    }};
    ($tag:expr,$fmt:expr,$($arg:tt)*) => {{
        log::debug!("[{}]:{}",$tag,format!($fmt,$($arg)*));
    }};
}

#[macro_export]
macro_rules! ret_verify {
    ($ret:expr, $fmt:expr) => {
        if $ret < 0 {
            imp_log_err!("Err","{}",format!($fmt));
            return Err($ret);
        }
    };

    ($ret:expr, $fmt:expr,$($arg:tt)*) => {
        if $ret < 0 {
            imp_log_err!("Err","{}",format!($fmt,$($arg)*));
            return Err($ret);
        }
    };
}

// pub struct PrintWriter;

// impl Write for PrintWriter {
//     fn write_str(&mut self, s: &str) -> fmt::Result {
//         // 将 Rust 字符串转换为 C 字符串并调用 printf
//         unsafe {
//             printf(s.as_ptr() as *const _);
//         }
//         Ok(())
//     }
// }

// #[macro_export]
// macro_rules! imp_log_err {
//     ($fmt:expr) => {{
//         use core::fmt::Write;
//         //panic!($fmt);
//         let mut writer = crate::debug::PrintWriter;
//         let _ = core::write!(writer, concat!("[ERROR] ","Sample: ", $fmt, "\n"));
//     }};

//     ($fmt:expr, $($arg:tt)*) => {{
//         use core::fmt::Write;
//         //panic!();
//         let mut writer = crate::debug::PrintWriter;
//         let _ = core::write!(writer, concat!("[ERROR] ","Sample: ", $fmt, "\n\0"), $($arg)*);
//     }};
// }

// #[macro_export]
// macro_rules! imp_log_info {
//     ($fmt:expr) => {{
//         use core::fmt::Write;
//         let mut writer = crate::debug::PrintWriter;
//         let _ = core::write!(writer, concat!("[INFO] ","Sample: ", $fmt, "\n\0"));
//     }};

//     ($fmt:expr, $($arg:tt)*) => {{
//         use core::fmt::Write;
//         let mut writer = crate::debug::PrintWriter;
//         let _ = core::write!(writer, concat!("[INFO] ","Sample: ", $fmt, "\n\0"), $($arg)*);
//     }};
// }

// #[macro_export]
// macro_rules! imp_log_dbg {
//     ($fmt:expr) => {{
//         use core::fmt::Write;
//         let mut writer = crate::debug::PrintWriter;
//         let _ = core::write!(writer, concat!("[DEBUG] ","Sample: ", $fmt, "\n\0"));
//     }};

//     ($fmt:expr, $($arg:tt)*) => {{
//         use core::fmt::Write;
//         let mut writer = crate::debug::PrintWriter;
//         let _ = core::write!(writer, concat!("[DEBUG] ","Sample: ", $fmt, "\n\0"), $($arg)*);
//     }};
// }

// #[macro_export]
// macro_rules! ret_verify {
// /*
//     ($ret:expr, $tag:expr, $err_msg:expr, $($arg:tt)*) => {
//         if $ret < 0 {
//             crate::imp_log_err!($tag, $err_msg, $($arg)*);
//             return Err($ret); // 返回错误
//         }
//     };
// */
//     ($ret:expr, $err_msg:expr, $($arg:tt)*) => {
//         if $ret < 0 {
//             crate::imp_log_err!( $err_msg, $($arg)*);
//             return Err($ret); // 返回错误
//         }
//     };

//     ($ret:expr, $err_msg:expr) => {
//         if $ret < 0 {
//             crate::imp_log_err!( "Error: {}", $err_msg);
//             return Err($ret);
//         }
//     };
// }
