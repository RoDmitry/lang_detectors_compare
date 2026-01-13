use ::std::{
    fs,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::LazyLock,
    time::Instant,
};
use ahash::{AHashMap, AHashSet};
use alphabet_detector::{ScriptLanguage, UcdScript};
use fraction::Decimal;
use itertools::Itertools;
use langram::{DetectorBuilder as LangramDetectorBuilder, ModelsStorage};
use lingua::Language as LinguaLanguage;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use strum::IntoEnumIterator;
use titlecase::titlecase;
use whatlang::{Detector as WhatlangDetector, Lang as WhatlangLanguage};
use whichlang::Lang as WhichlangLanguage;

mod detector_statistics;
mod statistic;

use detector_statistics::DetectorStatistics;
use statistic::Statistic;

static LANGRAM_MODELS: LazyLock<ModelsStorage> = LazyLock::new(|| ModelsStorage::new().unwrap());
static LINGUA_DETECTOR_HIGH_ACCURACY: LazyLock<lingua::LanguageDetector> = LazyLock::new(|| {
    lingua::LanguageDetectorBuilder::from_all_spoken_languages()
        .with_preloaded_language_models()
        .build()
});
static WHATLANG_DETECTOR: LazyLock<WhatlangDetector> = LazyLock::new(WhatlangDetector::new);

fn alphabet_detect(
    texts: &[String],
    _languages: &AHashSet<ScriptLanguage>,
) -> Vec<Option<ScriptLanguage>> {
    texts
        .par_iter()
        .map(|text| {
            let langs = alphabet_detector::fulltext_filter_max::<bool>(text.char_indices())
                .1
                .collect::<Vec<_>>();
            if langs.len() == 1 {
                langs.into_iter().next()
            } else {
                None
            }
        })
        .collect()
}

fn langram_detect_max_trigrams(
    texts: &[String],
    languages: &AHashSet<ScriptLanguage>,
    reorder: bool,
) -> Vec<Option<ScriptLanguage>> {
    let detector = LangramDetectorBuilder::new(&LANGRAM_MODELS)
        .languages(languages.clone())
        .max_trigrams()
        .build();
    texts
        .par_iter()
        .map(|text| detector.detect_top_one(text, reorder))
        .collect()
}

fn langram_detect_all_ngrams(
    texts: &[String],
    languages: &AHashSet<ScriptLanguage>,
    reorder: bool,
) -> Vec<Option<ScriptLanguage>> {
    let detector = LangramDetectorBuilder::new(&LANGRAM_MODELS)
        .languages(languages.clone())
        .build();
    texts
        .par_iter()
        .map(|text| detector.detect_top_one(text, reorder))
        .collect()
}

fn lingua_detect_high_accuracy(
    texts: &[String],
    _languages: &AHashSet<ScriptLanguage>,
) -> Vec<Option<ScriptLanguage>> {
    texts
        .par_iter()
        .map(|text| {
            LINGUA_DETECTOR_HIGH_ACCURACY
                .detect_language_of(text)
                .and_then(map_lingua)
        })
        .collect()
}

fn whatlang_detect(
    texts: &[String],
    _languages: &AHashSet<ScriptLanguage>,
) -> Vec<Option<ScriptLanguage>> {
    texts
        .par_iter()
        .map(|text| WHATLANG_DETECTOR.detect_lang(text).and_then(map_whatlang))
        .collect()
}

fn whichlang_detect(
    texts: &[String],
    _languages: &AHashSet<ScriptLanguage>,
) -> Vec<Option<ScriptLanguage>> {
    texts
        .par_iter()
        .map(|text| map_whichlang(whichlang::detect_language(text)))
        .collect()
}

fn load_texts() -> AHashMap<ScriptLanguage, Vec<String>> {
    let mut res = AHashMap::default();
    let paths = fs::read_dir("./texts").unwrap();
    for path in paths {
        let path = path.unwrap();
        let file_name = path.file_name().into_string().unwrap();
        let Some(lang) = ScriptLanguage::from_str(&file_name) else {
            panic!("*{file_name}* Not found lang");
        };

        let lines = BufReader::new(File::open(path.path()).expect("open failed"))
            .lines()
            .map(|line| line.unwrap());
        res.insert(lang, lines.collect());
    }

    res
}

