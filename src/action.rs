use crate::database::{HistoryData, KanjiStatistic};
use futures::future::join_all;
use image::DynamicImage;
use imageproc::rect::Rect;
use itertools::Itertools;
use log::info;
use open::that;
use ::serde::{Deserialize, Serialize};

use crate::detect::comictextdetector::{combine_overlapping_rects, Boxes, DETECT_STATE};
use crate::jpn::{dict, get_jpn_data, JpnData};
use crate::ocr::OcrBackend;
use crate::translation::google::translate;
use crate::{database, detect};

pub fn open_workdir() {
    let current_dir = std::env::current_dir().expect("Failed to get current_dir");
    that(current_dir).expect("Failed to open current_dir");
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct ScreenshotParameter {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub detect_boxes: bool,
    pub full_capture_ocr: bool,
    pub backends: Vec<OcrBackend>,
    pub threshold: f32,
}

pub async fn run_ocr(
    parameter: ScreenshotParameter,
    mut capture_image: DynamicImage,
) -> Result<ScreenshotResult, ()> {
    let backends: Vec<OcrBackend> = parameter.backends;

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

    let cutout_results = run_ocr_on_cutout_images(&image, &backends, rects);

    let mut futures = vec![];

    for cutout_result in cutout_results {
        futures.push(get_result_data(cutout_result.0, cutout_result.1));
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
    let mut capture_image = capture_image.clone();
    let mut debug_image = capture_image.clone();
    detect::comictextdetector::draw_rects(&mut capture_image, &boxes);
    detect::comictextdetector::draw_rects(&mut debug_image, &all_boxes);

    Ok(ScreenshotResult {
        capture_image: Some(capture_image),
        debug_image: Some(debug_image),
        ocr_results,
    })
}

fn run_ocr_on_cutout_images(
    capture_image: &DynamicImage,
    backends: &Vec<OcrBackend>,
    rects: Vec<Rect>,
) -> Vec<(String, Rect)> {
    let cutout_images: Vec<DynamicImage> = rects
        .iter()
        .map(|x| get_cutout_image(&capture_image, &x))
        .filter(|x| x.width() != 0 && x.height() != 0)
        .collect();

    OcrBackend::run_backends(&cutout_images, &backends)
        .unwrap_or_else(|e| vec![e.to_string(); rects.len()])
        .into_iter()
        .zip(rects.into_iter())
        .collect()
}

async fn get_result_data(ocr: String, rect: Rect) -> ResultData {
    let jpn: Vec<Vec<JpnData>> = get_jpn_data(&ocr).await;

    let translation = match database::load_history_data(&ocr) {
        Ok(x) => x.translation.unwrap_or_default(),
        Err(_) => "".to_string(),
    };

    ResultData {
        x: rect.left(),
        y: rect.top(),
        w: rect.width() as i32,
        h: rect.height() as i32,
        ocr,
        translation,
        jpn,
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

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
#[serde(default)]
pub struct ScreenshotResult {
    #[serde(skip)]
    pub capture_image: Option<DynamicImage>,
    #[serde(skip)]
    pub debug_image: Option<DynamicImage>,
    pub ocr_results: Vec<ResultData>,
}

#[derive(Deserialize, Serialize, Default, Clone)]
#[serde(default)]
pub struct ResultData {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub ocr: String,
    pub translation: String,
    pub jpn: Vec<Vec<JpnData>>,
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
    pub fn get_jpn_data_with_info_count(&self) -> i32 {
        self.get_jpn_data_with_info().count() as i32
    }

    pub fn get_jpn_data_with_info_by_index(&self, index: i32) -> Option<&JpnData> {
        let count = self.get_jpn_data_with_info_count();
        if count == 0 {
            return None;
        }
        self.get_jpn_data_with_info()
            .nth((index.rem_euclid(count)) as usize)
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

pub async fn load_history() -> Vec<HistoryData> {
    return database::load_full_history().unwrap_or_else(|err| {
        log::error!("Failed to load history: {err}");
        vec![]
    });
}

pub async fn increment_kanji_statistic(kanji: String) -> KanjiStatistic {
    database::increment_kanji_statistic(&kanji).expect("Failed to increment kanji statistic")
}

pub(crate) async fn load_statistic() -> Vec<KanjiStatistic> {
    return database::load_statistic().unwrap_or_else(|err| {
        log::error!("Failed to load statistic: {err}");
        vec![]
    });
}

pub async fn get_kanji_jpn_data(kanji: &str) -> Option<JpnData> {
    let vec = get_jpn_data(kanji).await;
    vec.into_iter().flatten().next()
}

#[cfg(test)]
mod tests {

    #[tokio::test(flavor = "multi_thread")]
    async fn test_name() {
        //load DynamicImage
        // let image = image::open("../input/input.jpg").expect("Failed to open image");
        // let run_ocr = run_ocr(
        //     ScreenshotParameter {
        //         detect_boxes: true,
        //         backends: vec![OcrBackend::MangaOcr],
        //         ..ScreenshotParameter::default()
        //     },
        //     image,
        // )
        // .await;

        // dbg!(run_ocr);
        assert!(true);
    }
}
