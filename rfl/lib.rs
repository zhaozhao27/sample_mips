pub mod kernel;
pub use kernel::alloc;
pub use kernel::print;
pub use kernel::str;
extern crate alloc;
pub mod bindings {
    mod lib;
    pub use lib::*;
}
