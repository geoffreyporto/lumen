#[cfg(all(not(target_arch = "wasm32"), test))]
mod test;

use anyhow::*;

use liblumen_alloc::erts::exception;
use liblumen_alloc::erts::process::Process;
use liblumen_alloc::erts::term::prelude::*;

#[native_implemented::function(erlang:list_to_tuple/1)]
pub fn result(process: &Process, list: Term) -> exception::Result<Term> {
    match list.decode().unwrap() {
        TypedTerm::Nil => Ok(process.tuple_from_slice(&[])),
        TypedTerm::List(cons) => {
            let vec: Vec<Term> = cons
                .into_iter()
                .collect::<std::result::Result<_, _>>()
                .map_err(|_| ImproperListError)
                .with_context(|| format!("list ({}) is improper", list))?;

            Ok(process.tuple_from_slice(&vec))
        }
        _ => Err(TypeError)
            .context(format!("list ({}) is not a list", list))
            .map_err(From::from),
    }
}
