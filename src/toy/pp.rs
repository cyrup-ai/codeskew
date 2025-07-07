use super::{
    bind::NUM_ASSERT_COUNTERS,
    utils::{fetch_include, parse_u32},
};
use async_recursion::async_recursion;

use lazy_regex::*;
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn wgsl_error_handler(summary: &str, row: usize, col: usize);
}

#[derive(Clone)]
pub struct WGSLError {
    summary: Cow<'static, str>,
    line: usize,
}

impl WGSLError {
    #[inline]
    pub fn new(summary: impl Into<Cow<'static, str>>, line: usize) -> Self {
        Self {
            summary: summary.into(),
            line,
        }
    }

    #[inline]
    pub fn handler(summary: &str, row: usize, col: usize) {
        #[cfg(target_arch = "wasm32")]
        wgsl_error_handler(summary, row, col);
        #[cfg(not(target_arch = "wasm32"))]
        println!("{row}:{col}: {summary}");
    }

    #[inline]
    pub fn submit(&self) {
        Self::handler(&self.summary, self.line, 0)
    }
}

#[derive(Clone)]
#[wasm_bindgen]
pub struct SourceMap {
    #[wasm_bindgen(skip)]
    pub extensions: String,
    #[wasm_bindgen(skip)]
    pub source: String,
    #[wasm_bindgen(skip)]
    pub map: Vec<usize>,
    #[wasm_bindgen(skip)]
    pub workgroup_count: FxHashMap<String, [u32; 3]>,
    #[wasm_bindgen(skip)]
    pub dispatch_once: FxHashMap<String, bool>,
    #[wasm_bindgen(skip)]
    pub dispatch_count: FxHashMap<String, u32>,
    #[wasm_bindgen(skip)]
    pub assert_map: Vec<usize>,
    #[wasm_bindgen(skip)]
    pub user_data: indexmap::IndexMap<String, Vec<u32>>,
}

impl SourceMap {
    #[inline]
    pub fn new() -> Self {
        Self {
            extensions: String::with_capacity(1024),
            source: String::with_capacity(65536),
            map: Vec::with_capacity(1024),
            workgroup_count: FxHashMap::with_capacity_and_hasher(16, Default::default()),
            dispatch_once: FxHashMap::with_capacity_and_hasher(16, Default::default()),
            dispatch_count: FxHashMap::with_capacity_and_hasher(16, Default::default()),
            assert_map: Vec::with_capacity(NUM_ASSERT_COUNTERS),
            user_data: indexmap::IndexMap::with_capacity(16),
        }
    }

    #[inline]
    fn push_line(&mut self, s: &str, n: usize) {
        self.source.push_str(s);
        self.source.push('\n');
        self.map.push(n);
    }

    #[inline]
    fn push_extension(&mut self, s: &str) {
        self.extensions.push_str(s);
        self.extensions.push('\n');
    }
}

