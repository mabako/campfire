use serde::{de, Deserialize, Deserializer};
use std::fmt;
use std::marker::PhantomData;

pub mod utc_date {
    use chrono::{Date, NaiveDate, Utc};
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Date<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parsed = s.parse::<NaiveDate>().map(|s| Date::from_utc(s, Utc));
        match parsed {
            Ok(p) => Ok(Some(p)),
            Err(_) => Ok(None),
        }
    }
}

pub fn deserialize_tags<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrVec(PhantomData<Vec<String>>);

    impl<'de> de::Visitor<'de> for StringOrVec {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or list of strings")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value
                .split(',')
                .map(|item| item.trim().to_owned())
                .collect())
        }

        fn visit_seq<S>(self, visitor: S) -> Result<Self::Value, S::Error>
        where
            S: de::SeqAccess<'de>,
        {
            Deserialize::deserialize(de::value::SeqAccessDeserializer::new(visitor))
        }
    }

    deserializer.deserialize_any(StringOrVec(PhantomData))
}

#[cfg(test)]
mod tests {
    use crate::markdown::Frontmatter;

    #[test]
    fn deserialize_frontmatter_list_tags() {
        let frontmatter: Frontmatter =
            serde_yaml::from_str("title: Hello\ntags:\n- my\n- list\n- example").unwrap();
        assert_eq!(frontmatter.title.unwrap(), "Hello");
        assert_eq!(frontmatter.tags, ["my", "list", "example"]);
    }

    #[test]
    fn deserialize_frontmatter_inline_tags() {
        let frontmatter: Frontmatter =
            serde_yaml::from_str("title: Hello\ntags: my, inline, example").unwrap();
        assert_eq!(frontmatter.title.unwrap(), "Hello");
        assert_eq!(frontmatter.tags, ["my", "inline", "example"]);
    }
}
