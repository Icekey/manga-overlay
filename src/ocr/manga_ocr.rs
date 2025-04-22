use crate::detect::session_builder::create_session_builder;
use hf_hub::api::sync::Api;
use image::DynamicImage;
use itertools::Itertools;
use ndarray::{s, stack, Array3, Array4, ArrayBase, Axis, Dim, Ix, OwnedRepr};
use ort::{inputs, session::Session};
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

    pub fn inference(&self, images: &[DynamicImage]) -> anyhow::Result<Vec<String>> {
        if images.is_empty() {
            return Ok(vec![]);
        }

        let batch_size = images.len();
        let tensor = Self::create_image_tensor(images);

        let token_ids = self.get_token_ids(batch_size, tensor)?;

        let texts = token_ids.iter().map(|x| self.decode_tokens(x)).collect();
        Ok(texts)
    }


    fn decode_tokens(&self, token_ids: &Vec<i64>) -> String {
        let text = token_ids
            .iter()
            .filter(|&&id| id >= 5)
            .filter_map(|&id| self.vocab.get(id as usize).cloned())
            .collect::<Vec<_>>();

        text.join("")
    }

    fn get_token_ids(
        &self,
        batch_size: usize,
        tensor: ArrayBase<OwnedRepr<f32>, Dim<[Ix; 4]>>,
    ) -> anyhow::Result<Vec<Vec<i64>>> {
        let mut done_state: Vec<bool> = vec![false; batch_size];
        let mut token_ids: Vec<Vec<i64>> = vec![vec![2i64]; batch_size]; // Start token

        'outer: for _ in 0..300 {
            // Create input tensors
            let input = ndarray::Array::from_shape_vec(
                (batch_size, token_ids[0].len()),
                token_ids.iter().flatten().cloned().collect(),
            )?;
            let inputs = inputs! {
                "image" => tensor.view(),
                "token_ids" => input,
            }?;

            // Run inference
            let outputs = self.model.run(inputs)?;

            // Extract logits from output
            let logits = outputs["logits"].try_extract_tensor::<f32>()?;

            // Get last token logits and find argmax
            let logits_view = logits.view();

            for i in 0..batch_size {
                if done_state[i] {
                    token_ids[i].push(3);
                    continue;
                }

                let last_token_logits = logits_view.slice(s![i, -1, ..]);

                let (token_id, _) = last_token_logits
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                    .unwrap_or((0, &0.0));

                token_ids[i].push(token_id as i64);

                // Break if end token
                if token_id as i64 == 3 {
                    done_state[i] = true;

                    if done_state.iter().all(|&x| x) {
                        break 'outer;
                    }
                }
            }
        }
        Ok(token_ids)
    }

    fn create_image_tensor(images: &[DynamicImage]) -> Array4<f32> {
        let arrays = images.iter().map(|x| Self::fast_image_to_ndarray(x)).collect_vec();
        let stack = Self::join_arrays_stack(&arrays);

        stack
    }

    fn fast_image_to_ndarray(img: &DynamicImage) -> Array3<f32> {
        let img = img.grayscale().to_rgb8();
        let img = image::imageops::resize(&img, 224, 224, image::imageops::FilterType::Lanczos3);

        let (width, height) = img.dimensions();
        let raw_buf = img.as_raw();

        let array = Array3::from_shape_vec((height as usize, width as usize, 3),
                                           raw_buf.iter().map(|&x| x as f32).collect())
            .unwrap().div(255.0).sub(0.5).div(0.5);

        array
    }

    fn join_arrays_stack(arrays: &[Array3<f32>]) -> Array4<f32> {
        let views: Vec<_> = arrays.iter().map(|a| a.view().permuted_axes([2, 0, 1])).collect();
        stack(Axis(0), &views).unwrap()
    }
}