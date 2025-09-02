use crate::database;
use crate::database::{HistoryData, KanjiStatistic};
use crate::detect::comictextdetector::{DETECT_STATE, combine_overlapping_rects};
use crate::event::event::Event::UpdateImageDisplay;
use crate::event::event::{Event, emit_event};
use crate::jpn::{JpnData, dict, get_jpn_data};
use crate::ocr::OcrBackend::MangaOcr;
use crate::ocr::manga_ocr::get_kanji_top_text;
use crate::ocr::{BackendResult, OcrBackend};
use crate::translation::google::translate;
use crate::ui::id_item::IdItem;
use crate::ui::settings::{Backend, BackendStatus, PreprocessConfig};
use ::serde::{Deserialize, Serialize};
use futures::future::join_all;
use image::{DynamicImage, GenericImage};
use imageproc::contrast::{ThresholdType, otsu_level};
use imageproc::rect::Rect;
use itertools::Itertools;
use log::info;

#[derive(Serialize, Deserialize, PartialEq, Debug, Default, Clone)]
pub struct ScreenshotParameter {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub pipeline: OcrPipeline,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct OcrPipeline(pub Vec<IdItem<OcrPipelineStep>>);

impl Default for OcrPipeline {
    fn default() -> Self {
        let steps = vec![
            OcrPipelineStep::ImageProcessing(PreprocessConfig::default()),
            OcrPipelineStep::BoxDetection { threshold: 0.08 },
            OcrPipelineStep::OcrStep { backend: MangaOcr },
        ];
        OcrPipeline(IdItem::from_vec(steps))
    }
}

pub async fn run_ocr(captured_image: DynamicImage, OcrPipeline(pipeline_steps): OcrPipeline) {
    let width = captured_image.width();
    let height = captured_image.height();
    let mut images = vec![SubImage {
        x: 0,
        y: 0,
        image: captured_image.clone(),
    }];
    show_debug_image(0, "Capture Image".to_string(), &images, width, height);

    for (index, step) in pipeline_steps.iter().enumerate() {
        if step.active {
            images = step
                .item
                .run_ocr_pipeline_step(&captured_image, &images)
                .await;
            show_debug_image(
                index + 1,
                step.item.name().to_string(),
                &images,
                width,
                height,
            );
        } else {
            emit_event(UpdateImageDisplay(
                index + 1,
                step.item.name().to_string(),
                None,
            ));
        }
    }
}

fn show_debug_image(
    index: usize,
    label: String,
    sub_images: &Vec<SubImage>,
    width: u32,
    height: u32,
) {
    if sub_images.is_empty() {
        return;
    }

    let mut image = DynamicImage::new(width, height, sub_images[0].image.color());

    for sub_image in sub_images {
        let dynamic_image = &sub_image.image;

        let _ = image.copy_from(dynamic_image, sub_image.x as u32, sub_image.y as u32);
    }

    emit_event(UpdateImageDisplay(index, label, Some(image)));
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum OcrPipelineStep {
    ImageProcessing(PreprocessConfig),
    BoxDetection { threshold: f32 },
    OcrStep { backend: OcrBackend },
    CutoutCaptureImage,
}

impl OcrPipelineStep {
    pub async fn run_ocr_pipeline_step(
        &self,
        capture_image: &DynamicImage,
        images: &Vec<SubImage>,
    ) -> Vec<SubImage> {
        match self {
            OcrPipelineStep::ImageProcessing(config) => images
                .iter()
                .map(|image| run_image_processing(image, config))
                .collect(),
            OcrPipelineStep::BoxDetection { threshold } => images
                .iter()
                .map(|image| run_box_detection(image, *threshold))
                .flatten()
                .collect(),
            OcrPipelineStep::OcrStep { backend } => run_ocr_step(images, backend).await,
            OcrPipelineStep::CutoutCaptureImage => images
                .iter()
                .map(|SubImage { x, y, image }| {
                    let crop =
                        capture_image.crop_imm(*x as u32, *y as u32, image.width(), image.height());
                    SubImage {
                        x: *x,
                        y: *y,
                        image: crop,
                    }
                })
                .collect(),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            OcrPipelineStep::ImageProcessing(config) => match config {
                PreprocessConfig::SharpenGaussian { .. } => "Sharpen Gaussian",
                PreprocessConfig::Threshold => "Threshold",
            },
            OcrPipelineStep::BoxDetection { .. } => "Box Detection",
            OcrPipelineStep::OcrStep { .. } => "OCR Step",
            OcrPipelineStep::CutoutCaptureImage => "Cutout Capture Image",
        }
    }
}

fn run_image_processing(sub_image: &SubImage, config: &PreprocessConfig) -> SubImage {
    let image = preprocess_image(&sub_image.image, config);
    SubImage {
        x: sub_image.x,
        y: sub_image.y,
        image,
    }
}

fn run_box_detection(sub_image: &SubImage, threshold: f32) -> Vec<SubImage> {
    let image = &sub_image.image;

    let boxes = DETECT_STATE.run_model(image, threshold);
    let boxes = combine_overlapping_rects(boxes);
    boxes
        .iter()
        .map(|x| x.get_rect(image))
        .map(|x| (x, get_cutout_image(image, &x)))
        .map(|(rect, img)| SubImage {
            x: sub_image.x + rect.left(),
            y: sub_image.y + rect.top(),
            image: img,
        })
        .collect()
}

async fn run_ocr_step(images: &Vec<SubImage>, backend: &OcrBackend) -> Vec<SubImage> {
    let images_ref: Vec<&DynamicImage> = images.iter().map(|x| &x.image).collect();

    emit_event(Event::UpdateBackendStatus(
        Backend::MangaOcr,
        BackendStatus::Running,
    ));

    let result: Vec<BackendResult> = backend.run_backend(images_ref).unwrap();

    emit_event(Event::UpdateBackendStatus(
        Backend::MangaOcr,
        BackendStatus::Ready,
    ));

    let result: Vec<(Rect, BackendResult)> = images
        .iter()
        .map(|sub| Rect::at(sub.x, sub.y).of_size(sub.image.width(), sub.image.height()))
        .zip(result)
        .collect();

    let ocr_results = get_ocr_results(result).await;

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

    emit_event(Event::UpdateScreenshotResult(ScreenshotResult {
        ocr_results,
    }));

    images.iter().map(|x| x.clone()).collect()
}

#[derive(PartialEq, Debug, Clone)]
pub struct SubImage {
    pub x: i32,
    pub y: i32,
    pub image: DynamicImage,
}

async fn get_ocr_results(cutout_results: Vec<(Rect, BackendResult)>) -> Vec<ResultData> {
    let mut futures = vec![];

    for (rect, result) in cutout_results {
        let ocr = match &result {
            BackendResult::MangaOcr(top_results) => get_kanji_top_text(&top_results, 0),
        };
        if let Some(x) = ocr {
            futures.push(get_result_data(x, rect, result))
        }
    }

    join_all(futures).await.into_iter().collect()
}

fn preprocess_image(image: &DynamicImage, config: &PreprocessConfig) -> DynamicImage {
    let gray_image = image.grayscale().to_luma8();
    let filtered = match config {
        PreprocessConfig::SharpenGaussian { sigma, amount } => {
            imageproc::filter::sharpen_gaussian(&gray_image, *sigma, *amount)
        }
        PreprocessConfig::Threshold => imageproc::contrast::threshold(
            &gray_image,
            otsu_level(&gray_image),
            ThresholdType::ToZero,
        ),
    };

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
    use crate::action::{OcrPipeline, run_ocr};
    use crate::event::event::{Event, get_events};
    use crate::ocr::BackendResult;
    use crate::ocr::manga_ocr::KanjiConf;
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

        run_ocr(DynamicImage::ImageLuma8(filtered), OcrPipeline::default()).await;

        let event = get_events()
            .into_iter()
            .find(|event| matches!(event, Event::UpdateScreenshotResult(_)))
            .unwrap();
        if let Event::UpdateScreenshotResult(run_ocr) = event {
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
        }
        // dbg!(run_ocr);
    }
}
