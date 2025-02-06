use jmdict::{Entry, KanjiElement, ReadingElement};
use multimap::MultiMap;
use std::sync::LazyLock;

const WINDOW_SIZE: usize = 50;
const LARGEST_WORD_SIZE: usize = 15;
const STEP_SIZE: usize = WINDOW_SIZE - LARGEST_WORD_SIZE;

static JMDICT_MAP: LazyLock<MultiMap<char, Entry>> = LazyLock::new(|| create_jmdict_map());

fn create_jmdict_map() -> MultiMap<char, Entry> {
    let x: Vec<(&'static str, Entry)> = jmdict::entries()
        .flat_map(|x| x.kanji_elements().map(move |e| (e.text, x)))
        .collect();
    let y: Vec<(&'static str, Entry)> = jmdict::entries()
        .flat_map(|x| x.reading_elements().map(move |e| (e.text, x)))
        .collect();

    let mut map: MultiMap<char, Entry> = MultiMap::new();
    for i in x {
        map.insert(i.0.chars().next().unwrap(), i.1);
    }

    for i in y {
        map.insert(i.0.chars().next().unwrap(), i.1);
    }

    map
}

pub async fn async_extract_words(input: &str) -> Vec<(String, Vec<Entry>)> {
    let inter = input.chars().collect::<Vec<char>>();

    if inter.len() <= WINDOW_SIZE {
        return extract_words(input);
    }

    let mut windows: Vec<String> = inter
        .windows(WINDOW_SIZE)
        .step_by(STEP_SIZE)
        .map(|x| x.iter().collect::<String>())
        .collect();
    let window_char_count = windows.len() * STEP_SIZE;
    let remainder: String = input
        .chars()
        .skip(window_char_count.saturating_sub(LARGEST_WORD_SIZE))
        .collect();
    windows.push(remainder);

    let window_input: Vec<_> = windows
        .into_iter()
        .map(|x| tokio::task::spawn(async move { extract_words(&x) }))
        .collect();

    let results: Vec<Vec<(String, Vec<Entry>)>> = futures::future::try_join_all(window_input)
        .await
        .unwrap_or_else(|e| {
            println!("async_extract_words: {}", e);
            vec![]
        });

    combine_overlapping_vecs_with_entries(results)
}

fn combine_overlapping_vecs_with_entries(
    result_vecs: Vec<Vec<(String, Vec<Entry>)>>,
) -> Vec<(String, Vec<Entry>)> {
    let mut buffer: Vec<(String, Vec<Entry>)> = vec![];
    let mut valid_until_index: usize = 0;

    let last_result_index = result_vecs.len() - 1;

    for (i, results) in result_vecs.into_iter().enumerate() {
        let mut offset = STEP_SIZE * i;

        let mut skip = 0;
        for (j, word) in results.iter().enumerate() {
            if offset == valid_until_index {
                skip = j;
                break;
            }
            let char_count = word.0.chars().count();

            offset += char_count;
        }

        let results_length = if i >= last_result_index {
            results.len()
        } else {
            results.len() - 1
        };
        let mut take: Vec<(String, Vec<Entry>)> = results
            .into_iter()
            .take(results_length)
            .skip(skip)
            .collect();
        valid_until_index += take.iter().map(|e| e.0.chars().count()).sum::<usize>();

        buffer.append(&mut take);
    }
    buffer
}

pub fn remove_whitespace(s: &str) -> String {
    s.split_whitespace().collect()
}

fn extract_words(input: &str) -> Vec<(String, Vec<Entry>)> {
    let mut output: Vec<(String, Vec<Entry>)> = Vec::new();
    let mut rest: Option<&str> = Some(input);
    while let Some(x) = rest {
        if x.is_empty() {
            return output;
        }

        let (prefix, matches) = extract_dict_entries(x);
        rest = x.strip_prefix(&prefix);

        output.push((prefix, matches));
    }

    output
}

fn extract_dict_entries(input: &str) -> (String, Vec<Entry>) {
    if input.is_empty() {
        panic!("input '{}'", input)
    }

    let mut current_prefix: String = input.chars().take(1).collect();
    let initial_entries = JMDICT_MAP.get_vec(&current_prefix.chars().next().unwrap());
    if initial_entries.is_none() {
        return (current_prefix, vec![]);
    }

    let mut possible_matches: Vec<Entry> = initial_entries.unwrap().clone();
    if possible_matches.is_empty() {
        return (current_prefix, vec![]);
    }

    for i in 2..input.len() {
        let sub: String = input.chars().take(i).collect();
        let new_matches = get_starting_matches(&sub, possible_matches.clone().into_iter());

        if new_matches.is_empty() {
            return get_full_matches(current_prefix, possible_matches);
        }
        current_prefix = sub;
        possible_matches = new_matches;
    }

    get_full_matches(current_prefix, possible_matches)
}

fn get_full_matches(prefix: String, possible_matches: Vec<Entry>) -> (String, Vec<Entry>) {
    let full_matches: Vec<Entry> = possible_matches
        .into_iter()
        .filter(|e| e.is_full_match(&prefix))
        .collect();

    (prefix, full_matches)
}

fn get_starting_matches(prefix: &str, entries: impl Iterator<Item = Entry>) -> Vec<Entry> {
    entries.filter(|e| e.has_prefix(prefix)).collect()
}

trait HasText {
    fn get_text(&self) -> &'static str;
}

trait MatchesText {
    fn has_prefix(&self, prefix: &str) -> bool;
    fn is_full_match(&self, prefix: &str) -> bool;
}

impl<T: HasText> MatchesText for T {
    fn has_prefix(&self, prefix: &str) -> bool {
        self.get_text().starts_with(prefix)
    }

    fn is_full_match(&self, prefix: &str) -> bool {
        self.get_text() == prefix
    }
}

impl HasText for ReadingElement {
    fn get_text(&self) -> &'static str {
        self.text
    }
}

impl HasText for KanjiElement {
    fn get_text(&self) -> &'static str {
        self.text
    }
}

