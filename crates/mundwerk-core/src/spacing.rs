//! A space between two dictations into the same window, as Agent-Orc puts one before a dictation that follows
//! text. Mundwerk cannot read what a field of another program holds; it knows only what it typed there itself.
//! So: a space before the text when the last dictation went into the same window and did not end in whitespace.

#[derive(Debug, Default)]
pub struct Spacing {
    /// The window of the last dictation, and whether that text ended in whitespace.
    last: Option<(u64, bool)>,
}

impl Spacing {
    /// What goes before `text` in `window` (`""` or `" "`); remembers this dictation for the next one.
    pub fn prefix(&mut self, window: u64, text: &str) -> &'static str {
        let glue = match self.last {
            Some((last_window, ended_in_space)) => last_window == window && !ended_in_space && !starts_with_space(text),
            None => false,
        };
        let ends_in_space = text.chars().last().is_some_and(char::is_whitespace);
        self.last = Some((window, ends_in_space));
        if glue {
            " "
        } else {
            ""
        }
    }

    /// Forget the last dictation (the user typed or clicked elsewhere, as far as Mundwerk can tell).
    pub fn forget(&mut self) {
        self.last = None;
    }
}

fn starts_with_space(text: &str) -> bool {
    text.chars().next().is_some_and(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_dictation_gets_no_space() {
        assert_eq!(Spacing::default().prefix(1, "Hallo."), "");
    }

    #[test]
    fn a_second_one_into_the_same_window_does() {
        let mut spacing = Spacing::default();
        spacing.prefix(1, "Erledigt.");
        assert_eq!(spacing.prefix(1, "Dann weiter."), " ");
    }

    #[test]
    fn another_window_gets_none() {
        let mut spacing = Spacing::default();
        spacing.prefix(1, "Erledigt.");
        assert_eq!(spacing.prefix(2, "Neu."), "");
    }

    #[test]
    fn text_that_ended_in_a_space_needs_none() {
        let mut spacing = Spacing::default();
        spacing.prefix(1, "Erledigt. ");
        assert_eq!(spacing.prefix(1, "Dann."), "");
    }

    #[test]
    fn after_forgetting_none() {
        let mut spacing = Spacing::default();
        spacing.prefix(1, "Erledigt.");
        spacing.forget();
        assert_eq!(spacing.prefix(1, "Dann."), "");
    }
}
