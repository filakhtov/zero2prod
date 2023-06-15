use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(name: &str) -> Result<Self, &str> {
        if name.trim().is_empty() {
            return Err("The name can't be empty");
        }

        if name.graphemes(true).count() > 256 {
            return Err("The name can't be longer than 256 characters");
        }

        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        if name
            .chars()
            .any(|char| forbidden_characters.contains(&char))
        {
            return Err("The name contains forbidden characters");
        }

        Ok(Self(name.to_owned()))
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use super::SubscriberName;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_256_character_long_name_is_valid() {
        let name = "ü".repeat(256);
        assert_ok!(SubscriberName::parse(&name));
    }

    #[test]
    fn a_name_longer_than_256_characters_is_invalid() {
        let name = "a".repeat(257);
        assert_err!(SubscriberName::parse(&name));
    }

    #[test]
    fn whitespace_only_name_is_invalid() {
        assert_err!(SubscriberName::parse("  "));
    }

    #[test]
    fn name_with_forbidden_characters_is_invalid() {
        assert_err!(SubscriberName::parse("(Forbidden Name"));
    }

    #[test]
    fn empty_string_is_invalid() {
        assert_err!(SubscriberName::parse(""));
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        assert_ok!(SubscriberName::parse("John Doe"));
    }
}
