use crate::database::{HistoryData, KanjiStatistic};
use crate::detect::comictextdetector::{combine_overlapping_rects, Boxes, DETECT_STATE};
use crate::event::event::{emit_event, Event};
use crate::jpn::{dict, get_jpn_data, JpnData};
use crate::ocr::manga_ocr::get_kanji_top_text;
use crate::ocr::{BackendResult, OcrBackend};
use crate::translation::google::translate;
use crate::ui::image_display::ImageDisplayType::{CAPTURE, DEBUG, PREPROCESSED};
use crate::{database, detect};
use futures::future::join_all;
use image::DynamicImage;
use imageproc::rect::Rect;
use itertools::Itertools;
use log::info;
use open::that;
use ::serde::{Deserialize, Serialize};

pub fn open_workdir() {
    let current_dir = std::env::current_dir().expect("Failed to get current_dir");
    that(current_dir).expect("Failed to open current_dir");
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Default, Clone)]
pub struct ScreenshotParameter {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub detect_boxes: bool,
    pub full_capture_ocr: bool,
    pub backend: OcrBackend,
    pub threshold: f32,
}

pub async fn run_ocr(
    parameter: ScreenshotParameter,
    mut capture_image: DynamicImage,
) -> anyhow::Result<ScreenshotResult> {
    let backend: OcrBackend = parameter.backend;

    //Detect Boxes
    let all_boxes: Vec<Boxes> = if parameter.detect_boxes {
        DETECT_STATE
            .clone()
            .run_model(parameter.threshold, &mut capture_image)
    } else {
        vec![]
    };

    let boxes = combine_overlapping_rects(all_boxes.clone());

    //Run OCR on Boxes
    let mut rects: Vec<Rect> = boxes.iter().map(|x| x.get_rect(&capture_image)).collect();

    if parameter.full_capture_ocr {
        //Add full image rect
        rects.insert(
            0,
            Rect::at(0, 0).of_size(capture_image.width(), capture_image.height()),
        );
    }

    let image = capture_image.clone();

    let cutout_results: Vec<(Rect, BackendResult)> =
        run_ocr_on_cutout_images(&image, &backend, rects)?;

    let mut futures = vec![];

    for (rect, result) in cutout_results {
        let ocr = match &result {
            BackendResult::MangaOcr(top_results) => get_kanji_top_text(&top_results, 1),
        };
        if let Some(x) = ocr {
            futures.push(get_result_data(x, rect, result))
        }
    }

    let ocr_results: Vec<ResultData> = join_all(futures).await.into_iter().collect();

    for ocr_result in &ocr_results {
        //Store OCR
        database::store_ocr(&ocr_result.ocr).expect("Failed to store ocr");

        for jpn_data in ocr_result.jpn.iter().flatten() {
            if jpn_data.has_kanji_data() {
                //Store Kanji statistic
                database::init_kanji_statistic(&jpn_data.get_kanji())
                    .expect("Failed to store kanji");
            }
        }
    }

    //Draw Boxes
    let capture_image = capture_image.clone();
    let mut debug_image = capture_image.clone();
    detect::comictextdetector::draw_rects(&mut debug_image, &all_boxes);

    emit_event(Event::UpdateImageDisplay(CAPTURE, Some(capture_image)));
    emit_event(Event::UpdateImageDisplay(DEBUG, Some(debug_image)));

    Ok(ScreenshotResult { ocr_results })
}

fn run_ocr_on_cutout_images(
    capture_image: &DynamicImage,
    backend: &OcrBackend,
    rects: Vec<Rect>,
) -> anyhow::Result<Vec<(Rect, BackendResult)>> {
    let preprocess_image = preprocess_image(capture_image);

    let cutout_images: Vec<DynamicImage> = rects
        .iter()
        .map(|x| get_cutout_image(&capture_image, x))
        .filter(|x| x.width() != 0 && x.height() != 0)
        .collect();

    let result: Vec<BackendResult> = backend.run_backend(&cutout_images)?;

    let result: Vec<(Rect, BackendResult)> = rects.into_iter().zip(result).collect();

    emit_event(Event::UpdateImageDisplay(
        PREPROCESSED,
        Some(preprocess_image),
    ));

    Ok(result)
}

fn preprocess_image(image: &DynamicImage) -> DynamicImage {
    let filtered = imageproc::filter::sharpen_gaussian(&image.grayscale().to_luma8(), 5., 10.);

    filtered.into()
}

async fn get_result_data(ocr: String, rect: Rect, result: BackendResult) -> ResultData {
    let jpn: Vec<Vec<JpnData>> = get_jpn_data(&ocr).await;

    let translation = match database::load_history_data(&ocr) {
        Ok(x) => x.translation.unwrap_or_default(),
        Err(_) => String::new(),
    };

    ResultData {
        x: rect.left(),
        y: rect.top(),
        w: rect.width() as i32,
        h: rect.height() as i32,
        ocr,
        translation,
        jpn,
        backend_result: Some(result),
    }
}

