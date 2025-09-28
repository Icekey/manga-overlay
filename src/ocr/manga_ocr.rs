use crate::detect::session_builder::create_session_builder;
use hf_hub::api::sync::Api;
use image::DynamicImage;
use itertools::Itertools;
use ndarray::{Array3, Array4, ArrayBase, Axis, Dim, Ix, OwnedRepr, s, stack};
use ort::value::TensorRef;
use ort::{inputs, session::Session};
use serde::{Deserialize, Serialize};
use std::ops::{Div, Sub};
use std::sync::{Arc, LazyLock, Mutex};

type MangaOcrState = Arc<Mutex<anyhow::Result<MangaOCR>>>;

pub static MANGA_OCR: LazyLock<MangaOcrState> =
    LazyLock::new(|| Arc::new(Mutex::new(MangaOCR::new())));

#[derive(Debug)]
pub struct MangaOCR {
    model: Session,
    vocab: Vec<String>,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug, PartialEq)]
pub struct KanjiConf {
    pub kanji: String,
    pub confidence: f32,
}

impl KanjiConf {
    pub fn get_ocr(vec: &[Self]) -> String {
        vec.iter().map(|x| x.kanji.clone()).join("")
    }

    pub fn get_conf_matrix(vec: &Vec<Vec<Self>>) -> String {
        vec.iter()
            .map(|x| {
                x.iter()
                    .map(
                        |KanjiConf {
                             kanji,
                             confidence: _,
                         }| format!("{kanji}"),
                    )
                    .join("")
            })
            .join(" | ")
    }
}

#[derive(Deserialize, Serialize, Default, Clone, Debug, PartialEq)]
pub struct TokenConf {
    pub token_id: i64,
    pub confidence: f32,
}

impl TokenConf {
    fn convert(&self, vocab: &Vec<String>) -> Option<KanjiConf> {
        let kanji = vocab.get(self.token_id as usize).cloned()?;

        if kanji.is_empty() {
            return None;
        }

        Some(KanjiConf {
            kanji,
            confidence: self.confidence,
        })
    }
}

//Batchsize, Sequenz, Top 10
pub type TokenConfVec = Vec<Vec<Vec<TokenConf>>>;

pub type KanjiResult = Vec<KanjiConf>;

pub type KanjiTopResults = Vec<KanjiResult>;

pub fn get_kanji_top_text(result: &KanjiTopResults, top: usize) -> Option<String> {
    let ocr = result
        .iter()
        .flat_map(|x| x.get(top))
        .map(|x| x.kanji.clone())
        .join("");
    Some(ocr)
}

const MAX_TOP_KANJI: usize = 20;

impl MangaOCR {
    pub fn new() -> anyhow::Result<Self> {
        let api = Api::new()?;
        let repo = api.model("mayocream/koharu".to_string());
        let model_path = repo.get("manga-ocr.onnx")?;
        let vocab_path = repo.get("vocab.txt")?;

        let builder = create_session_builder()?;

        let model = builder.commit_from_file(model_path)?;

        let vocab = std::fs::read_to_string(vocab_path)
            .map_err(|e| anyhow::anyhow!("Failed to read vocab file: {e}"))?
            .lines()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        Ok(Self { model, vocab })
    }

    pub fn inference(&mut self, images: Vec<&DynamicImage>) -> Vec<KanjiTopResults> {
        if images.is_empty() {
            return vec![];
        }

        let batch_size = images.len();
        let tensor = Self::create_image_tensor(images);

        let token_ids = self.get_token_ids(batch_size, tensor).unwrap_or_default();

        self.decode_tokens(&token_ids)
    }

    fn decode_tokens(&self, token_ids: &TokenConfVec) -> Vec<KanjiTopResults> {
        token_ids
            .iter()
            .map(|outer| self.get_kanji_top_results(outer))
            .filter(|x: &KanjiTopResults| !x.is_empty())
            .collect()
    }

    fn get_kanji_top_results(&self, outer: &Vec<Vec<TokenConf>>) -> KanjiTopResults {
        outer
            .iter()
            .map(|middle| self.convert(middle))
            .filter(|x: &KanjiResult| !x.is_empty())
            .collect()
    }

    fn convert(&self, middle: &[TokenConf]) -> KanjiResult {
        middle
            .iter()
            .filter(|token| token.token_id >= 5)
            .filter_map(|token| token.convert(&self.vocab))
            .collect()
    }

