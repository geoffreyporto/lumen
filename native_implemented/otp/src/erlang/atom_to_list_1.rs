use liblumen_alloc::erts::exception;
use liblumen_alloc::erts::process::Process;
use liblumen_alloc::erts::term::prelude::*;

#[native_implemented::function(erlang:atom_to_list/1)]
pub fn result(process: &Process, atom: Term) -> exception::Result<Term> {
    let atom_atom = term_try_into_atom!(atom)?;
    let chars = atom_atom.name().chars();
    let list = process.list_from_chars(chars);

    Ok(list)
}