fn collect_statistics(
    detector_name: &str,
    reports_directory: Option<PathBuf>,
    languages: &AHashSet<ScriptLanguage>,
    detector_fn: fn(&[String], &AHashSet<ScriptLanguage>) -> Vec<Option<ScriptLanguage>>,
    langs_texts: &AHashMap<ScriptLanguage, Vec<String>>,
    langs_two_words: &AHashMap<ScriptLanguage, Vec<String>>,
    langs_single_words: &AHashMap<ScriptLanguage, Vec<String>>,
) -> AHashMap<ScriptLanguage, DetectorStatistics> {
    let now = Instant::now();
    let mut language_statistics = AHashMap::new();

    if let Some(reports_directory) = &reports_directory {
        if !reports_directory.is_dir() {
            fs::create_dir_all(reports_directory).expect("Reports directory could not be created");
        }
    }

    let total_language_count = languages.len();

    for (idx, language) in languages.iter().enumerate() {
        let mut statistics = DetectorStatistics::new();

        let Some(texts) = langs_texts.get(language) else {
            continue;
        };
        {
            let sentence_results = detector_fn(texts, languages);
            for (i, sentence) in texts.iter().enumerate() {
                statistics.add_sentence_counts(*sentence_results.get(i).unwrap(), sentence);
            }
        }

        let Some(two_words) = langs_two_words.get(language) else {
            continue;
        };
        {
            let word_pair_results = detector_fn(two_words, languages);
            for (i, word_pair) in two_words.iter().enumerate() {
                statistics.add_word_pair_counts(*word_pair_results.get(i).unwrap(), word_pair);
            }
        }

        let Some(single_words) = langs_single_words.get(language) else {
            continue;
        };
        {
            let single_word_results = detector_fn(single_words, languages);
            for (i, single_word) in single_words.iter().enumerate() {
                let l = *single_word_results.get(i).unwrap();
                /* if *language == ScriptLanguage::Lao && l.is_none() {
                    println!("word: {:?}", single_word);
                } */
                statistics.add_single_word_counts(l, single_word);
            }
        }

        let average_accuracy = statistics.compute_accuracy_values(*language);

        if let Some(reports_directory) = &reports_directory {
            let report_file_name = titlecase(&format!("{language:?}.txt"));
            let report_file_path = reports_directory.join(&report_file_name);
            let report_data = statistics.create_report_data(*language, average_accuracy);

            if let Some(report) = report_data {
                fs::write(report_file_path, report).expect("Reports file could not be written");
            }
        }

        println!(
            "Writing {detector_name} reports for {:?}... ({}/{})",
            language,
            (idx + 1),
            total_language_count
        );
        language_statistics.insert(*language, statistics);
    }

    println!(
        "{detector_name} reports written in {:.3} seconds\n",
        now.elapsed().as_secs_f64()
    );

    language_statistics
}

fn write_reports_to_file(
    file_path: PathBuf,
    columns: &[&str],
    statistics: &[AHashMap<ScriptLanguage, DetectorStatistics>],
    languages: impl Iterator<Item = ScriptLanguage>,
) {
    let mut report_file = fs::File::create(file_path).expect("report file could not be created");

    report_file
        .write_all(columns.iter().join(",").as_bytes())
        .expect("CSV header row could not be written");

    for language in languages {
        let row = statistics
            .iter()
            .map(|s| {
                s.get(&language)
                    .map(|ls| ls.create_aggregated_report_row(language))
                    .unwrap_or(",,,".to_owned())
            })
            .join(", ");

        let report_row = format!("{:?}, {}\n", &language, row,);

        report_file
            .write_all(report_row.as_bytes())
            .expect("CSV data row could not be written");
    }
}