fn get_cutout_image(capture_image: &DynamicImage, rect: &Rect) -> DynamicImage {
    capture_image.crop_imm(
        rect.left() as u32,
        rect.top() as u32,
        rect.width(),
        rect.height(),
    )
}

#[derive(Deserialize, Serialize, Default, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct ScreenshotResult {
    pub ocr_results: Vec<ResultData>,
}

#[derive(Deserialize, Serialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct ResultData {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub ocr: String,
    pub translation: String,
    pub jpn: Vec<Vec<JpnData>>,
    pub backend_result: Option<BackendResult>,
}

impl std::fmt::Debug for ResultData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResultData")
            .field("x", &self.x)
            .field("y", &self.y)
            .field("w", &self.w)
            .field("h", &self.h)
            .field("ocr", &self.ocr)
            .finish()
    }
}

impl ResultData {
    pub fn get_jpn_data_with_info_count(&self) -> usize {
        self.get_jpn_data_with_info().count()
    }

    pub fn get_jpn_data_with_info_by_index(&self, index: i32) -> Option<&JpnData> {
        let count = self.get_jpn_data_with_info_count() as i32;
        if count == 0 {
            return None;
        }
        self.get_jpn_data_with_info()
            .nth(index.rem_euclid(count) as usize)
    }

    fn get_jpn_data_with_info(&self) -> impl Iterator<Item = &JpnData> {
        self.jpn.iter().flatten().filter(|y| y.has_kanji_data())
    }
}

pub async fn get_translation(input: &str) -> String {
    use std::time::Instant;
    let now = Instant::now();

    info!("Start get_translation");

    let input = input.lines().map(dict::remove_whitespace).join("\n");

    let elapsed = now.elapsed();
    info!("End get_translation elapsed: {elapsed:.2?}");

    let translation = translate(&input)
        .await
        .map_err(|err| err.to_string())
        .unwrap_or_else(|err_string| err_string)
        .trim()
        .to_string();

    database::store_ocr_translation(&input, &translation).expect("Failed to store history data");

    translation
}

pub fn load_history() -> Vec<HistoryData> {
    database::load_full_history().unwrap_or_else(|err| {
        log::error!("Failed to load history: {err}");
        vec![]
    })
}

pub fn increment_kanji_statistic(kanji: &str) -> KanjiStatistic {
    database::increment_kanji_statistic(kanji).expect("Failed to increment kanji statistic")
}

pub(crate) fn load_statistic() -> Vec<KanjiStatistic> {
    database::load_statistic().unwrap_or_else(|err| {
        log::error!("Failed to load statistic: {err}");
        vec![]
    })
}

pub async fn get_kanji_jpn_data(kanji: &str) -> Option<JpnData> {
    let vec = get_jpn_data(kanji).await;
    vec.into_iter().flatten().next()
}

#[cfg(test)]
mod tests {
    use crate::action::{run_ocr, ScreenshotParameter};
    use crate::ocr::manga_ocr::KanjiConf;
    use crate::ocr::{BackendResult, OcrBackend};
    use image::DynamicImage;
    use std::path::Path;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_name() {
        //load DynamicImage
        let path = Path::new("./input/blurry.png");
        dbg!(&path.canonicalize().unwrap());
        let image = image::open(path).expect("Failed to open image");

        // let filtered =
        //     imageproc::filter::bilateral_filter(&image.grayscale().to_luma8(), 5, 5., 5.);

        let filtered = imageproc::filter::sharpen_gaussian(&image.grayscale().to_luma8(), 5., 10.);
        let _ = filtered
            .clone()
            .save(Path::new("./input/blurry_filtered.png"));

        let run_ocr = run_ocr(
            ScreenshotParameter {
                detect_boxes: true,
                threshold: 0.08,
                backend: OcrBackend::MangaOcr,
                ..ScreenshotParameter::default()
            },
            DynamicImage::ImageLuma8(filtered),
        )
        .await
        .unwrap();

        for result in &run_ocr.ocr_results {
            if let Some(result) = &result.backend_result {
                match result {
                    BackendResult::MangaOcr(x) => {
                        for i in 0..x.len() {
                            let option = x.get(i).unwrap();
                            let ocr = KanjiConf::get_ocr(option);

                            dbg!(i, &ocr);
                        }
                    }
                }
            }
        }

        for result in &run_ocr.ocr_results {
            if let Some(result) = &result.backend_result {
                match result {
                    BackendResult::MangaOcr(x) => {
                        dbg!(KanjiConf::get_conf_matrix(&x));
                    }
                }
            }
        }

        // dbg!(run_ocr);
    }
}
