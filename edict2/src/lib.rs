mod deinflect;
mod edict;
mod fragments;
mod subedict;

pub use deinflect::Deinflector;
pub use edict::Edict;
pub use fragments::iter_fragments;
pub use subedict::{SubEdictCreator, SubEnamdictCreator};

use pyo3::prelude::*;

use lazy_static::lazy_static;

lazy_static! {
    static ref SUB_EDICT_CREATOR: SubEdictCreator = SubEdictCreator::from_files();
    static ref SUB_ENAMDICT_CREATOR: SubEnamdictCreator = SubEnamdictCreator::from_files();
}

#[pyfunction]
#[pyo3(name = "subedict")]
fn py_subedict(content: &str) -> Vec<&str> {
    SUB_EDICT_CREATOR.from(content)
}

#[pyfunction]
#[pyo3(name = "subenamdict")]
fn py_subenamdict(content: &str) -> Vec<&str> {
    SUB_EDICT_CREATOR.from(content)
}

#[pymodule]
fn edict2(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_subedict, m)?)?;
    m.add_function(wrap_pyfunction!(py_subenamdict, m)?)?;
    Ok(())
}
