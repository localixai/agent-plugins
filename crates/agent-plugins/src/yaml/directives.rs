//! `yaml`'s directives (`dist/doc/directives.js`): the `%YAML` and `%TAG` lines before a
//! document, how a tag's text resolves to its name, and how a name is written back.

use indexmap::IndexMap;

/// The default tag handles: `!!` for the YAML schema's tags.
pub(crate) const DEFAULT_TAG_PREFIX: &str = "tag:yaml.org,2002:";

/// A document's directives.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's directive flags, one each"
)]
pub(crate) struct Directives {
    /// The document starts with an explicit `---`.
    pub(crate) doc_start: bool,
    /// The document ends with an explicit `...`.
    pub(crate) doc_end: bool,
    /// A `%YAML` line was given.
    pub(crate) explicit: bool,
    pub(crate) version: String,
    /// Tag handles and their prefixes.
    pub(crate) tags: IndexMap<String, String>,
    at_next_document: bool,
}

fn default_tags() -> IndexMap<String, String> {
    IndexMap::from([("!!".to_owned(), DEFAULT_TAG_PREFIX.to_owned())])
}

/// `escapeTagName`.
fn escape_tag_name(name: &str) -> String {
    let mut out = String::new();
    for character in name.chars() {
        match character {
            '!' => out.push_str("%21"),
            ',' => out.push_str("%2C"),
            '[' => out.push_str("%5B"),
            ']' => out.push_str("%5D"),
            '{' => out.push_str("%7B"),
            '}' => out.push_str("%7D"),
            other => out.push(other),
        }
    }
    out
}

impl Directives {
    /// The directives of a stream that starts with none (version 1.2).
    pub(crate) fn new() -> Self {
        Self {
            doc_start: false,
            doc_end: false,
            explicit: false,
            version: "1.2".to_owned(),
            tags: default_tags(),
            at_next_document: false,
        }
    }

    /// `atDocument`: the directives a document starts with; the stream's own reset for the next
    /// one (YAML 1.2: directives hold for one document).
    pub(crate) fn at_document(&mut self) -> Self {
        let document = Self {
            doc_start: false,
            doc_end: false,
            explicit: self.explicit,
            version: self.version.clone(),
            tags: self.tags.clone(),
            at_next_document: false,
        };
        match self.version.as_str() {
            "1.1" => self.at_next_document = true,
            "1.2" => {
                self.at_next_document = false;
                self.explicit = false;
                "1.2".clone_into(&mut self.version);
                self.tags = default_tags();
            }
            _ => {}
        }
        document
    }

    /// `add`: one directive line; `on_error(offset, message, warning)` for a bad one.
    pub(crate) fn add(
        &mut self,
        line: &str,
        on_error: &mut dyn FnMut(usize, String, bool),
    ) -> bool {
        if self.at_next_document {
            self.explicit = false;
            "1.1".clone_into(&mut self.version);
            self.tags = default_tags();
            self.at_next_document = false;
        }
        let mut parts = crate::js::trim(line)
            .split([' ', '\t'])
            .filter(|part| !part.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>()
            .into_iter();
        let name = parts.next().unwrap_or_default();
        let parts: Vec<String> = parts.collect();
        match name.as_str() {
            "%TAG" => {
                if parts.len() != 2 {
                    on_error(
                        0,
                        "%TAG directive should contain exactly two parts".to_owned(),
                        false,
                    );
                    if parts.len() < 2 {
                        return false;
                    }
                }
                let mut parts = parts.into_iter();
                let (Some(handle), Some(prefix)) = (parts.next(), parts.next()) else {
                    return false;
                };
                self.tags.insert(handle, prefix);
                true
            }
            "%YAML" => {
                self.explicit = true;
                let [version] = parts.as_slice() else {
                    on_error(
                        0,
                        "%YAML directive should contain exactly one part".to_owned(),
                        false,
                    );
                    return false;
                };
                if version == "1.1" || version == "1.2" {
                    self.version.clone_from(version);
                    return true;
                }
                let valid = version.split_once('.').is_some_and(|(major, minor)| {
                    !major.is_empty()
                        && !minor.is_empty()
                        && major.bytes().all(|byte| byte.is_ascii_digit())
                        && minor.bytes().all(|byte| byte.is_ascii_digit())
                });
                on_error(6, format!("Unsupported YAML version {version}"), valid);
                false
            }
            _ => {
                on_error(0, format!("Unknown directive {name}"), true);
                false
            }
        }
    }

    /// `tagName`: the name a tag's text stands for, `None` when it does not resolve.
    pub(crate) fn tag_name(
        &self,
        source: &str,
        on_error: &mut dyn FnMut(String),
    ) -> Option<String> {
        if source == "!" {
            // The non-specific tag.
            return Some("!".to_owned());
        }
        if !source.starts_with('!') {
            on_error(format!("Not a valid tag: {source}"));
            return None;
        }
        if let Some(rest) = source.strip_prefix("!<") {
            let verbatim = rest.strip_suffix_one();
            if verbatim == "!" || verbatim == "!!" {
                on_error(format!(
                    "Verbatim tags aren't resolved, so {source} is invalid."
                ));
                return None;
            }
            if !source.ends_with('>') {
                on_error("Verbatim tags must end with a >".to_owned());
            }
            return Some(verbatim.to_owned());
        }
        // `/^(.*!)([^!]*)$/s`: the handle up to the last `!`, the suffix after it.
        let split = source.rfind('!').map_or(0, |at| at + 1);
        let (handle, suffix) = source.split_at(split);
        if suffix.is_empty() {
            on_error(format!("The {source} tag has no suffix"));
        }
        if let Some(prefix) = self.tags.get(handle) {
            return match crate::js::decode_uri_component(suffix) {
                Ok(decoded) => Some(format!("{prefix}{decoded}")),
                Err(error) => {
                    on_error(error);
                    None
                }
            };
        }
        if handle == "!" {
            // A local tag.
            return Some(source.to_owned());
        }
        on_error(format!("Could not resolve tag: {source}"));
        None
    }

    /// `tagString`: a tag's name as it is written.
    pub(crate) fn tag_string(&self, tag: &str) -> String {
        for (handle, prefix) in &self.tags {
            if let Some(rest) = tag.strip_prefix(prefix.as_str()) {
                return format!("{handle}{}", escape_tag_name(rest));
            }
        }
        if tag.starts_with('!') {
            tag.to_owned()
        } else {
            format!("!<{tag}>")
        }
    }

    /// `toString`: the directive lines written before a document, given the tags its nodes use.
    pub(crate) fn to_string(&self, used_tags: &[String]) -> String {
        let mut lines = Vec::new();
        if self.explicit {
            lines.push(format!("%YAML {}", self.version));
        }
        for (handle, prefix) in &self.tags {
            if handle == "!!" && prefix == DEFAULT_TAG_PREFIX {
                continue;
            }
            if used_tags
                .iter()
                .any(|name| name.starts_with(prefix.as_str()))
            {
                lines.push(format!("%TAG {handle} {prefix}"));
            }
        }
        lines.join("\n")
    }
}

/// `source.slice(2, -1)` after the `!<`: the text without its last character.
trait StripOne {
    fn strip_suffix_one(&self) -> &str;
}

impl StripOne for str {
    fn strip_suffix_one(&self) -> &str {
        self.char_indices()
            .last()
            .map_or("", |(at, _)| self.get(..at).unwrap_or(""))
    }
}
