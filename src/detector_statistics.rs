use crate::{format_accuracy, Statistic};
use ahash::AHashMap;
use alphabet_detector::ScriptLanguage;
use fraction::{Decimal, Zero};
use indoc::formatdoc;

pub(super) struct DetectorStatistics {
    single_word_statistic: Statistic,
    two_words_statistic: Statistic,
    text_statistic: Statistic,
    average_accuracies: AHashMap<ScriptLanguage, Decimal>,
}

impl DetectorStatistics {
    #[inline]
    pub(super) fn new() -> Self {
        Self {
            single_word_statistic: Statistic::new(),
            two_words_statistic: Statistic::new(),
            text_statistic: Statistic::new(),
            average_accuracies: AHashMap::new(),
        }
    }

    pub(super) fn add_single_word_counts(
        &mut self,
        language: Option<ScriptLanguage>,
        single_word: &str,
    ) {
        self.single_word_statistic.add_language_count(language);
        self.single_word_statistic.add_entity_count();
        self.single_word_statistic
            .add_entity_length_count(single_word);
    }

    pub(super) fn add_word_pair_counts(
        &mut self,
        language: Option<ScriptLanguage>,
        word_pair: &str,
    ) {
        self.two_words_statistic.add_language_count(language);
        self.two_words_statistic.add_entity_count();
        self.two_words_statistic.add_entity_length_count(word_pair);
    }

    pub(super) fn add_sentence_counts(&mut self, language: Option<ScriptLanguage>, sentence: &str) {
        self.text_statistic.add_language_count(language);
        self.text_statistic.add_entity_count();
        self.text_statistic.add_entity_length_count(sentence);
    }

    pub(super) fn compute_accuracy_values(&mut self, language: ScriptLanguage) -> Decimal {
        self.single_word_statistic.map_counts_to_accuracy_values();
        self.two_words_statistic.map_counts_to_accuracy_values();
        self.text_statistic.map_counts_to_accuracy_values();

        let single_word_accuracy = self.single_word_statistic.get_accuracy(language);
        let word_pair_accuracy = self.two_words_statistic.get_accuracy(language);
        let sentence_accuracy = self.text_statistic.get_accuracy(language);
        let average_accuracy =
            (single_word_accuracy + word_pair_accuracy + sentence_accuracy) / Decimal::from(3);

        self.average_accuracies.insert(language, average_accuracy);
        average_accuracy
    }

    pub(super) fn create_report_data(
        &mut self,
        language: ScriptLanguage,
        average_accuracy: Decimal,
    ) -> Option<String> {
        let single_word_report = self
            .single_word_statistic
            .create_report_data(language, "single words");

        let word_pair_report = self
            .two_words_statistic
            .create_report_data(language, "word pairs");

        let sentence_report = self
            .text_statistic
            .create_report_data(language, "sentences");

        if average_accuracy.is_zero() {
            return None;
        }

        Some(formatdoc!(
            r#"
             ##### {:?} #####
 
             >>> Accuracy on average: {}
 
             {}
             {}
             {}
             "#,
            language,
            format_accuracy(average_accuracy),
            single_word_report,
            word_pair_report,
            sentence_report
        ))
    }

    pub(super) fn create_aggregated_report_row(&self, language: ScriptLanguage) -> String {
        let average_accuracy_column = self
            .average_accuracies
            .get(&language)
            .map(|&accuracy| {
                if accuracy > Decimal::zero() {
                    accuracy.round().to_string()
                } else {
                    "0".to_owned()
                }
            })
            .unwrap_or("-".to_owned());

        let single_words_accuracy_column = self
            .single_word_statistic
            .language_accuracies
            .get(&Some(language))
            .map(|a| a.round().to_string())
            .unwrap_or("-".to_owned());

        let word_pairs_accuracy_column = self
            .two_words_statistic
            .language_accuracies
            .get(&Some(language))
            .map(|a| format!("{a:.1}"))
            .unwrap_or("-".to_owned());

        let sentences_accuracy_column = self
            .text_statistic
            .language_accuracies
            .get(&Some(language))
            .map(|a| format!("{a:.1}"))
            .unwrap_or("-".to_owned());

        let mut res = average_accuracy_column;
        res.push(',');
        res.push_str(&single_words_accuracy_column);
        res.push(',');
        res.push_str(&word_pairs_accuracy_column);
        res.push(',');
        res.push_str(&sentences_accuracy_column);
        res
    }
}
