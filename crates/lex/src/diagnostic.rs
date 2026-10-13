// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed diagnostic presentation, exact JSON arguments and validated rendering.

use std::fmt;

/// A presentation argument's value, independent of its human rendering.
#[derive(Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiagnosticValue {
    /// Verbatim text, such as a reference or a codec name.
    Text(String),
    /// An exact unsigned counter or address.
    Unsigned(u64),
    /// An exact signed integer.
    Signed(i64),
    /// A Boolean condition.
    Boolean(bool),
    /// One Unicode scalar value.
    Character(char),
}

crate::resident_clone!(DiagnosticValue);

impl DiagnosticValue {
    /// Copy immutable typed presentation data through original physical admission.
    /// The caller retains this memory with the returned payload.
    ///
    /// # Errors
    /// Returns the first physical storage refusal before publication.
    pub fn clone_with_memory<S: crate::allocation::Admission + ?Sized>(
        &self,
        memory: &mut crate::allocation::Memory<'_, S>,
    ) -> Result<Self, crate::allocation::StorageError> {
        Ok(match self {
            Self::Text(value) => Self::Text(memory.string(value)?),
            Self::Unsigned(value) => Self::Unsigned(*value),
            Self::Signed(value) => Self::Signed(*value),
            Self::Boolean(value) => Self::Boolean(*value),
            Self::Character(value) => Self::Character(*value),
        })
    }

    fn to_json(&self) -> crate::json::Value {
        use crate::json::{Object, Value};
        let (kind, value) = match self {
            Self::Text(value) => ("text", Value::from(value.clone())),
            Self::Unsigned(value) => ("unsigned", Value::from(value.to_string())),
            Self::Signed(value) => ("signed", Value::from(value.to_string())),
            Self::Boolean(value) => ("boolean", Value::Bool(*value)),
            Self::Character(value) => ("character", Value::from(value.to_string())),
        };
        let mut object = Object::new();
        object.insert("kind".to_owned(), Value::from(kind));
        // Decimal strings preserve exact counters in JSON hosts using binary64.
        object.insert("value".to_owned(), value);
        Value::Object(object)
    }
}

struct ValueDisplay<'a>(&'a DiagnosticValue, bool);

impl fmt::Display for ValueDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            DiagnosticValue::Text(value) if self.1 => write!(f, "{value:?}"),
            DiagnosticValue::Text(value) => f.write_str(value),
            DiagnosticValue::Unsigned(value) => fmt::Display::fmt(value, f),
            DiagnosticValue::Signed(value) => fmt::Display::fmt(value, f),
            DiagnosticValue::Boolean(value) => fmt::Display::fmt(value, f),
            DiagnosticValue::Character(value) if self.1 => write!(f, "{value:?}"),
            DiagnosticValue::Character(value) => fmt::Display::fmt(value, f),
        }
    }
}

/// One named presentation argument. Names are validated with the template.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct DiagnosticParameter {
    name: String,
    value: DiagnosticValue,
}

crate::resident_clone!(DiagnosticParameter);

impl DiagnosticParameter {
    /// Copy the original name and typed value after each allocation is admitted.
    ///
    /// # Errors
    /// Returns physical refusal, retaining already admitted scratch in the caller.
    pub fn clone_with_memory<S: crate::allocation::Admission + ?Sized>(
        &self,
        memory: &mut crate::allocation::Memory<'_, S>,
    ) -> Result<Self, crate::allocation::StorageError> {
        Ok(Self {
            name: memory.string(&self.name)?,
            value: self.value.clone_with_memory(memory)?,
        })
    }

    /// Construct a named typed argument.
    pub fn new(name: impl Into<String>, value: DiagnosticValue) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }

    /// Copy an argument name only after its destination is admitted.
    ///
    /// # Errors
    /// Returns checked physical storage refusal.
    pub fn try_new_with_memory<S: crate::allocation::Admission + ?Sized>(
        name: &str,
        value: DiagnosticValue,
        memory: &mut crate::allocation::Memory<'_, S>,
    ) -> Result<Self, crate::allocation::StorageError> {
        Ok(Self {
            name: memory.string(name)?,
            value,
        })
    }

    /// The template argument's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The original typed value.
    pub fn value(&self) -> &DiagnosticValue {
        &self.value
    }
}

/// A refused presentation/template contract.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiagnosticPresentationError {
    /// Physical storage refusal while constructing presentation data.
    Storage(crate::allocation::StorageError),
    /// The message identity is empty.
    EmptyIdentity,
    /// An argument name is empty or not an ASCII identifier.
    InvalidParameter(String),
    /// A name was supplied more than once.
    DuplicateParameter(String),
    /// The template has an unmatched brace or unsupported formatting operation.
    InvalidTemplate,
    /// A template argument was not supplied.
    MissingParameter(String),
    /// A supplied argument is absent from the template.
    UnusedParameter(String),
    /// Secondary detail itself carried nested detail.
    NestedDetail,
}

