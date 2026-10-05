// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed diagnostic presentation, exact JSON arguments and validated rendering.

use std::fmt;

/// A presentation argument's value, independent of its human rendering.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

impl DiagnosticValue {
    fn render(&self, debug: bool) -> String {
        match self {
            Self::Text(value) if debug => format!("{value:?}"),
            Self::Text(value) => value.clone(),
            Self::Unsigned(value) => value.to_string(),
            Self::Signed(value) => value.to_string(),
            Self::Boolean(value) => value.to_string(),
            Self::Character(value) if debug => format!("{value:?}"),
            Self::Character(value) => value.to_string(),
        }
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

/// One named presentation argument. Names are validated with the template.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagnosticParameter {
    name: String,
    value: DiagnosticValue,
}

impl DiagnosticParameter {
    /// Construct a named typed argument.
    pub fn new(name: impl Into<String>, value: DiagnosticValue) -> Self {
        Self {
            name: name.into(),
            value,
        }
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

impl fmt::Display for DiagnosticPresentationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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

/// Validated, extensible presentation data. Its identity is distinct from a code:
/// one diagnostic code can cover multiple message templates.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagnosticPresentation {
    message_id: String,
    parameters: Vec<DiagnosticParameter>,
    english: String,
    detail: Option<Box<Self>>,
}

impl DiagnosticPresentation {
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
                    parameter.name.clone(),
                ));
            }
            if parameters[..index]
                .iter()
                .any(|other| other.name == parameter.name)
            {
                return Err(DiagnosticPresentationError::DuplicateParameter(
                    parameter.name.clone(),
                ));
            }
        }
        let mut used = vec![false; parameters.len()];
        let mut english = String::with_capacity(english_template.len());
        let mut chars = english_template.chars().peekable();
        while let Some(character) = chars.next() {
            if matches!(character, '{' | '}') && chars.peek() == Some(&character) {
                chars.next();
                english.push(character);
                continue;
            }
            if character == '}' {
                return Err(DiagnosticPresentationError::InvalidTemplate);
            }
            if character != '{' {
                english.push(character);
                continue;
            }
            let mut token = String::new();
            loop {
                match chars.next() {
                    Some('}') => break,
                    Some('{') | None => return Err(DiagnosticPresentationError::InvalidTemplate),
                    Some(next) => token.push(next),
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
                .ok_or_else(|| DiagnosticPresentationError::MissingParameter(name.to_owned()))?;
            used[index] = true;
            english.push_str(&parameters[index].value.render(debug));
        }
        if let Some(index) = used.iter().position(|used| !used) {
            return Err(DiagnosticPresentationError::UnusedParameter(
                parameters[index].name.clone(),
            ));
        }
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
