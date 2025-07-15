use crate::jpn::kanji::{KanjiData, get_kanji_data};
use crate::ui::shutdown::TASK_TRACKER;
use jmdict::{Entry, GlossLanguage};

pub mod dict;
pub mod kanji;

#[derive(Debug, serde::Serialize, serde::Deserialize, Default, PartialEq, Clone)]
#[serde(default)]
pub struct JpnData {
    pub words: Vec<JpnWordInfo>,
    pub jm_dict: Vec<JmDictInfo>,
}

impl JpnData {
    fn new(word: &str, entries: &[Entry]) -> Self {
        let words = word.chars().map(JpnWordInfo::new).collect();

        let jm_dict = entries.iter().map(JmDictInfo::new).collect();

        Self { words, jm_dict }
    }

    pub fn has_kanji_data(&self) -> bool {
        self.words.iter().any(|w| w.kanji_data.is_some())
            || self.jm_dict.iter().any(|w| !w.info.is_empty())
    }

    pub fn get_kanji(&self) -> String {
        self.words.iter().map(|x| x.word).collect()
    }

    pub fn get_info_rows(&self) -> Vec<String> {
        if self.words.is_empty() {
            return vec![];
        }

        let mut info = vec![];

        self.jm_dict
            .iter()
            .for_each(|x| info.extend(x.info.iter().cloned()));

        self.words
            .iter()
            .filter(|x| {
                x.kanji_data
                    .as_ref()
                    .is_some_and(|x| !x.meanings.is_empty())
            })
            .map(|x| {
                [
                    format!(
                        "{}: {}",
                        x.word,
                        x.kanji_data.as_ref().unwrap().meanings.join(", ")
                    ),
                    format!(
                        "on Reading: {}",
                        x.kanji_data.as_ref().unwrap().readings_on.join(", ")
                    ),
                    format!(
                        "kun Reading: {}",
                        x.kanji_data.as_ref().unwrap().readings_kun.join(", ")
                    ),
                ]
            })
            .for_each(|x| info.extend(x.iter().cloned()));

        info.retain(|x| !x.is_empty());

        info
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Default, PartialEq, Clone)]
#[serde(default)]
pub struct JpnWordInfo {
    pub word: char,
    #[serde(skip)]
    pub kanji_data: Option<KanjiData>,
}

impl JpnWordInfo {
    fn new(word: char) -> Self {
        let kanji_data = get_kanji_data(word);

        Self { word, kanji_data }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Default, PartialEq, Clone)]
#[serde(default)]
pub struct JmDictInfo {
    pub info: Vec<String>,
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
            TASK_TRACKER.spawn(async move {
                dict::async_extract_words(&x)
                    .await
                    .iter()
                    .map(|(txt, entries)| JpnData::new(txt, entries))
                    .collect()
            })
        })
        .collect();

    let results: Vec<Vec<JpnData>> = futures::future::try_join_all(window_input)
        .await
        .unwrap_or_default();

    results
}

pub fn get_info_from_entry(e: &Entry) -> Vec<String> {
    let mut output: Vec<String> = Vec::new();
    for kanji in e.kanji_elements() {
        output.push(format!("Kanji: {:?}, ", kanji.text.to_string()));
    }

    for reading in e.reading_elements() {
        output.push(format!("Reading: {:?}, ", reading.text.to_string()));
        for info in reading.infos() {
            output.push(format!("{info:?}, "));
        }
    }
    output.push(String::new());

    for (index, sense) in e.senses().enumerate() {
        let parts_of_speech = sense
            .parts_of_speech()
            .map(|part| format!("{part}"))
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
            output.push(format!("{info:?}, "));
        }
    }

    output
}