fn main() {
    let now = Instant::now();

    let langs_texts: AHashMap<ScriptLanguage, Vec<String>> = load_texts();
    let words_iter = langs_texts.iter().map(|(&language, ts)| {
        let is_han = UcdScript::from(language) == UcdScript::Han;

        (
            language,
            ts.iter().flat_map(move |t| {
                alphabet_detector::words::from_ch_ind::<String>(t.char_indices())
                    .filter_map(move |wld| {
                        // unfiltered chars for `Script::Han`
                        if is_han {
                            let chars: Vec<_> = wld
                                .buf
                                .chars()
                                .filter(|&ch| UcdScript::find(ch) == UcdScript::Han)
                                .map(|ch| ch.to_string())
                                .collect();
                            return if chars.is_empty() {
                                None
                            } else {
                                Some(chars.into_iter())
                            };
                        }

                        if unsafe { *wld.langs_cnt.get_unchecked(language as usize) }
                            == wld.buf.chars().count() as u32
                        {
                            Some(vec![wld.buf].into_iter())
                        } else {
                            None
                        }
                    })
                    .flatten()
            }),
        )
    });

    let langs_single_words: AHashMap<ScriptLanguage, Vec<String>> = words_iter
        .clone()
        .map(|(language, ts)| (language, ts.collect::<AHashSet<_>>().into_iter().collect()))
        .collect();

    let langs_word_pairs: AHashMap<ScriptLanguage, Vec<String>> = words_iter
        .map(|(language, ts)| {
            let is_han = UcdScript::from(language) == UcdScript::Han;
            let separator = if is_han { "" } else { " " };

            (
                language,
                ts.tuple_windows()
                    .map(|(t1, t2)| t1 + separator + &t2)
                    .take(20000)
                    .collect::<AHashSet<_>>()
                    .into_iter()
                    .collect(),
            )
        })
        .collect();

    let accuracy_reports_directory = Path::new("accuracy");

    // Langram
    let languages: AHashSet<_> = ScriptLanguage::iter().collect();

    let alphabet_detector_statistics = collect_statistics(
        "alphabet_detector",
        Some(accuracy_reports_directory.join("alphabet_detector")),
        &languages,
        alphabet_detect,
        &langs_texts,
        &langs_word_pairs,
        &langs_single_words,
    );

    // force initialize
    LazyLock::force(&LANGRAM_MODELS);

    let langram_max_trigrams_statistics = collect_statistics(
        "langram_max_trigrams",
        Some(
            accuracy_reports_directory
                .join("langram")
                .join("max_trigrams"),
        ),
        &languages,
        |t, l| langram_detect_max_trigrams(t, l, false),
        &langs_texts,
        &langs_word_pairs,
        &langs_single_words,
    );

    let langram_all_ngrams_statistics = collect_statistics(
        "langram_all_ngrams",
        Some(
            accuracy_reports_directory
                .join("langram")
                .join("all_ngrams"),
        ),
        &languages,
        |t, l| langram_detect_all_ngrams(t, l, false),
        &langs_texts,
        &langs_word_pairs,
        &langs_single_words,
    );

    let langram_max_trigrams_reordered_statistics = collect_statistics(
        "langram_max_trigrams_reordered",
        None,
        &languages,
        |t, l| langram_detect_max_trigrams(t, l, true),
        &langs_texts,
        &langs_word_pairs,
        &langs_single_words,
    );

    let langram_all_ngrams_reordered_statistics = collect_statistics(
        "langram_all_ngrams_reordered",
        None,
        &languages,
        |t, l| langram_detect_all_ngrams(t, l, true),
        &langs_texts,
        &langs_word_pairs,
        &langs_single_words,
    );

    write_reports_to_file(
        accuracy_reports_directory.join("langram.csv"),
        &[
            "language",
            "alphabet_detector_avg",
            "alphabet_detector_word",
            "alphabet_detector_2words",
            "alphabet_detector_text",
            "langram_max_3grams_avg",
            "langram_max_3grams_word",
            "langram_max_3grams_2words",
            "langram_max_3grams_text",
            "langram_all_ngrams_avg",
            "langram_all_ngrams_word",
            "langram_all_ngrams_2words",
            "langram_all_ngrams_text",
            "langram_max_3grams_reordered_avg",
            "langram_max_3grams_reordered_word",
            "langram_max_3grams_reordered_2words",
            "langram_max_3grams_reordered_text",
            "langram_all_ngrams_reordered_avg",
            "langram_all_ngrams_reordered_word",
            "langram_all_ngrams_reordered_2words",
            "langram_all_ngrams_reordered_text\n",
        ],
        &[
            alphabet_detector_statistics,
            langram_max_trigrams_statistics,
            langram_all_ngrams_statistics,
            langram_max_trigrams_reordered_statistics,
            langram_all_ngrams_reordered_statistics,
        ],
        ScriptLanguage::iter().filter(|l| langs_texts.contains_key(l)),
    );

    // Lingua vs Langram
    let languages: AHashSet<_> = LinguaLanguage::iter()
        .filter_map(map_lingua)
        .filter(|&l| l != ScriptLanguage::Latin)
        .collect();

    // force initialize
    LazyLock::force(&LINGUA_DETECTOR_HIGH_ACCURACY);

    let lingua_statistics = collect_statistics(
        "lingua",
        Some(accuracy_reports_directory.join("lingua")),
        &languages,
        lingua_detect_high_accuracy,
        &langs_texts,
        &langs_word_pairs,
        &langs_single_words,
    );

    let langram_max_trigrams_statistics = collect_statistics(
        "langram_max_trigrams",
        None,
        &languages,
        |t, l| langram_detect_max_trigrams(t, l, false),
        &langs_texts,
        &langs_word_pairs,
        &langs_single_words,
    );

    let langram_all_ngrams_statistics = collect_statistics(
        "langram_all_ngrams",
        None,
        &languages,
        |t, l| langram_detect_all_ngrams(t, l, false),
        &langs_texts,
        &langs_word_pairs,
        &langs_single_words,
    );

    write_reports_to_file(
        accuracy_reports_directory.join("lingua_vs_langram.csv"),
        &[
            "language",
            "lingua_avg",
            "lingua_word",
            "lingua_2words",
            "lingua_text",
            "langram_max_3grams_avg",
            "langram_max_3grams_word",
            "langram_max_3grams_2words",
            "langram_max_3grams_text",
            "langram_all_ngrams_avg",
            "langram_all_ngrams_word",
            "langram_all_ngrams_2words",
            "langram_all_ngrams_text\n",
        ],
        &[
            lingua_statistics,
            langram_max_trigrams_statistics,
            langram_all_ngrams_statistics,
        ],
        ScriptLanguage::iter().filter(|l| languages.contains(l)),
    );

    // Whatlang vs Langram
    let languages: AHashSet<_> = WhatlangLanguage::all()
        .iter()
        .copied()
        .filter_map(map_whatlang)
        .collect();

    // force initialize
    LazyLock::force(&WHATLANG_DETECTOR);

    let whatlang_statistics = collect_statistics(
        "whatlang",
        Some(accuracy_reports_directory.join("whatlang")),
        &languages,
        whatlang_detect,
        &langs_texts,
        &langs_word_pairs,
        &langs_single_words,
    );

    let langram_max_trigrams_statistics = collect_statistics(
        "langram_max_trigrams",
        None,
        &languages,
        |t, l| langram_detect_max_trigrams(t, l, false),
        &langs_texts,
        &langs_word_pairs,
        &langs_single_words,
    );

    let langram_all_ngrams_statistics = collect_statistics(
        "langram_all_ngrams",
        None,
        &languages,
        |t, l| langram_detect_all_ngrams(t, l, false),
        &langs_texts,
        &langs_word_pairs,
        &langs_single_words,
    );

    write_reports_to_file(
        accuracy_reports_directory.join("whatlang_vs_langram.csv"),
        &[
            "language",
            "whatlang_avg",
            "whatlang_word",
            "whatlang_2words",
            "whatlang_text",
            "langram_max_3grams_avg",
            "langram_max_3grams_word",
            "langram_max_3grams_2words",
            "langram_max_3grams_text",
            "langram_all_ngrams_avg",
            "langram_all_ngrams_word",
            "langram_all_ngrams_2words",
            "langram_all_ngrams_text\n",
        ],
        &[
            whatlang_statistics,
            langram_max_trigrams_statistics,
            langram_all_ngrams_statistics,
        ],
        ScriptLanguage::iter().filter(|l| languages.contains(l)),
    );

    // Whichlang vs Langram
    let languages: AHashSet<_> = whichlang::LANGUAGES
        .into_iter()
        .filter_map(map_whichlang)
        .collect();

    let whichlang_statistics = collect_statistics(
        "whichlang",
        Some(accuracy_reports_directory.join("whichlang")),
        &languages,
        whichlang_detect,
        &langs_texts,
        &langs_word_pairs,
        &langs_single_words,
    );

    let langram_max_trigrams_statistics = collect_statistics(
        "langram_max_trigrams",
        None,
        &languages,
        |t, l| langram_detect_max_trigrams(t, l, false),
        &langs_texts,
        &langs_word_pairs,
        &langs_single_words,
    );

    let langram_all_ngrams_statistics = collect_statistics(
        "langram_all_ngrams",
        None,
        &languages,
        |t, l| langram_detect_all_ngrams(t, l, false),
        &langs_texts,
        &langs_word_pairs,
        &langs_single_words,
    );

    write_reports_to_file(
        accuracy_reports_directory.join("whichlang_vs_langram.csv"),
        &[
            "language",
            "whichlang_avg",
            "whichlang_word",
            "whichlang_2words",
            "whichlang_text",
            "langram_max_3grams_avg",
            "langram_max_3grams_word",
            "langram_max_3grams_2words",
            "langram_max_3grams_text",
            "langram_all_ngrams_avg",
            "langram_all_ngrams_word",
            "langram_all_ngrams_2words",
            "langram_all_ngrams_text\n",
        ],
        &[
            whichlang_statistics,
            langram_max_trigrams_statistics,
            langram_all_ngrams_statistics,
        ],
        ScriptLanguage::iter().filter(|l| languages.contains(l)),
    );

    println!(
        "All accuracy reports successfully written in {:.3} seconds",
        now.elapsed().as_secs_f64()
    );
}