impl DiagnosticPresentationError {
    /// Extract storage refusal from a presentation built with a fixed valid template.
    ///
    /// # Panics
    /// Panics if fixed source templates and their typed arguments disagree.
    #[must_use]
    pub fn fixed_template_storage(&self) -> crate::allocation::StorageError {
        match self {
            Self::Storage(error) => *error,
            _ => panic!("fixed diagnostic templates and typed arguments disagree"),
        }
    }
}

impl fmt::Display for DiagnosticPresentationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => error.fmt(f),
            Self::EmptyIdentity => f.write_str("diagnostic message identity is empty"),
            Self::InvalidParameter(name) => write!(f, "invalid diagnostic parameter {name:?}"),
            Self::DuplicateParameter(name) => write!(f, "duplicate diagnostic parameter {name:?}"),
            Self::InvalidTemplate => f.write_str("invalid diagnostic English template"),
            Self::MissingParameter(name) => write!(f, "missing diagnostic parameter {name:?}"),
            Self::UnusedParameter(name) => write!(f, "unused diagnostic parameter {name:?}"),
            Self::NestedDetail => {
                f.write_str("diagnostic secondary detail cannot nest further detail")
            }
        }
    }
}

impl std::error::Error for DiagnosticPresentationError {}

crate::variant_from!(DiagnosticPresentationError { Storage(crate::allocation::StorageError) });

/// Validated, extensible presentation data. Its identity is distinct from a code:
/// one diagnostic code can cover multiple message templates.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct DiagnosticPresentation {
    message_id: String,
    parameters: Vec<DiagnosticParameter>,
    english: String,
    detail: Option<Box<Self>>,
}

crate::resident_clone!(DiagnosticPresentation);

impl DiagnosticPresentation {
    /// Copy the validated identity, typed arguments and verbatim English.
    /// Secondary detail is bounded by the existing construction invariant.
    ///
    /// # Errors
    /// Returns original physical refusal before publishing any replacement.
    pub fn clone_with_memory<S: crate::allocation::Admission + ?Sized>(
        &self,
        memory: &mut crate::allocation::Memory<'_, S>,
    ) -> Result<Self, crate::allocation::StorageError> {
        let mut parameters = Vec::new();
        memory.reserve(&mut parameters, self.parameters.len())?;
        for parameter in &self.parameters {
            parameters.push(parameter.clone_with_memory(memory)?);
        }
        Ok(Self {
            message_id: memory.string(&self.message_id)?,
            parameters,
            english: memory.string(&self.english)?,
            detail: self
                .detail
                .as_deref()
                .map(|detail| {
                    let detail = detail.clone_with_memory(memory)?;
                    memory.boxed(detail)
                })
                .transpose()?,
        })
    }

    /// Validate a template and its exact argument set, rendering English once.
    /// Placeholders are `{name}` or `{name:?}`; doubled braces are literals.
    ///
    /// # Errors
    /// Refuses empty identities, invalid/duplicate names and template/argument drift.
    pub fn new(
        message_id: impl Into<String>,
        english_template: &str,
        parameters: Vec<DiagnosticParameter>,
    ) -> Result<Self, DiagnosticPresentationError> {
        let message_id = message_id.into();
        let mut resident = crate::allocation::Resident;
        let mut memory = crate::allocation::Memory::new(&mut resident);
        Self::try_new_with_memory(&message_id, english_template, parameters, &mut memory)
    }

