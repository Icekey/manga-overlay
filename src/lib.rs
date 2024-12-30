#![warn(clippy::all, rust_2018_idioms)]
mod ui;

pub use ui::app::OcrApp;

use anyhow::*;
use clap::Parser;
use image::{DynamicImage, RgbaImage};
use rusty_tesseract::Args;
use screenshots::Screen;

pub mod action;
pub mod database;
pub mod detect;
pub mod jpn;
pub mod ocr;
pub mod translation;

#[derive(Parser, Default, Debug)]
pub struct CaptureParameter {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl CaptureParameter {
    pub fn get_screenshot(&self) -> Result<DynamicImage> {
        let screen = Screen::from_point(self.x, self.y)?;
        let image = screen.capture_area(
            self.x - screen.display_info.x,
            self.y - screen.display_info.y,
            self.width,
            self.height,
        )?;

        let bytes = image.to_vec();
        let image = RgbaImage::from_raw(self.width, self.height, bytes).unwrap();

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