fn format_accuracy(accuracy: Decimal) -> String {
    format!("{accuracy:.2}%")
}

fn map_whatlang(language: WhatlangLanguage) -> Option<ScriptLanguage> {
    use WhatlangLanguage::*;
    match language {
        Afr => Some(ScriptLanguage::Afrikaans),
        Aka => Some(ScriptLanguage::AkanTwi),
        Amh => Some(ScriptLanguage::Amharic),
        Ara => Some(ScriptLanguage::Arabic),
        Aze => Some(ScriptLanguage::AzerbaijaniNorth),
        Bel => Some(ScriptLanguage::Belarusian),
        Ben => Some(ScriptLanguage::Bengali),
        Bul => Some(ScriptLanguage::Bulgarian),
        Cat => Some(ScriptLanguage::Catalan),
        Ces => Some(ScriptLanguage::Czech),
        Cmn => Some(ScriptLanguage::ChineseMandarinSimplified),
        Cym => Some(ScriptLanguage::Welsh),
        Dan => Some(ScriptLanguage::Danish),
        Deu => Some(ScriptLanguage::German),
        Ell => Some(ScriptLanguage::Greek),
        Eng => Some(ScriptLanguage::English),
        Epo => Some(ScriptLanguage::Esperanto),
        Est => Some(ScriptLanguage::Estonian),
        Fin => Some(ScriptLanguage::Finnish),
        Fra => Some(ScriptLanguage::French),
        Guj => Some(ScriptLanguage::Gujarati),
        Heb => Some(ScriptLanguage::Hebrew),
        Hin => Some(ScriptLanguage::Hindi),
        Hrv => Some(ScriptLanguage::Croatian),
        Hun => Some(ScriptLanguage::Hungarian),
        Hye => Some(ScriptLanguage::Armenian),
        Ind => Some(ScriptLanguage::Indonesian),
        Ita => Some(ScriptLanguage::Italian),
        Jav => Some(ScriptLanguage::Javanese),
        Jpn => Some(ScriptLanguage::Japanese),
        Kan => Some(ScriptLanguage::Kannada),
        Kat => Some(ScriptLanguage::Georgian),
        Khm => Some(ScriptLanguage::Khmer),
        Kor => Some(ScriptLanguage::Korean),
        Lat => Some(ScriptLanguage::Latin),
        Lav => Some(ScriptLanguage::Latvian),
        Lit => Some(ScriptLanguage::Lithuanian),
        Mal => Some(ScriptLanguage::Malayalam),
        Mar => Some(ScriptLanguage::Marathi),
        Mkd => Some(ScriptLanguage::Macedonian),
        Mya => Some(ScriptLanguage::Burmese),
        Nep => Some(ScriptLanguage::Nepali),
        Nld => Some(ScriptLanguage::Dutch),
        Nob => Some(ScriptLanguage::NorwegianBokmal),
        Ori => Some(ScriptLanguage::OriyaOdia),
        Pan => Some(ScriptLanguage::PunjabiEastern),
        Pes => Some(ScriptLanguage::PersianFarsi),
        Pol => Some(ScriptLanguage::Polish),
        Por => Some(ScriptLanguage::Portuguese),
        Ron => Some(ScriptLanguage::Romanian),
        Rus => Some(ScriptLanguage::Russian),
        Sin => Some(ScriptLanguage::Sinhala),
        Slk => Some(ScriptLanguage::Slovak),
        Slv => Some(ScriptLanguage::Slovenian),
        Sna => Some(ScriptLanguage::Shona),
        Spa => Some(ScriptLanguage::Spanish),
        Srp => Some(ScriptLanguage::Serbian),
        Swe => Some(ScriptLanguage::Swedish),
        Tam => Some(ScriptLanguage::Tamil),
        Tel => Some(ScriptLanguage::Telugu),
        Tgl => Some(ScriptLanguage::Filipino),
        Tha => Some(ScriptLanguage::Thai),
        Tuk => Some(ScriptLanguage::Turkmen),
        Tur => Some(ScriptLanguage::Turkish),
        Ukr => Some(ScriptLanguage::Ukrainian),
        Urd => Some(ScriptLanguage::Urdu),
        Uzb => Some(ScriptLanguage::UzbekNorthern),
        Vie => Some(ScriptLanguage::Vietnamese),
        Yid => Some(ScriptLanguage::YiddishEastern),
        Zul => Some(ScriptLanguage::Zulu),
    }
}

