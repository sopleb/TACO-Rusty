use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(into = "u8", try_from = "u8")]
pub enum AlertType {
    #[default]
    Ranged = 0,
    Custom = 1,
}

impl From<AlertType> for u8 {
    fn from(v: AlertType) -> u8 {
        v as u8
    }
}

impl TryFrom<u8> for AlertType {
    type Error = String;
    fn try_from(v: u8) -> Result<Self, String> {
        match v {
            0 => Ok(Self::Ranged),
            1 => Ok(Self::Custom),
            _ => Ok(Self::Ranged),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(into = "u8", try_from = "u8")]
pub enum RangeAlertOperator {
    #[default]
    Equal = 0,
    LessThan = 1,
    GreaterThan = 2,
    LessThanOrEqual = 3,
    GreaterThanOrEqual = 4,
}

impl From<RangeAlertOperator> for u8 {
    fn from(v: RangeAlertOperator) -> u8 {
        v as u8
    }
}

impl TryFrom<u8> for RangeAlertOperator {
    type Error = String;
    fn try_from(v: u8) -> Result<Self, String> {
        match v {
            0 => Ok(Self::Equal),
            1 => Ok(Self::LessThan),
            2 => Ok(Self::GreaterThan),
            3 => Ok(Self::LessThanOrEqual),
            4 => Ok(Self::GreaterThanOrEqual),
            _ => Ok(Self::Equal),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(into = "u8", try_from = "u8")]
pub enum RangeAlertType {
    #[default]
    Home = 0,
    System = 1,
    Character = 2,
    AnyCharacter = 3,
    None = 4,
    AnyFollowedCharacter = 5,
}

impl From<RangeAlertType> for u8 {
    fn from(v: RangeAlertType) -> u8 {
        v as u8
    }
}

impl TryFrom<u8> for RangeAlertType {
    type Error = String;
    fn try_from(v: u8) -> Result<Self, String> {
        match v {
            0 => Ok(Self::Home),
            1 => Ok(Self::System),
            2 => Ok(Self::Character),
            3 => Ok(Self::AnyCharacter),
            4 => Ok(Self::None),
            5 => Ok(Self::AnyFollowedCharacter),
            _ => Ok(Self::Home),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlertTrigger {
    #[serde(rename = "type")]
    pub alert_type: AlertType,
    pub upper_limit_operator: RangeAlertOperator,
    pub lower_limit_operator: RangeAlertOperator,
    pub upper_range: i32,
    pub lower_range: i32,
    pub range_to: RangeAlertType,
    #[serde(default)]
    pub character_name: String,
    #[serde(default = "default_neg_one")]
    pub system_id: i32,
    #[serde(default = "default_neg_one")]
    pub sound_id: i32,
    #[serde(default)]
    pub sound_path: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub repeat_interval: u32,
    #[serde(default)]
    pub system_name: String,
    #[serde(skip)]
    pub trigger_time: Option<DateTime<Utc>>,
}

fn default_neg_one() -> i32 {
    -1
}
fn default_true() -> bool {
    true
}

impl Default for AlertTrigger {
    fn default() -> Self {
        Self {
            alert_type: AlertType::Ranged,
            upper_limit_operator: RangeAlertOperator::Equal,
            lower_limit_operator: RangeAlertOperator::Equal,
            upper_range: 0,
            lower_range: 0,
            range_to: RangeAlertType::Home,
            character_name: String::new(),
            system_id: -1,
            sound_id: -1,
            sound_path: String::new(),
            enabled: true,
            text: String::new(),
            repeat_interval: 0,
            system_name: String::new(),
            trigger_time: None,
        }
    }
}

impl fmt::Display for AlertTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.alert_type {
            AlertType::Ranged => {
                let op = if self.upper_limit_operator == RangeAlertOperator::Equal {
                    "Range = "
                } else {
                    "Range <= "
                };
                write!(f, "{}{}", op, self.upper_range)?;

                if self.upper_limit_operator == RangeAlertOperator::Equal
                    || (self.lower_range == 0
                        && self.lower_limit_operator == RangeAlertOperator::GreaterThanOrEqual)
                {
                    let label = if self.upper_range == 1 {
                        " jump from: "
                    } else {
                        " jumps from: "
                    };
                    write!(f, "{}", label)?;
                } else {
                    let label = if self.upper_range == 1 {
                        " jump and"
                    } else {
                        " jumps and"
                    };
                    let lower_op = if self.lower_limit_operator == RangeAlertOperator::GreaterThan
                    {
                        " > "
                    } else {
                        " >= "
                    };
                    let lower_label = if self.lower_range == 1 {
                        " jump from: "
                    } else {
                        " jumps from: "
                    };
                    write!(f, "{}{}{}{}", label, lower_op, self.lower_range, lower_label)?;
                }

                match self.range_to {
                    RangeAlertType::Home | RangeAlertType::System => {
                        if self.system_id == -1 {
                            write!(f, "Home")?;
                        } else {
                            write!(f, "{}", self.system_name)?;
                        }
                    }
                    RangeAlertType::AnyFollowedCharacter => {
                        write!(f, "Any Followed Character")?;
                    }
                    RangeAlertType::AnyCharacter => write!(f, "Any Character")?,
                    _ => write!(f, "{}", self.character_name)?,
                }

                if self.sound_id == -1 {
                    write!(f, " (Custom Sound)")?;
                } else {
                    write!(f, " ({})", self.sound_path)?;
                }
            }
            AlertType::Custom => {
                write!(f, "When \"{}\" is seen, play (", self.text)?;
                if self.sound_id == -1 {
                    write!(f, "Custom Sound")?;
                } else {
                    write!(f, "{}", self.sound_path)?;
                }
                write!(f, "). Trigger ")?;
                if self.repeat_interval == 0 {
                    write!(f, "every detection.")?;
                } else {
                    write!(f, "every {}", self.repeat_interval)?;
                    if self.repeat_interval == 1 {
                        write!(f, " sec.")?;
                    } else {
                        write!(f, " secs.")?;
                    }
                }
            }
        }
        Ok(())
    }
}
