use log::info;
use pyo3::ffi::c_str;
use pyo3::prelude::*;
use rusty_tesseract::Image;
use std::ops::Deref;
use std::sync::LazyLock;

pub static MANGA_OCR: LazyLock<Py<PyModule>> = LazyLock::new(init_manga_ocr);

fn init_manga_ocr() -> Py<PyModule> {
    info!("Initializing Manga OCR py03");
    let py_ocr = c_str!(include_str!("ocr.py"));
    let py_run = c_str!(include_str!("run.py"));
    // Initialize Python interpreter
    Python::with_gil(|py| {
        // Create a Python module with the code snippet
        let _ = PyModule::from_code(py, py_ocr, c_str!("ocr.py"), c_str!("ocr"));

        let run_module: Py<PyModule> =
            PyModule::from_code(py, py_run, c_str!("run.py"), c_str!("run"))
                .unwrap()
                .into();

        info!("Initializing Manga OCR py03 done");
        run_module
    })
}

pub fn run_manga_ocr(images: &[Image]) -> Vec<String> {
    let lock = MANGA_OCR.deref();
    Python::with_gil(|py| {
        // Convert the GIL-independent reference into a usable reference scoped to the GIL lock closure
        let module = lock.bind(py);

        // Get the function and call it.
        let function = module.getattr("get_images_ocr").unwrap();

        let args: Vec<&str> = images
            .iter()
            .filter_map(|x| x.get_image_path().ok())
            .collect();
        let args = (args,);

        let bound = function.call1(args).unwrap();
        bound.extract().unwrap()
    })
}
