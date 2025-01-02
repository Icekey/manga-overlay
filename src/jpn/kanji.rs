use std::collections::HashMap;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DefaultOnNull};

lazy_static! {
    pub static ref KANJI_MAP: HashMap<char, KanjiData> =
        get_map_from_json(include_str!("kanji.json"));
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug, Default)]
#[serde(default)]
pub struct KanjiData {
    pub strokes: u8,
    pub grade: Option<u8>,
    pub freq: Option<u16>,
    pub jlpt_old: Option<u8>,
    pub jlpt_new: Option<u8>,
    pub meanings: Vec<String>,
    pub readings_on: Vec<String>,
    pub readings_kun: Vec<String>,
    pub wk_level: Option<u8>,
    #[serde_as(deserialize_as = "DefaultOnNull")]
    pub wk_meanings: Vec<String>,
    #[serde_as(deserialize_as = "DefaultOnNull")]
    pub wk_readings_on: Vec<String>,
    #[serde_as(deserialize_as = "DefaultOnNull")]
    pub wk_readings_kun: Vec<String>,
    #[serde_as(deserialize_as = "DefaultOnNull")]
    pub wk_radicals: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
pub struct KanjiMap {
    map: HashMap<char, KanjiData>,
}

pub fn get_kanji_data(word: &char) -> Option<KanjiData> {
    KANJI_MAP.get(word).cloned()
}

pub fn get_meanings(word: &char) -> Vec<String> {
    let kanji_data = KANJI_MAP.get(word);

    kanji_data
        .into_iter()
        .flat_map(|e| e.meanings.clone())
        .collect()
}

fn get_map_from_json(json: &str) -> HashMap<char, KanjiData> {
    serde_json::from_str(json).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jpn::JpnWordInfo;
    use log::info;

    #[test]
    fn typed_example() -> Result<(), ()> {
        // Some JSON input data as a &str. Maybe this comes from the user.
        // let data = std::include_str!("kanji.json");

        // Parse the string of data into a Person object. This is exactly the
        // same function as the one that produced serde_json::Value above, but
        // now we are asking it for a Person as output.
        // let string = fs::read_to_string("/kanji.json").unwrap();
        // get_map_from_json(&string);

        let word = "唖".chars().next().unwrap();
        let option = KANJI_MAP.get(&word).unwrap();
        info!("{:#?}", option);

        let kanji = serde_json::to_string(option).unwrap();

        info!("{}", kanji);

        let info = JpnWordInfo::new(word);

        let kanji = serde_json::to_string(&info).unwrap();

        info!("{}", kanji);

        Ok(())
    }
}
