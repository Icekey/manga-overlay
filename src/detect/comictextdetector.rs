use std::sync::{Arc, LazyLock, Mutex};

use anyhow::Result;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, Rgba};
use imageproc::drawing::draw_hollow_rect_mut;
use imageproc::rect::Rect;
use log::{error, info, warn};
use ndarray::Array4;
use ort::execution_providers::{CUDAExecutionProvider, ExecutionProvider};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;

const INPUT_WIDTH: f32 = 1024.0;
const INPUT_HEIGHT: f32 = 1024.0;

pub static DETECT_STATE: LazyLock<DetectState> = LazyLock::new(DetectState::init);

#[derive(Clone)]
pub struct DetectState {
    pub session: Arc<Mutex<Option<Session>>>,
}

impl DetectState {
    pub fn init() -> Self {
        let data = load_model().ok();
        let data = Mutex::new(data);
        let session = Arc::new(data);

        Self { session }
    }

    pub fn run_model(&self, threshold: f32, img: &mut DynamicImage) -> Vec<Boxes> {
        let model = self.session.lock().unwrap();
        if let Some(model) = model.as_ref() {
            run_model(model, threshold, img).unwrap_or_else(|e| {
                error!("run_model error: {}", e);
                vec![]
            })
        } else {
            vec![]
        }
    }
}

pub fn load_model() -> Result<Session> {
    let mut builder = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?;

    let cuda = CUDAExecutionProvider::default();
    if cuda.is_available()? {
        info!("CUDA is available");
    } else {
        warn!("CUDA is not available");
    }

    let result = cuda.register(&mut builder);
    if result.is_err() {
        warn!("Failed to register CUDA! {}", result.unwrap_err());
    } else {
        info!("Registered CUDA");
    }

    let detector_model = include_bytes!("../../resources/comictextdetector_blk.pt.onnx");

    let session = builder.commit_from_memory(detector_model)?;
    Ok(session)
}

pub fn detect_boxes(model: &Session, original_img: &DynamicImage) -> Result<Vec<Boxes>> {
    let mut input = Array4::<f32>::zeros((1, 3, INPUT_WIDTH as usize, INPUT_HEIGHT as usize));

    let img = original_img.resize_exact(
        INPUT_WIDTH as u32,
        INPUT_HEIGHT as u32,
        FilterType::CatmullRom,
    );

    for pixel in img.pixels() {
        let x = pixel.0 as _;
        let y = pixel.1 as _;
        let [r, g, b, _] = pixel.2 .0;
        input[[0, 0, y, x]] = f32::from(r) / 255.;
        input[[0, 1, y, x]] = f32::from(g) / 255.;
        input[[0, 2, y, x]] = f32::from(b) / 255.;
    }

    // let outputs: SessionOutputs = model.run(ort::inputs!["images" => input.view()]?)?;
    let outputs = model.run(ort::inputs![input]?)?;

    let output_blk = outputs.get("blk").unwrap().try_extract_tensor::<f32>()?;

    let rows = output_blk
        .view()
        .axis_iter(ndarray::Axis(1))
        .map(|row| Boxes::new(row.iter().copied().collect()))
        .collect();

    Ok(rows)
}

#[derive(Clone, Debug)]
pub struct Boxes {
    confidence: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Boxes {
    fn new(row: Vec<f32>) -> Self {
        let x = (row[0] / INPUT_WIDTH).max(0.0);
        let y = (row[1] / INPUT_HEIGHT).max(0.0);
        let w = (row[2] / INPUT_WIDTH).max(0.0);
        let h = (row[3] / INPUT_HEIGHT).max(0.0);

        let confidence = row[4];

        Self {
            confidence,
            x,
            y,
            w,
            h,
        }
    }

    fn get_top(&self) -> f32 {
        (self.y - self.h / 2.0).max(0.0)
    }

    fn get_bottom(&self) -> f32 {
        self.y + self.h / 2.0
    }

    fn get_left(&self) -> f32 {
        (self.x - self.w / 2.0).max(0.0)
    }

    fn get_right(&self) -> f32 {
        self.x + self.w / 2.0
    }

    fn overlaps(&self, other: &Boxes) -> bool {
        // if rectangle has area 0, no overlap
        if self.get_left() == self.get_right()
            || self.get_top() == self.get_bottom()
            || other.get_left() == other.get_right()
            || other.get_top() == other.get_bottom()
        {
            return false;
        }

        // If one rectangle is on left side of other
        if self.get_left() >= other.get_right() || other.get_left() >= self.get_right() {
            return false;
        }

        // If one rectangle is above other
        if self.get_top() >= other.get_bottom() || other.get_top() >= self.get_bottom() {
            return false;
        }

        true

        // Implement the logic to check if two boxes overlap
    }

    fn merge(&self, other: &Boxes) -> Boxes {
        // Implement the logic to merge two overlapping boxes into a combined box
        let min_left = self.get_left().min(other.get_left());
        let min_top = self.get_top().min(other.get_top());
        let max_right = self.get_right().max(other.get_right());
        let max_bottom = self.get_bottom().max(other.get_bottom());

        Boxes {
            confidence: (self.confidence + other.confidence) / 2.0,
            x: min_left + (max_right - min_left) / 2.0,
            y: min_top + (max_bottom - min_top) / 2.0,
            w: max_right - min_left,
            h: max_bottom - min_top,
        }
    }

    pub fn get_rect(&self, img: &DynamicImage) -> Rect {
        let img_width = img.width() as f32;
        let img_height = img.height() as f32;

        let x = self.get_left() * img_width;
        let y = self.get_top() * img_height;
        let width = self.w * img_width;
        let height = self.h * img_height;
        Rect::at(x as i32, y as i32).of_size(width as u32, height as u32)
    }
}

pub fn combine_overlapping_rects(boxes: Vec<Boxes>) -> Vec<Boxes> {
    let mut combined_boxes: Vec<Boxes> = vec![];

    for next_box in boxes {
        let mut overlapped = false;
        for aggregate_box in &mut combined_boxes {
            if next_box.overlaps(aggregate_box) {
                *aggregate_box = aggregate_box.merge(&next_box);
                overlapped = true;
            }
        }
        if !overlapped {
            combined_boxes.push(next_box);
        }
    }

    combined_boxes
}

pub fn run_model(model: &Session, threshold: f32, img: &mut DynamicImage) -> Result<Vec<Boxes>> {
    info!("detect_boxes...");
    let mut boxes = detect_boxes(model, img)?;

    boxes.retain(|x| x.confidence > threshold);
    info!("detect_boxes done with {}", boxes.len());
    Ok(boxes)
}

pub fn draw_rects(img: &mut DynamicImage, boxes: &[Boxes]) {
    let red = Rgba([255, 0, 0, 255]);

    for row in boxes {
        let rect = row.get_rect(img);
        draw_hollow_rect_mut(img, rect, red);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn test_load() {
        let model = load_model().unwrap();
        info!("Model loaded");

        vec![0.0, 0.01, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]
            .into_iter()
            .enumerate()
            .for_each(|(i, conf)| {
                info!("Run {}", i);
                let res_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
                let output = res_dir
                    .join("output")
                    .join(format!("output_{conf:.2}.jpg"));
                let input_path = res_dir.join("input").join("input.jpg");
                let mut original_img = image::open(input_path.as_path()).unwrap();

                let boxes = run_model(&model, conf, &mut original_img).unwrap();

                draw_rects(&mut original_img, &boxes);

                let _ = original_img.save(&output);
            });
    }
}
