use core::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

use nt_string::unicode_string::NtUnicodeString;
use serde::{de::Visitor, Deserialize, Serialize};

pub struct SerializableNtString(pub NtUnicodeString);

unsafe impl Send for SerializableNtString {}
unsafe impl Sync for SerializableNtString {}

impl SerializableNtString {
    pub fn new(nt_str: NtUnicodeString) -> Self {
        Self(nt_str)
    }

    pub fn empty() -> Self {
        Self(NtUnicodeString::new())
    }
}

impl Debug for SerializableNtString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_empty() {
            Display::fmt("[[[Empty]]]", f)
        } else {
            Display::fmt(&self.0, f)
        }
    }
}

impl Display for SerializableNtString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_empty() {
            Display::fmt("[[[Empty]]]", f)
        } else {
            Display::fmt(&self.0, f)
        }
    }
}

impl Clone for SerializableNtString {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Deref for SerializableNtString {
    type Target = NtUnicodeString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SerializableNtString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<NtUnicodeString> for SerializableNtString {
    fn from(value: NtUnicodeString) -> Self {
        Self(value)
    }
}

impl Serialize for SerializableNtString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.0.len() == 0 {
            let empty_buffer: &[u16] = &[];
            serializer.collect_seq(empty_buffer.iter())
        } else {
            serializer.collect_seq(self.0.as_slice().iter())
        }
    }
}

impl<'de> Deserialize<'de> for SerializableNtString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NtStringVisitor;

        impl<'de> Visitor<'de> for NtStringVisitor {
            type Value = SerializableNtString;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("a byte sequence representing UTF-16 encoded data")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let size = seq.size_hint().unwrap();

                let mut string = NtUnicodeString::with_capacity(size as _);

                loop {
                    let result = seq.next_element()?;
                    if let Some(element) = result {
                        string.try_push_u16(&[element]).unwrap();
                    } else {
                        break Ok(SerializableNtString(string));
                    }
                }
            }
        }

        deserializer.deserialize_seq(NtStringVisitor)
    }
}
