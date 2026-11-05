//! The replacement list: words the speech recognition gets wrong, especially names. A rule with `variants`
//! matches them as whole words, case ignored, with any space between the words of a variant; a rule with a
//! `pattern` is a regular expression for what variants cannot say. Rules apply top to bottom.

use regex::Regex;

use crate::config::Replacement;
use crate::Error;

pub struct Replacer {
    rules: Vec<(Regex, String)>,
}

impl Replacer {
    /// Compile the rules of the settings.
    ///
    /// # Errors
    /// A rule with neither or both of `variants` and `pattern`, an empty variant, or a pattern that is no
    /// regular expression.
    pub fn new(replacements: &[Replacement]) -> Result<Self, Error> {
        let rules = replacements.iter().map(compile).collect::<Result<_, _>>()?;
        Ok(Self { rules })
    }

    #[must_use]
    pub fn apply(&self, text: &str) -> String {
        self.rules.iter().fold(text.to_owned(), |text, (pattern, replacement)| {
            // NoExpand: a `$` in the replacement is written as it stands.
            pattern.replace_all(&text, regex::NoExpand(replacement)).into_owned()
        })
    }
}

fn compile(rule: &Replacement) -> Result<(Regex, String), Error> {
    let refuse = |why: &str| Error::Config(format!("replacement {:?}: {why}", rule.replacement));
    let source = match (&rule.pattern, rule.variants.is_empty()) {
        (Some(pattern), true) => pattern.clone(),
        (None, false) => {
            if rule.variants.iter().any(|variant| variant.trim().is_empty()) {
                return Err(refuse("an empty variant"));
            }
            let alternatives: Vec<String> = rule.variants.iter().map(|variant| words_pattern(variant)).collect();
            format!(r"(?i)\b(?:{})\b", alternatives.join("|"))
        }
        _ => return Err(refuse("either variants or pattern")),
    };
    let pattern = Regex::new(&source).map_err(|error| refuse(&error.to_string()))?;
    Ok((pattern, rule.replacement.clone()))
}

/// A variant as a pattern: its words literally, any whitespace between them.
fn words_pattern(variant: &str) -> String {
    variant.split_whitespace().map(regex::escape).collect::<Vec<_>>().join(r"\s+")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variants(list: &[&str], replacement: &str) -> Replacement {
        Replacement {
            variants: list.iter().map(|&variant| variant.to_owned()).collect(),
            pattern: None,
            replacement: replacement.into(),
        }
    }

    #[test]
    fn variants_match_whole_words_case_ignored() {
        let replacer = Replacer::new(&[variants(&["Komm mit", "Kommit"], "commit")]).unwrap();
        assert_eq!(replacer.apply("Dann komm  mit und push."), "Dann commit und push.");
        assert_eq!(replacer.apply("KOMMIT jetzt"), "commit jetzt");
        assert_eq!(replacer.apply("Kommitment bleibt"), "Kommitment bleibt");
    }

    #[test]
    fn umlauts_count_as_letters() {
        let replacer = Replacer::new(&[variants(&["Peki"], "Peuqui")]).unwrap();
        assert_eq!(replacer.apply("Hallo Peki, Pekiö bleibt."), "Hallo Peuqui, Pekiö bleibt.");
    }

    #[test]
    fn a_pattern_rule() {
        let rule = Replacement {
            variants: vec![],
            pattern: Some(r"(?i)\bagent[ -]?org?\b".into()),
            replacement: "Agent-Orc".into(),
        };
        let replacer = Replacer::new(&[rule]).unwrap();
        assert_eq!(replacer.apply("Frag Agent Org und agentor."), "Frag Agent-Orc und Agent-Orc.");
    }

    #[test]
    fn a_dollar_in_the_replacement_is_kept() {
        let replacer = Replacer::new(&[variants(&["Dollar"], "$1")]).unwrap();
        assert_eq!(replacer.apply("ein Dollar"), "ein $1");
    }

    #[test]
    fn rules_apply_top_to_bottom() {
        let replacer = Replacer::new(&[variants(&["a"], "b"), variants(&["b"], "c")]).unwrap();
        assert_eq!(replacer.apply("a"), "c");
    }

    #[test]
    fn a_rule_needs_exactly_one_kind() {
        let both = Replacement { variants: vec!["x".into()], pattern: Some("y".into()), replacement: "z".into() };
        let neither = Replacement { variants: vec![], pattern: None, replacement: "z".into() };
        assert!(Replacer::new(&[both]).is_err());
        assert!(Replacer::new(&[neither]).is_err());
        assert!(Replacer::new(&[variants(&[" "], "z")]).is_err());
    }
}
