//! Helpers for reading the short and the long form of a value.

use serde::Deserialize;
use serde::de::value::MapAccessDeserializer;
use serde::de::{Deserializer, Error, MapAccess, Visitor};
use std::fmt;
use std::marker::PhantomData;

/// Read a value that is a plain string or a map: `"A bare room."` or
/// `{ text: …, when: … }`.
///
/// `#[serde(untagged)]` would do the same, but it replaces every error of the
/// map form with "data did not match any variant". Here the map form reports
/// its own error, for example an unknown field with its name.
pub(crate) fn string_or_map<'de, D, M, T>(
    deserializer: D,
    expecting: &'static str,
    plain: fn(String) -> T,
    full: fn(M) -> T,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    M: Deserialize<'de>,
{
    struct StringOrMap<M, T> {
        expecting: &'static str,
        plain: fn(String) -> T,
        full: fn(M) -> T,
        marker: PhantomData<M>,
    }

    impl<'de, M: Deserialize<'de>, T> Visitor<'de> for StringOrMap<M, T> {
        type Value = T;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str(self.expecting)
        }

        fn visit_str<E: Error>(self, v: &str) -> Result<T, E> {
            Ok((self.plain)(v.to_string()))
        }

        fn visit_string<E: Error>(self, v: String) -> Result<T, E> {
            Ok((self.plain)(v))
        }

        fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
            M::deserialize(MapAccessDeserializer::new(map)).map(self.full)
        }
    }

    deserializer.deserialize_any(StringOrMap {
        expecting,
        plain,
        full,
        marker: PhantomData,
    })
}
