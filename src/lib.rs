#![warn(clippy::all, rust_2018_idioms)]
#![allow(
    clippy::must_use_candidate,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::float_cmp
)]
mod ui;

use action::ScreenshotParameter;
pub use ui::app::OcrApp;

use anyhow::{Context, Ok, Result};
use image::{DynamicImage, RgbaImage};
use rusty_tesseract::Args;
use screenshots::Screen;

pub(crate) mod action;
pub(crate) mod database;
pub(crate) mod detect;
pub(crate) mod jpn;
pub(crate) mod ocr;
pub(crate) mod translation;

impl ScreenshotParameter {
    pub fn get_screenshot(&self) -> Result<DynamicImage> {
        let screen = Screen::from_point(self.x, self.y)?;
        let image = screen.capture_area(
            self.x - screen.display_info.x,
            self.y - screen.display_info.y,
            self.width,
            self.height,
        )?;

        let bytes = image.to_vec();
        let image = RgbaImage::from_raw(image.width(), image.height(), bytes)
            .context("screenshot failed")?;

        Ok(DynamicImage::ImageRgba8(image))
    }
}

pub struct OcrParameter {
    pub args: Args,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OcrResult {
    pub ocr: String,
    pub confidence: f32,
    pub rects: Vec<OcrRect>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OcrRect {
    symbol: String,
    top: i32,
    left: i32,
    width: i32,
    height: i32,
}
