use serde::{Deserialize, Deserializer};

/// Deserializes a field that can be:
/// - Not present in JSON → None (outer Option)
/// - Present as null → Some(None)
/// - Present with value → Some(Some(T))
///
/// This is useful for PATCH-style updates where we need to distinguish between
/// "field not provided" (keep existing) and "field explicitly set to null" (clear value).
pub fn deserialize_optional_nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    // If the field is present, deserialize it
    // If it's null, we get Some(None)
    // If it has a value, we get Some(Some(value))
    Option::<T>::deserialize(deserializer).map(Some)
}
