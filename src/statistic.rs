use crate::format_accuracy;
use ahash::AHashMap;
use alphabet_detector::ScriptLanguage;
use fraction::{Decimal, Zero};
use indoc::formatdoc;
use itertools::Itertools;

#[derive(Default)]
pub(super) struct Statistic {
    language_counts: AHashMap<Option<ScriptLanguage>, u32>,
    pub(super) language_accuracies: AHashMap<Option<ScriptLanguage>, Decimal>,
    entity_count: u32,
    entity_length_count: u32,
}

impl Statistic {
    #[inline(always)]
    pub(super) fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub(super) fn add_language_count(&mut self, language: Option<ScriptLanguage>) {
        *self.language_counts.entry(language).or_insert(0) += 1;
    }

    #[inline]
    pub(super) fn add_entity_count(&mut self) {
        self.entity_count += 1;
    }

    #[inline]
    pub(super) fn add_entity_length_count(&mut self, entity: &str) {
        self.entity_length_count += entity.chars().count() as u32;
    }

    pub(super) fn map_counts_to_accuracy_values(&mut self) {
        let sum_of_counts: u32 = self.language_counts.values().sum();
        self.language_accuracies = self
            .language_counts
            .iter()
            .map(|(language, count)| {
                (
                    *language,
                    Decimal::from(*count) / Decimal::from(sum_of_counts) * Decimal::from(100),
                )
            })
            .collect();
    }

    pub(super) fn get_accuracy(&self, language: ScriptLanguage) -> Decimal {
        self.language_accuracies
            .get(&Some(language))
            .copied()
            .unwrap_or(Decimal::zero())
    }

    pub(super) fn create_report_data(&self, language: ScriptLanguage, description: &str) -> String {
        let accuracy = self.get_accuracy(language);

        let average_length =
            ((self.entity_length_count as f64) / (self.entity_count as f64)).round();

        formatdoc!(
            r#"
                 >> Detection of {} {} (average length: {} chars)
                 Accuracy: {}
                 Erroneously classified as {}
                 "#,
            self.entity_count,
            description,
            average_length,
            format_accuracy(accuracy),
            self.format_language_accuracies(language)
        )
    }

    fn format_language_accuracies(&self, language: ScriptLanguage) -> String {
        self.language_accuracies
            .iter()
            .filter(|(lang, _)| **lang != Some(language))
            .sorted_by(
                |(first_lang, &first_accuracy), (second_lang, &second_accuracy)| {
                    let sorted_by_accuracy = second_accuracy.partial_cmp(&first_accuracy).unwrap();
                    let sorted_by_language = first_lang.partial_cmp(second_lang).unwrap();
                    sorted_by_accuracy.then(sorted_by_language)
                },
            )
            .map(|(lang, &accuracy)| {
                let formatted_lang = if lang.is_some() {
                    format!("{:?}", lang.as_ref().unwrap())
                } else {
                    "Unknown (multiple)".to_owned()
                };
                format!("{formatted_lang}: {accuracy:.2}%")
            })
            .join(", ")
    }
}
