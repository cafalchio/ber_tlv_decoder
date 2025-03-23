use pyo3::prelude::*;
use ber_tlv_decoder::TlvObject;

#[pyfunction]
pub fn decode_from_gz_file() -> PyResult<TlvObject>{

}

#[pyfunction]
pub fn decode_from_gz_path() -> PyResult<Vec<TlvObject>>{

}

#[pymodule]
fn ber_tlv_decoder(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(decode_from_gz_file, m)?)?;
    m.add_function(wrap_pyfunction!(decode_from_gz_path, m)?)?;
    Ok(())
}