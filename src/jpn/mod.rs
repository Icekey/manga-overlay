use crate::jpn::kanji::{get_kanji_data, get_meanings, KanjiData};
use itertools::Itertools;
use jmdict::{Entry, GlossLanguage};

pub mod dict;
pub mod kanji;

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
pub struct JpnData {
    words: Vec<JpnWordInfo>,
    jm_dict: Vec<JmDictInfo>,
}

impl JpnData {
    fn new(word: (String, Vec<Entry>)) -> Self {
        let words = word.0.chars().map(JpnWordInfo::new).collect();

        let jm_dict = word.1.iter().map(JmDictInfo::new).collect();

        Self { words, jm_dict }
    }

    pub fn has_kanji_data(&self) -> bool {
        self.words.iter().any(|w| w.kanji_data.is_some())
            || self.jm_dict.iter().any(|w| !w.info.is_empty())
    }

    pub fn get_kanji(&self) -> String {
        self.words.iter().map(|x| x.word).collect()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
pub struct JpnWordInfo {
    word: char,
    #[serde(skip)]
    kanji_data: Option<KanjiData>,
}

impl JpnWordInfo {
    fn new(word: char) -> Self {
        let kanji_data = get_kanji_data(&word);

        Self { word, kanji_data }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
pub struct JmDictInfo {
    info: Vec<String>,
}

impl JmDictInfo {
    fn new(entry: &Entry) -> Self {
        let info: Vec<String> = get_info_from_entry(entry).into_iter().collect();
        Self { info }
    }
}

pub async fn get_jpn_data(input: &str) -> Vec<Vec<JpnData>> {
    let lines: Vec<String> = input.lines().map(dict::remove_whitespace).collect();

    let window_input: Vec<_> = lines
        .into_iter()
        .map(|x| {
            tokio::task::spawn(async move {
                dict::async_extract_words(&x)
                    .await
                    .into_iter()
                    .map(JpnData::new)
                    .collect()
            })
        })
        .collect();

    let results: Vec<Vec<JpnData>> = futures::future::try_join_all(window_input).await.unwrap();

    results
}

pub fn get_meaning_line(word: &char) -> Option<String> {
    let meanings = get_meanings(word);
    if meanings.is_empty() {
        return None;
    }
    let line = format!("{} meaning: {}", word, meanings.into_iter().join(", "));
    Some(line)
}

pub fn get_dict_output_vec(input: (String, Vec<Entry>)) -> Vec<String> {
    let mut output: Vec<String> = input.1.iter().flat_map(get_info_from_entry).collect();

    let count = output.len();

    output.push(format!("{} entries for {}", count, input.0));

    output
}

pub fn get_info_from_entry(e: &Entry) -> Vec<String> {
    let mut output: Vec<String> = Vec::new();
    for kanji in e.kanji_elements() {
        output.push(format!("Kanji: {:?}, ", kanji.text.to_string()));
    }

    for reading in e.reading_elements() {
        output.push(format!("Reading: {:?}, ", reading.text.to_string()));
        for info in reading.infos() {
            output.push(format!("{:?}, ", info));
        }
    }
    output.push(String::new());

    for (index, sense) in e.senses().enumerate() {
        let parts_of_speech = sense
            .parts_of_speech()
            .map(|part| format!("{}", part))
            .collect::<Vec<String>>()
            .join(", ");
        let english_meaning = sense
            .glosses()
            .filter(|g| g.language == GlossLanguage::English)
            .map(|g| g.text)
            .collect::<Vec<&str>>()
            .join("; ");
        output.push(format!(
            "{}. {}: {}",
            index + 1,
            parts_of_speech,
            english_meaning
        ));

        for info in sense.topics() {
            output.push(format!("{:?}, ", info));
        }
    }

    output
}
