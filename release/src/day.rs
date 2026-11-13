use std::fmt;
use std::str::FromStr;

pub struct Day(String);

impl FromStr for Day {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let shape = text.replace(|character: char| character.is_ascii_digit(), "0");
        if shape != "0000-00-00" {
            return Err(format!("`{text}` isn't a day written like 2026-11-13"));
        }
        Ok(Self(text.to_owned()))
    }
}

impl fmt::Display for Day {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