impl Default for SourceMap {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

pub struct Preprocessor {
    defines: FxHashMap<String, String>,
    source: SourceMap,
    storage_count: usize,
    assert_count: usize,
    special_strings: bool,
    temp_string: String,
    temp_chars: Vec<u32>,
}

static RE_COMMENT: Lazy<Regex> = lazy_regex!(r"(//.*|(?s:/\*.*?\*/))");
static RE_QUOTES: Lazy<Regex> = lazy_regex!(r#""((?:[^\\"]|\\.)*)""#);
static RE_CHEVRONS: Lazy<Regex> = lazy_regex!("<(.*)>");
static RE_WORD: Lazy<Regex> = lazy_regex!("[[:word:]]+");

const STRING_MAX_LEN: usize = 20;

#[inline]
pub fn strip_comments(s: &str) -> Cow<str> {
    RE_COMMENT.replace_all(s, "")
}

impl Preprocessor {
    pub fn new(mut defines: FxHashMap<String, String>) -> Self {
        defines.insert("STRING_MAX_LEN".to_string(), STRING_MAX_LEN.to_string());
        Self {
            defines,
            source: SourceMap::new(),
            storage_count: 0,
            assert_count: 0,
            special_strings: false,
            temp_string: String::with_capacity(1024),
            temp_chars: Vec::with_capacity(STRING_MAX_LEN),
        }
    }

    #[inline]
    fn subst_defines(&mut self, source: &str) -> Cow<str> {
        let result = RE_WORD.replace_all(source, |caps: &regex::Captures| {
            let name = &caps[0];
            self.defines
                .get(name)
                .map_or(name.to_string(), |v| v.to_string())
        });
        Cow::Owned(result.into_owned())
    }

    async fn preprocess(&mut self, shader: &str) -> Result<(), WGSLError> {
        let lines: Vec<&str> = shader.lines().collect();
        let _total_lines = lines.len();

        for (idx, line) in lines.into_iter().enumerate() {
            let line_num = idx + 1;
            self.process_line(line, line_num).await?;
        }
        Ok(())
    }

    #[async_recursion(?Send)]
    async fn process_line(&mut self, line_orig: &str, n: usize) -> Result<(), WGSLError> {
        let line_substituted = self.subst_defines(line_orig).into_owned();
        let line_ref = line_substituted.as_str();

        let trimmed = line_ref.trim_start();

        if trimmed.starts_with("enable") {
            let comment_stripped = strip_comments(line_ref);
            self.source.push_extension(&comment_stripped);
            return Ok(());
        }

        if !trimmed.starts_with('#') {
            if self.special_strings {
                self.temp_string.clear();
                let mut error_occurred = false;
                let mut error_msg = None;

                let processed = RE_QUOTES.replace_all(line_ref, |caps: &regex::Captures| {
                    if error_occurred {
                        return caps[0].to_string();
                    }

                    if let Ok(s) = snailquote::unescape(&caps[0]) {
                        self.temp_chars.clear();
                        self.temp_chars.extend(s.chars().map(|c| c as u32));
                        let len = self.temp_chars.len();

                        if len > STRING_MAX_LEN {
                            error_occurred = true;
                            error_msg = Some(WGSLError::new(
                                Cow::Borrowed(
                                    "String literals cannot be longer than 20 characters",
                                ),
                                n,
                            ));
                            return caps[0].to_string();
                        }

                        self.temp_chars.resize(STRING_MAX_LEN, 0);

                        self.temp_string.clear();
                        self.temp_string.push_str("String(");
                        self.temp_string.push_str(&len.to_string());
                        self.temp_string.push_str(", array<uint,20>(");

                        for (i, &c) in self.temp_chars.iter().enumerate() {
                            if i > 0 {
                                self.temp_string.push_str(", ");
                            }
                            self.temp_string.push_str(&format!("{c:#04x}"));
                        }

                        self.temp_string.push_str("))");
                        self.temp_string.clone()
                    } else {
                        caps[0].to_string()
                    }
                });

                if let Some(e) = error_msg {
                    return Err(e);
                }

                self.source.push_line(&processed, n);
            } else {
                self.source.push_line(line_ref, n);
            }
            return Ok(());
        }

        let comment_stripped = strip_comments(line_ref);
        let tokens: Vec<&str> = comment_stripped.split_whitespace().collect();

        if tokens.is_empty() {
            return Ok(());
        }

        match tokens[0] {
            "#include" => {
                if tokens.len() != 2 {
                    return Err(WGSLError::new(
                        Cow::Borrowed("Include directive requires exactly one argument"),
                        n,
                    ));
                }

                let name = tokens[1];
                let include_result = if let Some(quotes_cap) = RE_QUOTES.captures(name) {
                    fetch_include(quotes_cap[1].to_string()).await
                } else if let Some(chevrons_cap) = RE_CHEVRONS.captures(name) {
                    let path = &chevrons_cap[1];
                    if path == "string" {
                        self.special_strings = true;
                    }
                    fetch_include(format!("std/{path}")).await
                } else {
                    return Err(WGSLError::new(
                        Cow::Borrowed("Path must be enclosed in quotes or chevrons"),
                        n,
                    ));
                };

                if let Some(code) = include_result {
                    for line in code.lines() {
                        self.process_line(line, n).await?;
                    }
                } else {
                    return Err(WGSLError::new(
                        Cow::Owned(format!("Cannot find include {name}")),
                        n,
                    ));
                }
            }
            "#workgroup_count" => {
                if tokens.len() != 5 {
                    return Err(WGSLError::new(
                        Cow::Borrowed("Workgroup count directive requires name and three values"),
                        n,
                    ));
                }

                let name = tokens[1].to_string();
                let x = parse_u32(tokens[2], n)?;
                let y = parse_u32(tokens[3], n)?;
                let z = parse_u32(tokens[4], n)?;

                self.source.workgroup_count.insert(name, [x, y, z]);
            }
            "#dispatch_once" => {
                if tokens.len() != 2 {
                    return Err(WGSLError::new(
                        Cow::Borrowed("Dispatch once directive requires exactly one name"),
                        n,
                    ));
                }

                self.source
                    .dispatch_once
                    .insert(tokens[1].to_string(), true);
            }
            "#dispatch_count" => {
                if tokens.len() != 3 {
                    return Err(WGSLError::new(
                        Cow::Borrowed("Dispatch count directive requires name and count"),
                        n,
                    ));
                }

                let name = tokens[1].to_string();
                let count = parse_u32(tokens[2], n)?;

                self.source.dispatch_count.insert(name, count);
            }
            "#define" => {
                if tokens.len() < 2 {
                    return Err(WGSLError::new(
                        Cow::Borrowed("Define directive requires at least a name"),
                        n,
                    ));
                }

                let name = tokens[1];
                let value = if tokens.len() > 2 {
                    tokens[2..].join(" ")
                } else {
                    String::new()
                };

                if self.defines.contains_key(name) {
                    return Err(WGSLError::new(
                        Cow::Owned(format!("Cannot redefine {name}")),
                        n,
                    ));
                }

                self.defines.insert(name.to_string(), value);
            }
            "#storage" => {
                if tokens.len() < 3 {
                    return Err(WGSLError::new(
                        Cow::Borrowed("Storage directive requires name and type"),
                        n,
                    ));
                }

                if self.storage_count >= 2 {
                    return Err(WGSLError::new(
                        Cow::Borrowed("Only two storage buffers are currently supported"),
                        n,
                    ));
                }

                let name = tokens[1];
                let type_str = tokens[2..].join(" ");

                self.temp_string.clear();
                self.temp_string.push_str("@group(0) @binding(");
                self.temp_string.push_str(&self.storage_count.to_string());
                self.temp_string.push_str(") var<storage,read_write> ");
                self.temp_string.push_str(name);
                self.temp_string.push_str(": ");
                self.temp_string.push_str(&type_str);
                self.temp_string.push(';');

                self.source.push_line(&self.temp_string, n);
                self.storage_count += 1;
            }
            "#assert" => {
                if tokens.len() < 2 {
                    return Err(WGSLError::new(
                        Cow::Borrowed("Assert directive requires predicate"),
                        n,
                    ));
                }

                if self.assert_count >= NUM_ASSERT_COUNTERS {
                    return Err(WGSLError::new(
                        Cow::Owned(format!(
                            "A maximum of {NUM_ASSERT_COUNTERS} assertions are currently supported"
                        )),
                        n,
                    ));
                }

                let predicate = tokens[1..].join(" ");

                self.temp_string.clear();
                self.temp_string.push_str("assert(");
                self.temp_string.push_str(&self.assert_count.to_string());
                self.temp_string.push_str(", ");
                self.temp_string.push_str(&predicate);
                self.temp_string.push_str(");");

                self.source.push_line(&self.temp_string, n);
                self.source.assert_map.push(n);
                self.assert_count += 1;
            }
            "#data" => {
                if tokens.len() < 4 || tokens[2] != "u32" {
                    return Err(WGSLError::new(
                        Cow::Borrowed("Data directive requires name, u32 type, and data"),
                        n,
                    ));
                }

                let name = tokens[1].to_string();
                let data_str = tokens[3..].join("");

                let parsed_data: Result<Vec<u32>, _> = data_str
                    .split(',')
                    .map(|s| parse_u32(s.trim(), n))
                    .collect();

                match parsed_data {
                    Ok(mut data) => {
                        if self.source.user_data.len() == 1
                            && self.source.user_data.contains_key("_dummy")
                        {
                            self.source.user_data.clear();
                        }

                        if let Some(existing) = self.source.user_data.get_mut(&name) {
                            existing.append(&mut data);
                        } else {
                            self.source.user_data.insert(name, data);
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
            _ => {
                return Err(WGSLError::new(
                    Cow::Borrowed("Unrecognised preprocessor directive"),
                    n,
                ));
            }
        }

        Ok(())
    }

    pub async fn run(&mut self, shader: &str) -> Option<SourceMap> {
        match self.preprocess(shader).await {
            Ok(()) => Some(std::mem::take(&mut self.source)),
            Err(e) => {
                e.submit();
                None
            }
        }
    }
}
