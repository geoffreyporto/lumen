//! Lumen intrinsics

pub mod is_big_integer_1;
pub mod is_small_integer_1;

use liblumen_alloc::erts::term::prelude::*;

pub fn module() -> Atom {
    Atom::from_str("lumen")
}

pub fn module_id() -> usize {
    module().id()
}
