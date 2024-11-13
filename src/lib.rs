use pyo3::prelude::*;

pub mod mobject;

/// Sum two matrices.
#[pyfunction]
fn sum_matrix(a: Vec<Vec<f64>>, b: Vec<Vec<f64>>) -> Vec<Vec<f64>> {
    a.into_iter()
        .zip(b.into_iter())
        .map(|(a, b)| {
            a.into_iter()
                .zip(b.into_iter())
                .map(|(a, b)| a + b)
                .collect()
        })
        .collect()
}

/// A Python module implemented in Rust.
#[pymodule]
fn ranim(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(sum_matrix, m)?)?;
    Ok(())
}
