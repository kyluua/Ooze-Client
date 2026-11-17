//! Typed module settings with validation. Every value that enters a profile (from the UI or an
//! imported file) passes `SettingDef::validate`; invalid values never reach game code.

use crate::json::Json;

#[derive(Debug)]
pub enum SettingKind {
    Bool {
        default: bool,
    },
    Int {
        min: i64,
        max: i64,
        step: i64,
        default: i64,
    },
    Float {
        min: f64,
        max: f64,
        step: f64,
        default: f64,
    },
    Choice {
        options: &'static [&'static str],
        default: &'static str,
    },
    /// 0xAARRGGBB.
    Color {
        default: u32,
    },
    /// A vanilla key name such as `key.keyboard.c` or `key.mouse.middle`.
    Key {
        default: &'static str,
    },
}

#[derive(Debug)]
pub struct SettingDef {
    pub key: &'static str,
    pub label_key: &'static str,
    pub kind: SettingKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Choice(String),
    Color(u32),
    Key(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingError {
    WrongType,
    OutOfRange,
    NotAnOption,
    BadColor,
    BadKey,
}

impl Value {
    pub fn to_json(&self) -> Json {
        match self {
            Self::Bool(b) => Json::Bool(*b),
            Self::Int(i) => Json::Number(*i as f64),
            Self::Float(f) => Json::Number(*f),
            Self::Choice(s) | Self::Key(s) => Json::String(s.clone()),
            Self::Color(c) => Json::String(format!("#{c:08X}")),
        }
    }

    /// A short, locale-neutral rendering for "what this changes" lists.
    pub fn display(&self) -> String {
        match self {
            Self::Bool(b) => b.to_string(),
            Self::Int(i) => i.to_string(),
            Self::Float(f) => format!("{f}"),
            Self::Choice(s) | Self::Key(s) => s.clone(),
            Self::Color(c) => format!("#{c:08X}"),
        }
    }
}

impl SettingDef {
    pub fn default(&self) -> Value {
        match &self.kind {
            SettingKind::Bool { default } => Value::Bool(*default),
            SettingKind::Int { default, .. } => Value::Int(*default),
            SettingKind::Float { default, .. } => Value::Float(*default),
            SettingKind::Choice { default, .. } => Value::Choice((*default).to_owned()),
            SettingKind::Color { default } => Value::Color(*default),
            SettingKind::Key { default } => Value::Key((*default).to_owned()),
        }
    }

    pub fn validate(&self, json: &Json) -> Result<Value, SettingError> {
        match &self.kind {
            SettingKind::Bool { .. } => json
                .as_bool()
                .map(Value::Bool)
                .ok_or(SettingError::WrongType),
            SettingKind::Int { min, max, step, .. } => {
                let n = json.as_f64().ok_or(SettingError::WrongType)?;
                if n.fract() != 0.0 {
                    return Err(SettingError::WrongType);
                }
                let n = n as i64;
                if n < *min || n > *max || (n - min) % step != 0 {
                    return Err(SettingError::OutOfRange);
                }
                Ok(Value::Int(n))
            }
            SettingKind::Float { min, max, step, .. } => {
                let n = json.as_f64().ok_or(SettingError::WrongType)?;
                let steps = (n - min) / step;
                if n < *min || n > *max || (steps - steps.round()).abs() > 1e-6 {
                    return Err(SettingError::OutOfRange);
                }
                Ok(Value::Float(n))
            }
            SettingKind::Choice { options, .. } => {
                let s = json.as_str().ok_or(SettingError::WrongType)?;
                if options.contains(&s) {
                    Ok(Value::Choice(s.to_owned()))
                } else {
                    Err(SettingError::NotAnOption)
                }
            }
            SettingKind::Color { .. } => {
                let s = json.as_str().ok_or(SettingError::WrongType)?;
                let hex = s
                    .strip_prefix('#')
                    .filter(|h| h.len() == 8)
                    .ok_or(SettingError::BadColor)?;
                u32::from_str_radix(hex, 16)
                    .map(Value::Color)
                    .map_err(|_| SettingError::BadColor)
            }
            SettingKind::Key { .. } => {
                let s = json.as_str().ok_or(SettingError::WrongType)?;
                if is_key_name(s) {
                    Ok(Value::Key(s.to_owned()))
                } else {
                    Err(SettingError::BadKey)
                }
            }
        }
    }
}

/// Vanilla key names: `key.keyboard.<name>` or `key.mouse.<name>`, lowercase ASCII.
pub fn is_key_name(s: &str) -> bool {
    let rest = s
        .strip_prefix("key.keyboard.")
        .or_else(|| s.strip_prefix("key.mouse."));
    rest.is_some_and(|r| {
        !r.is_empty()
            && r.len() <= 32
            && r.bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'.' || c == b'_')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const INT: SettingDef = SettingDef {
        key: "n",
        label_key: "n",
        kind: SettingKind::Int {
            min: 5,
            max: 50,
            step: 5,
            default: 20,
        },
    };
    const FLOAT: SettingDef = SettingDef {
        key: "f",
        label_key: "f",
        kind: SettingKind::Float {
            min: 2.0,
            max: 10.0,
            step: 0.5,
            default: 4.0,
        },
    };

    #[test]
    fn numbers_respect_range_and_step() {
        assert_eq!(INT.validate(&Json::Number(25.0)), Ok(Value::Int(25)));
        assert_eq!(
            INT.validate(&Json::Number(26.0)),
            Err(SettingError::OutOfRange)
        );
        assert_eq!(
            INT.validate(&Json::Number(55.0)),
            Err(SettingError::OutOfRange)
        );
        assert_eq!(
            INT.validate(&Json::Number(2.5)),
            Err(SettingError::WrongType)
        );
        assert_eq!(
            INT.validate(&Json::String("25".into())),
            Err(SettingError::WrongType)
        );
        assert_eq!(FLOAT.validate(&Json::Number(3.5)), Ok(Value::Float(3.5)));
        assert_eq!(
            FLOAT.validate(&Json::Number(3.3)),
            Err(SettingError::OutOfRange)
        );
    }

    #[test]
    fn colors_and_keys_are_strict() {
        let color = SettingDef {
            key: "c",
            label_key: "c",
            kind: SettingKind::Color { default: 0 },
        };
        assert_eq!(
            color.validate(&Json::String("#FF00FF80".into())),
            Ok(Value::Color(0xFF00_FF80))
        );
        assert_eq!(
            color.validate(&Json::String("#FFF".into())),
            Err(SettingError::BadColor)
        );
        assert_eq!(
            Value::Color(0xFF00_FF80).to_json(),
            Json::String("#FF00FF80".into())
        );
        assert!(
            is_key_name("key.keyboard.c")
                && is_key_name("key.mouse.middle")
                && is_key_name("key.keyboard.left.shift")
        );
        assert!(
            !is_key_name("key.keyboard.") && !is_key_name("C") && !is_key_name("key.keyboard.C")
        );
    }
}
