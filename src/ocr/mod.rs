use crate::ocr::manga_ocr::MANGA_OCR;
use anyhow::Result;
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumString};

pub mod manga_ocr;

#[derive(Debug, Clone, PartialEq, strum::Display, EnumString, EnumIter, Serialize, Deserialize)]
pub enum OcrBackend {
    #[strum(ascii_case_insensitive)]
    MangaOcr,
}

impl OcrBackend {
    pub fn run_backends(images: &[DynamicImage], backends: &[OcrBackend]) -> Vec<String> {
        let backend_count: usize = backends.len();

        let backend_outputs: Vec<Vec<String>> = backends
            .iter()
            .map(|e| (e, e.run_ocr(&images)))
            .map(|e| concat_backend_output(e.0, e.1, backend_count))
            .collect();

        let mut output: Vec<String> = vec![];
        for (i, backend_output) in backend_outputs.iter().enumerate() {
            if i == 0 {
                output.clone_from(backend_output);
            } else {
                output = output
                    .into_iter()
                    .zip(backend_output.iter())
                    .map(|x| [x.0, x.1.to_string()].join("\n\n").trim().to_string())
                    .collect();
            }
        }

        output
    }

    pub fn run_ocr(&self, images: &[DynamicImage]) -> Result<Vec<String>> {
        if images.is_empty() {
            return Ok(vec![]);
        }

        match self {
            OcrBackend::MangaOcr => Ok(run_manga_ocr(images)),
        }
    }
}

fn run_manga_ocr(images: &[DynamicImage]) -> Vec<String> {
    let model = MANGA_OCR.lock().unwrap();
    if let Ok(model) = model.as_ref() {
        return model.inference(images).unwrap();
    }
    vec![]
}

fn concat_backend_output(
    backend: &OcrBackend,
    output: Result<Vec<String>>,
    backend_count: usize,
) -> Vec<String> {
    let outputs = output.unwrap_or_else(|e| vec![e.to_string()]);
    outputs
        .into_iter()
        .map(|x| {
            if backend_count > 1 {
                [backend.to_string(), x].join("\n")
            } else {
                x
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use log::info;

    use crate::action::{run_ocr, ResultData, ScreenshotParameter, ScreenshotResult};
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
        let run_ocr: ScreenshotResult = run_ocr(
            ScreenshotParameter {
                detect_boxes: true,
                backends: vec![OcrBackend::MangaOcr],
                ..ScreenshotParameter::default()
            },
            image,
        )
        .await
        .unwrap();

        run_ocr
            .ocr_results
            .iter()
            .zip(expected.iter())
            .for_each(|(a, b)| {
                test_result_data(a, b);
            });
    }

    fn test_result_data(a: &ResultData, b: &ResultData) {
        assert_eq!(a.x, b.x);
        assert_eq!(a.y, b.y);
        assert_eq!(a.w, b.w);
        assert_eq!(a.h, b.h);
        assert_eq!(a.ocr, b.ocr);
    }
}
