// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The two composite datatypes, and the vocabulary IRIs the grammar pins.
//!
//! Every IRI constant here is a **third-party, spec-defined** string: the two CDT
//! datatype IRIs come from SEP-0009 and the three RDF/XSD ones from the W3C
//! Recommendations. PurRDF mints no vocabulary, and hard-coding these is not
//! minting — they are the fixed spelling the grammar itself is written in, exactly
//! as `purrdf-xsd` hard-codes the XML Schema namespace. They are **not**
//! caller-supplied configuration and there is no default to fabricate.

/// The SEP-0009 composite-datatype namespace.
pub const CDT_NS: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/";

/// `cdt:List` — the SEP-0009 list datatype IRI.
pub const CDT_LIST: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";

/// `cdt:Map` — the SEP-0009 map datatype IRI.
pub const CDT_MAP: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map";

/// `xsd:string` — the datatype a `String` with no `LANGTAG` and no `^^` denotes.
pub const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// `xsd:integer` — the datatype the `INTEGER` numeric shorthand denotes.
pub const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// `xsd:decimal` — the datatype the `DECIMAL` numeric shorthand denotes.
pub const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";

/// `xsd:double` — the datatype the `DOUBLE` numeric shorthand denotes.
pub const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";

/// `xsd:boolean` — the datatype the `BooleanLiteral` shorthand denotes.
pub const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

/// `rdf:langString` — the datatype of a language-tagged string with no direction.
pub const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// `rdf:dirLangString` — the RDF 1.2 datatype of a *directional* language-tagged
/// string (`"lex"@lang--ltr` / `"lex"@lang--rtl`).
pub const RDF_DIR_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";

/// The SEP-0009 composite datatypes.
///
/// This is a **closed set by design**, exactly as [`purrdf_xsd::XsdDatatype`] is:
/// SEP-0009 defines two composite datatypes and does not grow at runtime, so
/// dispatch over this enum is closed-but-correct and there is no runtime datatype
/// registry. A datatype IRI outside this set is simply "not a composite datatype"
/// — see [`CdtDatatype::from_iri`] returning `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CdtDatatype {
    /// `cdt:List` — an ordered, possibly heterogeneous sequence of elements.
    List,
    /// `cdt:Map` — an unordered set of key/value entries with pairwise distinct keys.
    Map,
}

impl CdtDatatype {
    /// Resolve a datatype IRI to its [`CdtDatatype`], or `None` when the IRI is not
    /// one of the two composite datatypes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::CdtDatatype;
    ///
    /// assert_eq!(
    ///     CdtDatatype::from_iri("http://w3id.org/awslabs/neptune/SPARQL-CDTs/List"),
    ///     Some(CdtDatatype::List)
    /// );
    /// assert_eq!(CdtDatatype::from_iri(CdtDatatype::Map.iri()), Some(CdtDatatype::Map));
    /// assert_eq!(CdtDatatype::from_iri("http://example.org/List"), None);
    /// ```
    #[must_use]
    pub fn from_iri(iri: &str) -> Option<Self> {
        match iri {
            CDT_LIST => Some(Self::List),
            CDT_MAP => Some(Self::Map),
            _ => None,
        }
    }

    /// The datatype IRI for this composite datatype.
    #[must_use]
    pub const fn iri(self) -> &'static str {
        match self {
            Self::List => CDT_LIST,
            Self::Map => CDT_MAP,
        }
    }

    /// The opening delimiter of this datatype's lexical form (`[` or `{`).
    #[must_use]
    pub const fn open(self) -> u8 {
        match self {
            Self::List => b'[',
            Self::Map => b'{',
        }
    }

    /// The closing delimiter of this datatype's lexical form (`]` or `}`).
    #[must_use]
    pub const fn close(self) -> u8 {
        match self {
            Self::List => b']',
            Self::Map => b'}',
        }
    }
}
