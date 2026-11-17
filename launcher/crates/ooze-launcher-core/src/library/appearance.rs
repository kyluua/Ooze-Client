use super::model::invalid;
use crate::util::Result;
use ooze_ui::theme_doc::{BaseColors, ThemeDoc};
use serde::{Deserialize, Serialize};

/// Global desktop appearance, using Claude's single product validator and schema.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Appearance {
    pub document: ThemeDoc,
}
impl Appearance {
    pub fn from_json(json: &str) -> Result<Self> {
        if json.len() > 65536 {
            return Err(invalid("Appearance import exceeds 64 KiB."));
        }
        let base = BaseColors::classic()
            .ok_or_else(|| invalid("The bundled base theme is unavailable."))?;
        let document=ThemeDoc::from_json(json,&base).map_err(|_|invalid("Invalid ooze.theme/1 appearance, unsupported token or insufficient accent contrast."))?;
        Ok(Self { document })
    }
    pub(crate) fn validate(&self) -> Result<()> {
        let base = BaseColors::classic()
            .ok_or_else(|| invalid("The bundled base theme is unavailable."))?;
        self.document
            .validate(&base)
            .map_err(|_| invalid("Appearance settings or accent contrast are invalid."))
    }
    pub fn to_json(&self) -> String {
        self.document.to_json()
    }
}
impl Serialize for Appearance {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let value: serde_json::Value =
            serde_json::from_str(&self.to_json()).map_err(serde::ser::Error::custom)?;
        value.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for Appearance {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        Self::from_json(&value.to_string()).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_appearance_round_trips_and_rejects_executable_or_unreadable_data() {
        let theme = Appearance::default();
        theme.validate().unwrap();
        assert_eq!(Appearance::from_json(&theme.to_json()).unwrap(), theme);
        assert!(
            Appearance::from_json(
                "{\"schema\":\"ooze.theme/1\",\"base\":\"ooze_classic\",\"script\":\"x\"}"
            )
            .is_err()
        );
        let mut invalid = theme.document;
        invalid.ui_scale = 999;
        assert!(Appearance { document: invalid }.validate().is_err());
    }
}
