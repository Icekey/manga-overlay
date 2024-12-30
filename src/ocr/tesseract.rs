use anyhow::Result;
use rusty_tesseract::Args;
use rusty_tesseract::Image;

use crate::ocr::TesseractParameter;

pub fn run_ocr_tesseract(image: &Image, parameter: &TesseractParameter) -> Result<String> {
    let args = Args {
        lang: parameter.lang.clone(),
        dpi: parameter.dpi,
        ..Args::default()
    };

    let result = rusty_tesseract::image_to_string(image, &args)?;
    Ok(result)
}
