//! Historical authored material projection, not design acceptance or authority.
use crate::{
    CodecError,
    documents::{MarkdownDocument, unfenced_lines},
    unicode,
};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use sha2::{Digest, Sha256};
use std::{
    borrow::Cow,
    io::{self, Write},
};
use writfold_domain::{DesignRevision, ProposalMaterialRevision};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaterialIdentity {
    Legacy(ProposalMaterialRevision),
    Design(DesignRevision),
}

impl MaterialIdentity {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Legacy(value) => value.as_str(),
            Self::Design(value) => value.as_str(),
        }
    }
}

/// Workflow 2 (including absent version) retains unprefixed historical hashes.
/// Only explicit workflow 3 selects the disjoint design-v3 domain and binding
/// exclusions. Unknown versions are refused, as in authored record admission.
pub fn material_revision(document: &MarkdownDocument) -> Result<MaterialIdentity, CodecError> {
    let design = match document.field("workflow_version") {
        None | Some("2") => false,
        Some("3") => true,
        Some(_) => {
            return Err(CodecError::malformed(
                "unsupported authored workflow version",
            ));
        }
    };
    let mut fields: Vec<_> = document
        .frontmatter()
        .filter(|(key, _)| !bookkeeping(key, design))
        .collect();
    fields.sort_unstable_by(|left, right| left.0.cmp(right.0));
    let body = document.body();
    let end = unfenced_lines(body)
        .find_map(|(offset, line)| {
            line.strip_prefix("##")
                .and_then(|rest| rest.chars().next())
                .is_some_and(unicode::space)
                .then_some(offset)
        })
        .unwrap_or(body.len());
    let projection = Projection {
        fields: &fields,
        preamble: &body[..end],
        document,
    };
    let mut hash = Sha256::new();
    let mut serializer = serde_json::Serializer::with_formatter(DigestWriter(&mut hash), Spaced);
    projection
        .serialize(&mut serializer)
        .map_err(|error| CodecError::malformed(error.to_string()))?;
    let mut identity = String::with_capacity(if design { 74 } else { 64 });
    if design {
        identity.push_str("design-v3:");
    }
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in hash.finalize() {
        identity.push(char::from(HEX[usize::from(byte >> 4)]));
        identity.push(char::from(HEX[usize::from(byte & 15)]));
    }
    if design {
        DesignRevision::try_from(identity).map(MaterialIdentity::Design)
    } else {
        ProposalMaterialRevision::try_from(identity).map(MaterialIdentity::Legacy)
    }
    .map_err(|error| CodecError::malformed(error.to_string()))
}

fn bookkeeping(key: &str, design: bool) -> bool {
    matches!(
        key,
        "status"
            | "updated"
            | "decided_by"
            | "acceptor"
            | "executor"
            | "verifier"
            | "accepted_revision"
            | "superseded_by"
            | "swept_at"
            | "landed"
            | "last_actor"
            | "regression_note"
    ) || (design
        && matches!(
            key,
            "target_revision"
                | "binding_design_revision"
                | "binding_source"
                | "binding_target_revision"
                | "binding_actor"
                | "binding_observed_at"
        ))
}

fn lf(text: &str) -> Cow<'_, str> {
    if !text.contains('\r') {
        return Cow::Borrowed(text);
    }
    let mut output = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\r' {
            if characters.peek() == Some(&'\n') {
                characters.next();
            }
            output.push('\n');
        } else {
            output.push(character);
        }
    }
    Cow::Owned(output)
}

struct Projection<'a> {
    fields: &'a [(&'a str, &'a str)],
    preamble: &'a str,
    document: &'a MarkdownDocument,
}

impl Serialize for Projection<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("frontmatter", &Fields(self.fields))?;
        map.serialize_entry("preamble", &lf(self.preamble))?;
        map.serialize_entry("sections", &Sections(self.document))?;
        map.end()
    }
}

struct Fields<'a>(&'a [(&'a str, &'a str)]);
impl Serialize for Fields<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

struct Sections<'a>(&'a MarkdownDocument);
impl Serialize for Sections<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(None)?;
        for (name, content) in self
            .0
            .sections()
            .filter(|(name, _)| *name != "Decision log")
        {
            sequence.serialize_element(&(name, lf(content)))?;
        }
        sequence.end()
    }
}

struct DigestWriter<'a>(&'a mut Sha256);
impl Write for DigestWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// Material identities use Python json.dumps defaults, not compact signing JSON.
struct Spaced;
impl serde_json::ser::Formatter for Spaced {
    fn begin_array_value<W: ?Sized + Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        if first {
            Ok(())
        } else {
            writer.write_all(b", ")
        }
    }
    fn begin_object_key<W: ?Sized + Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        if first {
            Ok(())
        } else {
            writer.write_all(b", ")
        }
    }
    fn begin_object_value<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        writer.write_all(b": ")
    }
}