impl MatchesText for Entry {
    fn has_prefix(&self, prefix: &str) -> bool {
        self.kanji_elements().any(|k| k.has_prefix(prefix))
    }

    fn is_full_match(&self, prefix: &str) -> bool {
        self.kanji_elements().any(|k| k.is_full_match(prefix))
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;

    const LOREM : &str = "規ょフ記刊ねトゃ懸文朽っ面務75載ユ対芸フルラ寄63分ちょと対本1張スヘワツ大覧げんち語世び職学ヒヨフ報断ネケホ盟工フトミ開査亜才きほあ。例キネヒユ旅揮あれ況柱ッしわひ剤平さ注分投天タウヤ支警うイほさ考広もび施甲マニテタ告容イじ版提聞チ幅行ミニヒル属内て任喜らラよ着集輝れ冷済成索のでつ。

督だょ職真ばを確辺ぐ碁近ネ然有タラリオ未3備月ラノムテ員職トね録記ご選図コフイ史経82置リフ湯震ムシリタ展査テ清面をト格9検め。1同勢ト形界めり禁私メヒア航移だとせ昇分革会上ミイ感築わっば事購おリフ生人シヌタ残革書ゅリ委何ヱマ従写ヲノヤネ都地みろ意携をん月男妊ね。

大エヲモ別意ユタセテ指車載城さ影真ラ界年じフうめ一子葉けラえだ者質ょずせ研言アロスリ迎村ゃ決欺茶針促さよば。果ハ週7効ご読失転探とめみリ婚71常ねあべ文式セ京討そばス育望ツエ訴5村びン医僕滞硬イルッた。89情モハエ顔書素ミ求動ぱ供先ざをトル宣択ぼ館聞ごへな扶観ほもぞト今合ヘモコ見費ナミ理発ぐふ州7過掛海ま頭型ルサフメ投要サリメル持務れほ威悲カ判覇しすは。

後ぼ旅他がつル人宮めはに研最ドやじ小情新むぱにっ元亡ネケ論都磨ア屋永覧橋びいあ術21編クトキ庁体みるを作71惑はスづ始一ノフヲ無運ラリこふ。理ろわ真広以クヒ思撮1化4著ホムヘ京芸るだ応氷ンルふ刑勝スみフめ私作ユウコ出更び伝露キシ月断メマシ応根企かねす朝慶レコセ今価ル山子ねみべそ。

載えすめ太軒つでゅン読方ヤウ関消ずスば優載ど成日目リ広各さ伊選メアタウ直7水ゃ古検スヒ育読イセヒソ聞63報るゃつ覧裁つちゅぜ記馬の。終撃トぐほ世凍ホチ前内マハ寄敵コ信2違ヱヤヘロ恋第ソテ見中車せスえ始音細へ経警べぎ選卸す。高57作才ニソノ除家ずク鮮不のけえス欺別出湘ほび理軍ごラぜ朗皇がこへ総幕ヒ不本オクイ改地トノ何能3般セサラ図都ムヤハテ捕仙沢温ひぐえ。

体子っをさ変質ツチヒロ新害トなあ倍上サ駒誰ふ込験ルソハ下堀なじよゆ資之ユ月9問ミメケ止苗きフぼ者載ど長真嚇クぞ生書マヲ使幅採べめぐじ。療をとば森省ぽく竹月物せいほぶ速属切っ更94告算京20聞ヌ値読然ヲネ紀未アヱ荒読転スイ告与ほっッ委天条ヤキヘ軍機健了つ。絶3北ナ量43説れつ器教つン常牲むあス利経ロエユ断過リ国彫記ゆひあ支光へがれぴ子気じ伊化ヱスラ備偉塔にて。

書ぱふうつ和部だ愛根ろ位館定レ増気ーぽ止8読ヱスリオ号社ヨケミノ験盛るルほ日記べま官横ゅがゃげ黒協外せ勝浦ヨス申真ねゅ朝入殺ぜかさ載康メ視周おっが。転ー一菊セノロ年川ツウナフ天京メヱ施96連東ふ責平能そでほ覧公ヲルソナ事機ゃ特74高無旗昼栗びぜ。察んそ供遺ッわにみ医夢ユ願親ラセヘキ少識ナ韓疑時シコネテ強男ワネリ研効とょ球9加ッし給覚格8隊セ集乳クあリづ。

4耕クコ町74選タ崎浦権長そっざ厳左ルメ問42台会ナトア策軽だつ生佐ヱカ多千政伎券ぜ。医レ聞止的ろづ供明ケ提明ノネイセ推整オケア会禁ホユ藤覧フイ資谷さ川5初コエノ社96知辞たしくぶ済遺拡よじお。攻チヲユ小国イ材関理け父化画ヨナミ語笑ざこ神之えるはう終垂んスせな要遺も届志ヌタ初日ドをろた歌応ざ変一え要天ロフ刊変税はあぞ回界つ実円紹へたれ可伝拓泰至こおに。

石オヱツ指1車ゆラ軽明ぶめた喰周おて起研禁際ゆちーだ刊政ミヱネヌ知服今ろひ稿応のあふ今内選イぽッ写就覧喜ふろみ。教キノ年13革まど全記じリさ講中アネ書2全テラヘ青近崎てすゃ出71引ウレフユ首代そす禁自書まーぽ雪保ヤテヒ防景ヒリ長韓ノクフヲ利止叟噂愉びどむ。人ぶげなつ愛重ドろ催五キ詳短移アヒ折泰ケル開塊ぎぼゅ企8意囲まゅめみ産選あてリ障男長ラヲ北瀬セ入成販ょすは。

三業オネ各政タホ技九づッン題任ノリ載75左ゅとのあ豆条必野きりゅ一際最ナアカロ高8著ンごイな区港まさ日天よびド収金ょぽ。睦べむクふ実93家福ウツヘ競満万キハモソ長投せ強巨そ観条マセ速能続ぶづの使保ゆ試町ラア江雑コナ福富開王乏えか。悪どぜとせ遺意志ムヒ事経からス真取ぴぐっ芸験ざ闘調たざへ広上ぶ聞題メワテヘ阜13家ネサ家秋ラ経都チメヨ職左削幸績よし。";

    #[tokio::test(flavor = "multi_thread")]
    async fn benchmark() {
        let input = LOREM.repeat(2);

        for _ in 0..10 {
            let label = "async_extract_words";
            let start = SystemTime::now();
            let _ = async_extract_words(&input).await;
            let end = SystemTime::now();
            let duration = end.duration_since(start).unwrap();
            println!("function took {} {:?}", label, duration);

            println!("----");
        }
    }
}
