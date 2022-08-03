use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::label::LabelError::LabelTooLong;

#[derive(Error, Debug)]
pub enum LabelError {
    #[error("Label is longer than 100 characters")]
    LabelTooLong(String),
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub struct Label(String);

impl Label {
    pub fn new<T: Into<String>>(value: T) -> Result<Self, LabelError> {
        let value = value.into();
        Label::validate_length(&value).map(|_| Self { 0: value })
    }

    pub fn create_latest_version_label(tag: &str, labels: Vec<Label>) -> anyhow::Result<Label> {
        let mut res: Vec<Label> = labels.into_iter().filter(|l| l.0.contains(tag)).collect();

        res.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        if let Some(latest_label) = res.last() {
            // + 1 for '_' separator.
            let splits: Vec<&str> = latest_label.0.rsplit("_").collect();
            let counter = splits[0].parse::<usize>()?;
            let counter = counter + 1;
            let label_value = format!("{}_{}", tag, counter);

            Label::new(label_value).map_err(anyhow::Error::from)
        } else {
            let label_value = format!("{}_1", tag);
            // This call can not cause an error.
            Label::new(label_value).map_err(anyhow::Error::from)
        }
    }

    fn validate_length(value: &str) -> Result<(), LabelError> {
        if value.len() > 100 {
            return Err(LabelTooLong(String::from(value)));
        } else {
            Ok(())
        }
    }

    pub fn update<T: Into<String>>(&mut self, value: T) -> Result<(), LabelError> {
        let value = value.into();
        Label::validate_length(&value).map(|_| self.0 = value.into())
    }

    pub fn str(&self) -> &String {
        &self.0
    }
}

impl TryFrom<String> for Label {
    type Error = LabelError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Label::new(value)
    }
}

impl Default for Label {
    fn default() -> Self {
        Label::new(String::new()).unwrap()
    }
}

impl Clone for Label {
    fn clone(&self) -> Self {
        Label::new(self.0.clone()).unwrap()
    }
}

impl Display for Label {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq<String> for Label {
    fn eq(&self, other: &String) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<str> for Label {
    fn eq(&self, other: &str) -> bool {
        self.0.eq(other)
    }
}

impl AsRef<str> for Label {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

#[cfg(test)]
mod test {
    use crate::label::Label;

    #[test]
    fn create_latest_version_label_should_return_label_with_highest_copy_counter_on_end() {
        let labels = vec![
            Label::new("GE-Proton7-22_1").unwrap(),
            Label::new("GE-Proton7-22_2").unwrap(),
            Label::new("GE-Proton7-22_3").unwrap(),
        ];
        let tag = "GE-Proton7-22";

        assert_eq!(
            Label::create_latest_version_label(tag, labels).unwrap(),
            Label::new("GE-Proton7-22").unwrap()
        )
    }
}