    /// Validate and render using the original admitted argument owners.
    /// Argument arrays and their text must already belong to this memory.
    ///
    /// # Errors
    /// Returns a template-contract error or a typed physical storage refusal.
    pub fn try_new_with_memory<S: crate::allocation::Admission + ?Sized>(
        message_id: &str,
        english_template: &str,
        parameters: Vec<DiagnosticParameter>,
        memory: &mut crate::allocation::Memory<'_, S>,
    ) -> Result<Self, DiagnosticPresentationError> {
        let message_id = memory.string(message_id)?;
        if message_id.is_empty() {
            return Err(DiagnosticPresentationError::EmptyIdentity);
        }
        for (index, parameter) in parameters.iter().enumerate() {
            if !parameter
                .name
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                || !parameter
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            {
                return Err(DiagnosticPresentationError::InvalidParameter(
                    memory.string(&parameter.name)?,
                ));
            }
            if parameters[..index]
                .iter()
                .any(|other| other.name == parameter.name)
            {
                return Err(DiagnosticPresentationError::DuplicateParameter(
                    memory.string(&parameter.name)?,
                ));
            }
        }
        let mut used = memory.collect(std::iter::repeat_n(false, parameters.len()))?;
        let mut english = String::new();
        let mut chars = english_template.chars().peekable();
        while let Some(character) = chars.next() {
            if matches!(character, '{' | '}') && chars.peek() == Some(&character) {
                chars.next();
                memory.push_char(&mut english, character)?;
                continue;
            }
            if character == '}' {
                return Err(DiagnosticPresentationError::InvalidTemplate);
            }
            if character != '{' {
                memory.push_char(&mut english, character)?;
                continue;
            }
            let mut token = String::new();
            loop {
                match chars.next() {
                    Some('}') => break,
                    Some('{') | None => return Err(DiagnosticPresentationError::InvalidTemplate),
                    Some(next) => memory.push_char(&mut token, next)?,
                }
            }
            let (name, debug) = token
                .strip_suffix(":?")
                .map_or((token.as_str(), false), |name| (name, true));
            if name.contains(':') || name.is_empty() {
                return Err(DiagnosticPresentationError::InvalidTemplate);
            }
            let index = parameters
                .iter()
                .position(|parameter| parameter.name == name)
                .map_or_else(
                    || {
                        Err(DiagnosticPresentationError::MissingParameter(
                            memory.string(name)?,
                        ))
                    },
                    Ok,
                )?;
            used[index] = true;
            let rendered = memory.format(&ValueDisplay(&parameters[index].value, debug))?;
            memory.push_str(&mut english, &rendered)?;
            memory.release_string(rendered)?;
            memory.release_string(token)?;
        }
        if let Some(index) = used.iter().position(|used| !used) {
            return Err(DiagnosticPresentationError::UnusedParameter(
                memory.string(&parameters[index].name)?,
            ));
        }
        memory.release_vec(used)?;
        Ok(Self {
            message_id,
            parameters,
            english,
            detail: None,
        })
    }

    /// Attach independently structured secondary detail.
    ///
    /// # Errors
    /// Refuses nested secondary detail, bounding clone/drop/render stack depth.
    pub fn with_detail(mut self, detail: Self) -> Result<Self, DiagnosticPresentationError> {
        if detail.detail.is_some() {
            return Err(DiagnosticPresentationError::NestedDetail);
        }
        self.detail = Some(Box::new(detail));
        Ok(self)
    }

    /// Attach original admitted detail through one fallible boxed allocation.
    ///
    /// # Errors
    /// Refuses nested detail or physical storage admission/allocation.
    pub fn try_with_detail_with_memory<S: crate::allocation::Admission + ?Sized>(
        mut self,
        detail: Self,
        memory: &mut crate::allocation::Memory<'_, S>,
    ) -> Result<Self, DiagnosticPresentationError> {
        if detail.detail.is_some() {
            return Err(DiagnosticPresentationError::NestedDetail);
        }
        self.detail = Some(memory.boxed(detail)?);
        Ok(self)
    }

    /// Exact heap layouts retained by this immutable presentation, excluding
    /// the outer value itself. Includes spare parameter capacity, text arguments
    /// and independently boxed secondary detail.
    #[must_use]
    pub fn retained_bytes(&self) -> Option<usize> {
        let mut bytes = 0usize;
        let mut current = Some(self);
        while let Some(value) = current {
            bytes = bytes
                .checked_add(value.message_id.capacity())?
                .checked_add(value.english.capacity())?
                .checked_add(
                    value
                        .parameters
                        .capacity()
                        .checked_mul(size_of::<DiagnosticParameter>())?,
                )?;
            for parameter in &value.parameters {
                bytes = bytes.checked_add(parameter.name.capacity())?;
                if let DiagnosticValue::Text(text) = &parameter.value {
                    bytes = bytes.checked_add(text.capacity())?;
                }
            }
            current = value.detail.as_deref();
            if current.is_some() {
                bytes = bytes.checked_add(size_of::<Self>())?;
            }
        }
        Some(bytes)
    }

    /// Stable message identity used by a presentation catalogue.
    pub fn message_id(&self) -> &str {
        &self.message_id
    }
    /// Named typed arguments, in producer order.
    pub fn parameters(&self) -> &[DiagnosticParameter] {
        &self.parameters
    }
    /// The validated compatibility English rendering.
    pub fn english(&self) -> &str {
        &self.english
    }
    /// Independently structured secondary detail, when supplied.
    pub fn detail(&self) -> Option<&Self> {
        self.detail.as_deref()
    }

    /// A machine record containing identity and original typed parameters.
    /// Exact integers use typed decimal strings to survive JSON binary64 hosts.
    pub fn to_json(&self) -> crate::json::Value {
        use crate::json::{Object, Value};
        let mut object = Object::new();
        object.insert("messageId".to_owned(), Value::from(self.message_id.clone()));
        let mut parameters = Object::new();
        for parameter in &self.parameters {
            parameters.insert(parameter.name.clone(), parameter.value.to_json());
        }
        object.insert("parameters".to_owned(), Value::Object(parameters));
        if let Some(detail) = &self.detail {
            object.insert("detail".to_owned(), detail.to_json());
        }
        Value::Object(object)
    }
}
