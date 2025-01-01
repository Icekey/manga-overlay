use crate::database::{HistoryData, KanjiStatistic};
use ::serde::{Deserialize, Serialize};
use futures::future::join_all;
use image::DynamicImage;
use imageproc::rect::Rect;
use itertools::Itertools;
use log::info;
use open::that;

use crate::detect::comictextdetector::{Boxes, DETECT_STATE};
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
}

pub async fn run_ocr(
    parameter: ScreenshotParameter,
    mut capture_image: DynamicImage,
) -> Result<ScreenshotResult, ()> {
    let backends: Vec<OcrBackend> = parameter.backends;

    //Detect Boxes
    let boxes: Vec<Boxes> = if parameter.detect_boxes {
        DETECT_STATE.clone().run_model(0.5, &mut capture_image)
    } else {
        vec![]
    };

    //Run OCR on Boxes
    let mut rects: Vec<Rect> = boxes.iter().map(|x| x.get_rect(&capture_image)).collect();

    if rects.is_empty() || parameter.full_capture_ocr {
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
    detect::comictextdetector::draw_rects(&mut capture_image, &boxes);

    Ok(ScreenshotResult {
        capture_image: Some(capture_image),
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

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ScreenshotResult {
    #[serde(skip)]
    pub capture_image: Option<DynamicImage>,
    pub ocr_results: Vec<ResultData>,
}

#[derive(Deserialize, Serialize, Default)]
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

async fn get_translation(input: String) -> String {
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

#[derive(Serialize, serde::Deserialize)]
pub struct MousePosition {
    x: i32,
    y: i32,
}

async fn load_history() -> Vec<HistoryData> {
    return database::load_full_history().unwrap_or_else(|err| {
        log::error!("Failed to load history: {err}");
        vec![]
    });
}

async fn increment_kanji_statistic(kanji: String) -> KanjiStatistic {
    database::increment_kanji_statistic(&kanji).expect("Failed to increment kanji statistic")
}

async fn load_statistic() -> Vec<KanjiStatistic> {
    return database::load_statistic().unwrap_or_else(|err| {
        log::error!("Failed to load statistic: {err}");
        vec![]
    });
}

async fn get_kanji_jpn_data(kanji: String) -> Option<JpnData> {
    let vec = get_jpn_data(&kanji).await;
    vec.into_iter().flatten().next()
}
