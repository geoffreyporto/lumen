use liblumen_alloc::erts::process::Process;
use liblumen_alloc::erts::term::prelude::*;

#[native_implemented::function(erlang:get/0)]
pub fn result(process: &Process) -> Term {
    process.get_entries()
}
