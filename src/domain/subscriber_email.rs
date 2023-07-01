use validator::validate_email;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(string: &str) -> Result<Self, String> {
        if !validate_email(string) {
            return Err(format!("{} email is invalid", string));
        }

        Ok(Self(string.to_owned()))
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SubscriberEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod test {
    use super::SubscriberEmail;
    use claims::{assert_err, assert_ok};
    use fake::{faker::internet::en::SafeEmail, Fake};
    use quickcheck::Arbitrary;
    use quickcheck_macros::quickcheck;

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(String);

    impl Arbitrary for ValidEmailFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            Self(SafeEmail().fake_with_rng(g))
        }
    }

    #[test]
    fn empty_string_is_invalid() {
        assert_err!(SubscriberEmail::parse(""));
    }

    #[test]
    fn string_without_at_symbol_is_invalid() {
        assert_err!(SubscriberEmail::parse("not-an-email.com"));
    }

    #[test]
    fn email_without_subject_is_invalid() {
        assert_err!(SubscriberEmail::parse("@missing-subject.net"));
    }

    #[quickcheck]
    fn valid_email_is_properly_parsed(email: ValidEmailFixture) {
        assert_ok!(SubscriberEmail::parse(&email.0));
    }
}