fn map_whichlang(language: WhichlangLanguage) -> Option<ScriptLanguage> {
    use WhichlangLanguage::*;
    match language {
        Ara => Some(ScriptLanguage::Arabic),
        Cmn => Some(ScriptLanguage::ChineseMandarinSimplified),
        Deu => Some(ScriptLanguage::German),
        Eng => Some(ScriptLanguage::English),
        Fra => Some(ScriptLanguage::French),
        Hin => Some(ScriptLanguage::Hindi),
        Ita => Some(ScriptLanguage::Italian),
        Jpn => Some(ScriptLanguage::Japanese),
        Kor => Some(ScriptLanguage::Korean),
        Nld => Some(ScriptLanguage::Dutch),
        Por => Some(ScriptLanguage::Portuguese),
        Rus => Some(ScriptLanguage::Russian),
        Spa => Some(ScriptLanguage::Spanish),
        Swe => Some(ScriptLanguage::Swedish),
        Tur => Some(ScriptLanguage::Turkish),
        Vie => Some(ScriptLanguage::Vietnamese),
    }
}

fn map_lingua(language: LinguaLanguage) -> Option<ScriptLanguage> {
    use LinguaLanguage::*;
    match language {
        Afrikaans => Some(ScriptLanguage::Afrikaans),
        Albanian => Some(ScriptLanguage::AlbanianTosk),
        Arabic => Some(ScriptLanguage::Arabic),
        Armenian => Some(ScriptLanguage::Armenian),
        Azerbaijani => Some(ScriptLanguage::AzerbaijaniNorth),
        Basque => Some(ScriptLanguage::Basque),
        Belarusian => Some(ScriptLanguage::Belarusian),
        Bengali => Some(ScriptLanguage::Bengali),
        Bokmal => Some(ScriptLanguage::NorwegianBokmal),
        Bosnian => Some(ScriptLanguage::Bosnian),
        Bulgarian => Some(ScriptLanguage::Bulgarian),
        Catalan => Some(ScriptLanguage::Catalan),
        Chinese => Some(ScriptLanguage::ChineseMandarinSimplified),
        Croatian => Some(ScriptLanguage::Croatian),
        Czech => Some(ScriptLanguage::Czech),
        Danish => Some(ScriptLanguage::Danish),
        Dutch => Some(ScriptLanguage::Dutch),
        English => Some(ScriptLanguage::English),
        Esperanto => Some(ScriptLanguage::Esperanto),
        Estonian => Some(ScriptLanguage::Estonian),
        Finnish => Some(ScriptLanguage::Finnish),
        French => Some(ScriptLanguage::French),
        Ganda => Some(ScriptLanguage::Ganda),
        Georgian => Some(ScriptLanguage::Georgian),
        German => Some(ScriptLanguage::German),
        Greek => Some(ScriptLanguage::Greek),
        Gujarati => Some(ScriptLanguage::Gujarati),
        Hebrew => Some(ScriptLanguage::Hebrew),
        Hindi => Some(ScriptLanguage::Hindi),
        Hungarian => Some(ScriptLanguage::Hungarian),
        Icelandic => Some(ScriptLanguage::Icelandic),
        Indonesian => Some(ScriptLanguage::Indonesian),
        Irish => Some(ScriptLanguage::Irish),
        Italian => Some(ScriptLanguage::Italian),
        Japanese => Some(ScriptLanguage::Japanese),
        Kazakh => Some(ScriptLanguage::Kazakh),
        Korean => Some(ScriptLanguage::Korean),
        Latin => Some(ScriptLanguage::Latin),
        Latvian => Some(ScriptLanguage::Latvian),
        Lithuanian => Some(ScriptLanguage::Lithuanian),
        Macedonian => Some(ScriptLanguage::Macedonian),
        Malay => Some(ScriptLanguage::Malay),
        Maori => Some(ScriptLanguage::Maori),
        Marathi => Some(ScriptLanguage::Marathi),
        Mongolian => Some(ScriptLanguage::MongolianHalh),
        Nynorsk => Some(ScriptLanguage::NorwegianNynorsk),
        Persian => Some(ScriptLanguage::PersianFarsi),
        Polish => Some(ScriptLanguage::Polish),
        Portuguese => Some(ScriptLanguage::Portuguese),
        Punjabi => Some(ScriptLanguage::PunjabiEastern),
        Romanian => Some(ScriptLanguage::Romanian),
        Russian => Some(ScriptLanguage::Russian),
        Serbian => Some(ScriptLanguage::Serbian),
        Shona => Some(ScriptLanguage::Shona),
        Slovak => Some(ScriptLanguage::Slovak),
        Slovene => Some(ScriptLanguage::Slovenian),
        Somali => Some(ScriptLanguage::Somali),
        Sotho => Some(ScriptLanguage::Sesotho),
        Spanish => Some(ScriptLanguage::Spanish),
        Swahili => Some(ScriptLanguage::Swahili),
        Swedish => Some(ScriptLanguage::Swedish),
        Tagalog => Some(ScriptLanguage::Filipino),
        Tamil => Some(ScriptLanguage::Tamil),
        Telugu => Some(ScriptLanguage::Telugu),
        Thai => Some(ScriptLanguage::Thai),
        Tsonga => Some(ScriptLanguage::Tsonga),
        Tswana => Some(ScriptLanguage::Tswana),
        Turkish => Some(ScriptLanguage::Turkish),
        Ukrainian => Some(ScriptLanguage::Ukrainian),
        Urdu => Some(ScriptLanguage::Urdu),
        Vietnamese => Some(ScriptLanguage::Vietnamese),
        Welsh => Some(ScriptLanguage::Welsh),
        Xhosa => Some(ScriptLanguage::Xhosa),
        Yoruba => Some(ScriptLanguage::Yoruba),
        Zulu => Some(ScriptLanguage::Zulu),
    }
}
