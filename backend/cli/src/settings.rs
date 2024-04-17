use std::{fs::File, path::Path};

use anyhow::Result;
use regex::Regex;
use serde::{
    de::{Unexpected, Visitor},
    Deserialize, Serialize,
};
use size::Size;
use url::Url;

#[derive(Clone, Debug, PartialEq)]
pub struct StorageSizeLimit(pub Size);

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_lists: Vec<Url>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    #[serde(
        default = "default_storage_size_limit",
        skip_serializing_if = "is_default_storage_size_limit"
    )]
    pub storage_size_limit: StorageSizeLimit,
}

impl Settings {
    pub fn from_file_or_default(path: &Path) -> Result<Self> {
        if let Ok(file) = File::open(path) {
            Ok(serde_json::from_reader(file)?)
        } else {
            Ok(Default::default())
        }
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let file = File::create(path)?;

        Ok(serde_json::to_writer(file, self)?)
    }
}

fn default_storage_size_limit() -> StorageSizeLimit {
    StorageSizeLimit(Size::from_megabytes(2048))
}

fn is_default_storage_size_limit(size: &StorageSizeLimit) -> bool {
    *size == default_storage_size_limit()
}

impl Default for StorageSizeLimit {
    fn default() -> Self {
        Self(Size::from_bytes(0))
    }
}

impl Serialize for StorageSizeLimit {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for StorageSizeLimit {
    fn deserialize<D>(deserializer: D) -> std::prelude::v1::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as SerdeDeserialziationError;

        struct StorageSizeLimitVisitor;

        impl<'de> Visitor<'de> for StorageSizeLimitVisitor {
            type Value = StorageSizeLimit;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid size with dimensions (e.g. 2 GB, 2048 MB)")
            }

            fn visit_str<E>(self, v: &str) -> std::prelude::v1::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                // FIXME this might be supported by `Size` eventually, but for now we just use a regex
                let regex = Regex::new(r"(?<value>[\d.]+) *(?<dimension>GB|MB)").unwrap();
                let capture = regex.captures(v).ok_or_else(|| {
                    SerdeDeserialziationError::invalid_value(
                        Unexpected::Str(v),
                        &"a valid size with dimensions (e.g. 2 GB, 2048 MB)",
                    )
                })?;

                let value: f64 = capture["value"].parse().map_err(|_| {
                    SerdeDeserialziationError::invalid_value(
                        Unexpected::Str(v),
                        &"a valid float value as the size",
                    )
                })?;
                let dimension = &capture["dimension"];

                let size = match dimension {
                    "GB" => Size::from_gigabytes(value),
                    "MB" => Size::from_megabytes(value),
                    _ => panic!("unexpected dimension: {dimension}"),
                };

                Ok(StorageSizeLimit(size))
            }
        }

        deserializer.deserialize_str(StorageSizeLimitVisitor {})
    }
}
