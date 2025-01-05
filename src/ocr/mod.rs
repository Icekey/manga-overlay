use std::sync::{Arc, LazyLock, Mutex};

use anyhow::{bail, Result};
use image::DynamicImage;
use rusty_tesseract::Image;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumString};

use crate::ocr::manga_ocr::MangaOcrInstance;

mod easy_ocr;
mod manga_ocr;
mod tesseract;

pub static OCR_STATE: LazyLock<OcrState> = LazyLock::new(OcrState::init);

#[derive(Clone)]
pub struct OcrState {
    pub manga_ocr: Arc<Mutex<Option<MangaOcrInstance>>>,
}

impl OcrState {
    pub fn init() -> Self {
        let data = MangaOcrInstance::init().ok();
        let data = Mutex::new(data);
        let manga_ocr = Arc::new(data);

        Self { manga_ocr }
    }
}

#[derive(Debug, Clone, PartialEq, strum::Display, EnumString, EnumIter, Serialize, Deserialize)]
pub enum OcrBackend {
    #[strum(ascii_case_insensitive)]
    Tesseract(TesseractParameter),
    #[strum(ascii_case_insensitive)]
    EasyOcr(EasyOcrParameter),
    #[strum(ascii_case_insensitive)]
    MangaOcr,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TesseractParameter {
    pub lang: String,
    pub dpi: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EasyOcrParameter {
    pub lang: String,
}

impl OcrBackend {
    pub fn run_backends(
        images: &Vec<DynamicImage>,
        backends: &Vec<OcrBackend>,
    ) -> Result<Vec<String>> {
        let image: Vec<Image> = images
            .iter()
            .filter_map(|x| Image::from_dynamic_image(x).ok())
            .collect();
        let backend_count: usize = backends.len();

        let backend_outputs: Vec<Vec<String>> = backends
            .into_iter()
            .map(|e| (e.to_string(), e.run_ocr(&image)))
            .map(|e| concat_backend_output(e.0, e.1, backend_count))
            .collect();

        let mut output: Vec<String> = vec![];
        for i in 0..backend_outputs.len() {
            if i == 0 {
                output = backend_outputs.get(i).unwrap().clone();
            } else {
                output = output
                    .into_iter()
                    .zip(backend_outputs.get(i).unwrap().iter())
                    .map(|x| [x.0, x.1.to_string()].join("\n\n").trim().to_string())
                    .collect();
            }
        }

        Ok(output)
    }

    pub fn run_ocr(&self, images: &Vec<Image>) -> Result<Vec<String>> {
        log::info!("run_ocr {}", images.len());
        return match self {
            OcrBackend::Tesseract(x) => images
                .iter()
                .map(|image| tesseract::run_ocr_tesseract(&image, x))
                .collect(),
            OcrBackend::EasyOcr(x) => images
                .iter()
                .map(|image| easy_ocr::run_ocr_easy_ocr(&image, x))
                .collect(),
            OcrBackend::MangaOcr => Self::run_manga_ocr(images),
        };
    }

    fn run_manga_ocr(images: &Vec<Image>) -> Result<Vec<String>> {
        if let Some(x) = OCR_STATE.clone().manga_ocr.lock().unwrap().as_mut() {
            manga_ocr::run_manga_ocr(images, x)
        } else {
            bail!("No MangaOcrInstance")
        }
    }
}

fn concat_backend_output(
    backend: String,
    output: Result<Vec<String>>,
    backend_count: usize,
) -> Vec<String> {
    let outputs = output.unwrap_or_else(|e| vec![e.to_string()]);
    outputs
        .into_iter()
        .map(|x| {
            if backend_count > 1 {
                [backend.clone(), x].join("\n")
            } else {
                x
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use log::info;

    use crate::ocr::OcrBackend::{EasyOcr, MangaOcr, Tesseract};
    use crate::ocr::{EasyOcrParameter, OcrBackend, TesseractParameter};

    #[test]
    fn ocr_backend_serialize() {
        let backends: Vec<OcrBackend> = vec![
            Tesseract(TesseractParameter {
                lang: "jpn".into(),
                dpi: 200,
            }),
            Tesseract(TesseractParameter {
                lang: "eng".into(),
                dpi: 0,
            }),
            EasyOcr(EasyOcrParameter { lang: "eng".into() }),
            MangaOcr,
        ];

        let json = serde_json::to_string(&backends).unwrap();
        info!("json: {}", json);
        assert_eq!(
            json,
            r#"[{"Tesseract":{"lang":"jpn","dpi":200}},{"Tesseract":{"lang":"eng","dpi":null}},{"EasyOcr":{"lang":"eng"}},"MangaOcr"]"#
        );

        let result: Vec<OcrBackend> = serde_json::from_str(&json).unwrap();
        info!("parsed: {:?}", result);
        assert_eq!(backends, result);
    }
}