    fn get_token_ids(
        &mut self,
        batch_size: usize,
        tensor: ArrayBase<OwnedRepr<f32>, Dim<[Ix; 4]>>,
    ) -> anyhow::Result<TokenConfVec> {
        let mut done_state: Vec<bool> = vec![false; batch_size];
        let mut token_ids: Vec<Vec<i64>> = vec![vec![2i64]; batch_size]; // Start token

        let mut token_confs: TokenConfVec = vec![Vec::new(); batch_size];

        'outer: for run in 0..300 {
            // Create input tensors
            let input_token_ids = token_ids.iter().flatten().cloned().collect();
            let input =
                ndarray::Array::from_shape_vec((batch_size, token_ids[0].len()), input_token_ids)
                    .expect("Input Shape is invalid");
            let inputs = inputs! {
                "image" => TensorRef::from_array_view(&tensor)?,
                "token_ids" => TensorRef::from_array_view(&input)?,
            };

            // Run inference
            let outputs = self.model.run(inputs)?;

            // Extract logits from output
            let logits = outputs["logits"].try_extract_array::<f32>()?;

            // Get last token logits and find argmax
            let logits_view = logits.view();

            for i in 0..batch_size {
                if done_state[i] {
                    token_ids[i].push(3);
                    continue;
                }

                let last_token_logits = logits_view.slice(s![i, -1, ..]);

                let top_ten: Vec<TokenConf> = last_token_logits
                    .iter()
                    .enumerate()
                    .map(|(id, &conf)| TokenConf {
                        token_id: id as i64,
                        confidence: conf,
                    })
                    .sorted_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap())
                    .take(MAX_TOP_KANJI)
                    .collect();

                let token_id = top_ten[0].token_id;

                token_ids[i].push(token_id);

                if run > 0 {
                    token_confs[i].push(top_ten);
                }

                // Break if end token
                if token_id == 3 {
                    done_state[i] = true;
                }
            }

            if done_state.iter().all(|&x| x) {
                token_confs.iter_mut().for_each(|x| {
                    x.pop();
                });
                break 'outer;
            }
        }
        Ok(token_confs)
    }

    fn create_image_tensor(images: Vec<&DynamicImage>) -> Array4<f32> {
        let arrays = images
            .iter()
            .map(|x| Self::fast_image_to_ndarray(x))
            .collect_vec();
        let stack = Self::join_arrays_stack(&arrays);

        stack
    }

    fn fast_image_to_ndarray(img: &DynamicImage) -> Array3<f32> {
        let img = img.grayscale().to_rgb8();
        let img = image::imageops::resize(&img, 224, 224, image::imageops::FilterType::Lanczos3);

        let (width, height) = img.dimensions();
        let raw_buf = img.as_raw();

        let array = Array3::from_shape_vec(
            (height as usize, width as usize, 3),
            raw_buf.iter().map(|&x| x as f32).collect(),
        )
        .unwrap()
        .div(255.0)
        .sub(0.5)
        .div(0.5);

        array
    }

    fn join_arrays_stack(arrays: &[Array3<f32>]) -> Array4<f32> {
        let views: Vec<_> = arrays
            .iter()
            .map(|a| a.view().permuted_axes([2, 0, 1]))
            .collect();
        stack(Axis(0), &views).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test() {
        let res_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let input_path = res_dir.join("input").join("input_rect.jpg");
        let original_img = image::open(input_path.as_path()).unwrap();
        let images = vec![&original_img];

        let mut model = MANGA_OCR.lock().unwrap();
        if let Ok(model) = model.as_mut() {
            let result = model.inference(images);

            for data in result.iter() {
                for i in 0..10 {
                    let text = get_kanji_top_text_with_conf(data, i)
                        .unwrap_or("<no_kanji_top_text>".to_string());
                    dbg!("{}: {:#?}", i, text);
                }
            }
        }
    }

    pub fn get_kanji_top_text_with_conf(result: &KanjiTopResults, top: usize) -> Option<String> {
        let ocr = result
            .iter()
            .flat_map(|x| x.get(top))
            .map(|x| format!("{}({:.2})", x.kanji.clone(), x.confidence))
            .join("");
        Some(ocr)
    }
}
