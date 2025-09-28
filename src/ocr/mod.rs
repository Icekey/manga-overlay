use crate::ocr::manga_ocr::{KanjiTopResults, MANGA_OCR};
use anyhow::{Result, bail};
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumString};

pub mod manga_ocr;

#[derive(
    Debug, Clone, PartialEq, strum::Display, EnumString, EnumIter, Serialize, Deserialize, Default,
)]
pub enum OcrBackend {
    #[strum(ascii_case_insensitive)]
    #[default]
    MangaOcr,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Default)]
pub enum BackendResult {
    MangaOcr(KanjiTopResults),
    #[default]
    Unknown,
}

impl OcrBackend {
    pub fn run_backend(&self, images: Vec<&DynamicImage>) -> Result<Vec<BackendResult>> {
        match self {
            OcrBackend::MangaOcr => run_manga_ocr(images),
        }
    }
}

fn run_manga_ocr(images: Vec<&DynamicImage>) -> Result<Vec<BackendResult>> {
    let mut model = MANGA_OCR.lock().expect("Manga OCR lock failed");
    if let Ok(model) = model.as_mut() {
        let result = model.inference(images);
        let result = result.into_iter().map(BackendResult::MangaOcr).collect();

        return Ok(result);
    }
    bail!("MangaOCR model not found")
}

#[cfg(test)]
mod tests {
    use log::info;

    use crate::action::{OcrPipeline, ResultData, run_ocr};
    use crate::event::event::{Event, get_events};
    use crate::ocr::OcrBackend;
    use crate::ocr::OcrBackend::MangaOcr;

    #[test]
    fn ocr_backend_serialize() {
        let backends: Vec<OcrBackend> = vec![MangaOcr];

        let json = serde_json::to_string(&backends).unwrap();
        info!("json: {}", json);
        assert_eq!(json, r#"["MangaOcr"]"#);

        let result: Vec<OcrBackend> = serde_json::from_str(&json).unwrap();
        info!("parsed: {:?}", result);
        assert_eq!(backends, result);
    }

    #[tokio::test]
    async fn test_detect_boxes_and_manga_ocr() {
        let expected = vec![
            ResultData {
                x: 565,
                y: 159,
                w: 96,
                h: 131,
                ocr: "今年はいいことがありそうだ。".to_string(),
                ..Default::default()
            },
            ResultData {
                x: 749,
                y: 205,
                w: 63,
                h: 155,
                ocr: "のどかなお正月だなあ。".to_string(),
                ..Default::default()
            },
            ResultData {
                x: 758,
                y: 711,
                w: 94,
                h: 92,
                ocr: "四十分後火あぶりなる。".to_string(),
                ..Default::default()
            },
            ResultData {
                x: 121,
                y: 717,
                w: 67,
                h: 84,
                ocr: "出てこいつ。".to_string(),
                ..Default::default()
            },
            ResultData {
                x: 437,
                y: 727,
                w: 83,
                h: 75,
                ocr: "だれだへんないうや".to_string(),
                ..Default::default()
            },
            ResultData {
                x: 100,
                y: 102,
                w: 111,
                h: 81,
                ocr: "いやあ、ろくなことがないね。".to_string(),
                ..Default::default()
            },
            ResultData {
                x: 60,
                y: 403,
                w: 130,
                h: 124,
                ocr: "野比のび太は三十分後に道をつる。".to_string(),
                ..Default::default()
            },
        ];

        run_test(&expected).await;
    }

    async fn run_test(expected: &[ResultData]) {
        let image = image::open("input/input.jpg").expect("Failed to open image");
        run_ocr(image, OcrPipeline::default()).await;

        let event = get_events()
            .into_iter()
            .find(|event| matches!(event, Event::UpdateScreenshotResult(_)))
            .unwrap();
        if let Event::UpdateScreenshotResult(run_ocr) = event {
            run_ocr
                .ocr_results
                .iter()
                .zip(expected.iter())
                .for_each(|(a, b)| {
                    test_result_data(a, b);
                });
        }
    }

    fn test_result_data(a: &ResultData, b: &ResultData) {
        assert_eq!(a.x, b.x);
        assert_eq!(a.y, b.y);
        assert_eq!(a.w, b.w);
        assert_eq!(a.h, b.h);
        assert_eq!(a.ocr, b.ocr);
    }
}
