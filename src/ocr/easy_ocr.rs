use std::process::Command;

use anyhow::{Context, Result};
use itertools::Itertools;
use regex::Regex;
use rusty_tesseract::Image;

use crate::ocr::EasyOcrParameter;

pub fn run_ocr_easy_ocr(image: &Image, parameter: &EasyOcrParameter) -> Result<String> {
    let image_path = image.get_image_path()?;

    let lang = match parameter.lang.as_str() {
        x if x.contains("jpn") => "ja",
        _ => "en",
    };

    run_easy_ocr_command(image_path, lang).map(|e| parse_easy_ocr_output(&e))
}

fn run_easy_ocr_command(image_path: &str, lang: &str) -> Result<String> {
    let command = format!(
        "python -X utf8 -m easyocr.cli -l {lang} -f {image_path} --verbose=False"
    );
    dbg!(&command);
    let output = Command::new("cmd").args(["/C", &command]).output()?;
    let result = String::from_utf8(output.stdout)?;
    Ok(result)
}

pub fn parse_easy_ocr_output(output: &str) -> String {
    output
        .lines()
        .filter_map(|x| parse_easy_ocr_line(x).ok())
        .join("\n")
}

fn parse_easy_ocr_line(line: &str) -> Result<String> {
    let re = Regex::new(r#", (\\"|')+(.*)('\\"|')+,"#)?;
    let capture = re.captures(line).context("no regex capture")?;

    let result = capture.get(2).context("Capture group 2 does not exist")?;
    Ok(result.as_str().to_string())
}

#[test]
#[ignore]
fn test_easy_ocr() {
    let output = Command::new("cmd")
        .args([
            "/C",
            "python -X utf8 -m easyocr.cli -l en -f input/japanese.jpg --verbose=False",
        ])
        .output()
        .expect("failed to execute process");

    let output = String::from_utf8(output.stdout).unwrap();
    let parsed = parse_easy_ocr_output(&output);

    assert_eq!(parsed, "#TRTRIl\nNO LITTER\n{xt#nl trEr\nm E MINATO CITY");
}
