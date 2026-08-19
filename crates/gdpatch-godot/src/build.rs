use crate::gdscript::TokenType;
use color_eyre::Report;
use color_eyre::eyre::{Context as _, ContextCompat, OptionExt, bail, eyre};
use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, btree_map};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::sync::LazyLock;
use thiserror::Error;

/// Tokenizer information specific to the GDScript V1 tokenizer.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GDScriptV1Build {
    // TODO: add built in function names for V1 binary tokenization
}

/// GDScript tokenizer and parser information.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GDScriptV2Build {
    // Binary tokenizer info
    /// Version number in the bytecode header. Unset if this version doesn't have a bytecode
    /// format.
    pub tokenizer_version: Option<u32>,

    /// Order of tokens in the `Token` enum.
    pub tokens: Vec<TokenType>,

    // Text tokenizer flags
    /// Whether the new mixed indentation behavior applies. Official builds changed this in
    /// [`4d38529284120562abec62425b21c9b90b56faa7`] (4.0.3).
    ///
    /// [`4d38529284120562abec62425b21c9b90b56faa7`]: https://github.com/godotengine/godot/commit/4d38529284120562abec62425b21c9b90b56faa7
    pub allow_mixed_indentation_when_multiline: bool,

    /// Whether to use the old or new invalid character error text. Official builds changed this in
    /// [`54770ba9c545bd1fd2f3c2b1be52228ab5728a85`] (4.1)
    ///
    /// [`54770ba9c545bd1fd2f3c2b1be52228ab5728a85`]: https://github.com/godotengine/godot/commit/54770ba9c545bd1fd2f3c2b1be52228ab5728a85
    pub has_improved_invalid_character_error: bool,

    /// Whether numbers with a leading `+` or `-` are parsed as literals or an operator followed by
    /// a literal. Official builds changed this in [`d15511725acdfe90f9d5967119294b591becd8fa`]
    /// (4.1).
    ///
    /// [`d15511725acdfe90f9d5967119294b591becd8fa`]: https://github.com/godotengine/godot/commit/d15511725acdfe90f9d5967119294b591becd8fa
    pub has_literal_sign_handling: bool,

    /// Whether number literal parsing uses new behavior around `_` tokens. Official builds changed
    /// this in [`fba8cbe6dbf17399e06ac9141a862734187dfb65`] (4.1).
    ///
    /// [`fba8cbe6dbf17399e06ac9141a862734187dfb65`]: https://github.com/godotengine/godot/commit/fba8cbe6dbf17399e06ac9141a862734187dfb65
    pub has_new_number_underscore_parsing: bool,

    /// Whether raw string literals (`r"text"`) exist. Official builds added this in
    /// [`2964c7d51cbdaa616841c23d03f4a2f9966554b5`] (4.2).
    ///
    /// [`2964c7d51cbdaa616841c23d03f4a2f9966554b5`]: https://github.com/godotengine/godot/commit/2964c7d51cbdaa616841c23d03f4a2f9966554b5
    pub has_raw_strings: bool,

    /// Whether `when` exists in this version. Official builds added this in
    /// [`54a1414500ee2f8f87647fc0ffe921498332446f`] (4.2).
    ///
    /// [`54a1414500ee2f8f87647fc0ffe921498332446f`]: https://github.com/godotengine/godot/commit/54a1414500ee2f8f87647fc0ffe921498332446f
    pub has_when: bool,

    /// Whether `0x` and `0b` are valid on their own. Official builds changed this in
    /// [`4e5b545c0465c8c007440e21b72c6d0ac35feb4e`] (4.2.2).
    ///
    /// [`4e5b545c0465c8c007440e21b72c6d0ac35feb4e`]: https://github.com/godotengine/godot/commit/4e5b545c0465c8c007440e21b72c6d0ac35feb4e
    pub need_digits_in_hex_and_binary: bool,

    /// Whether continuation lines handle whitespace properly. Official builds changed this in
    /// [`02253b6b91472e251418bd0545afb2b653b5385c`] (4.3).
    ///
    /// [`02253b6b91472e251418bd0545afb2b653b5385c`]: https://github.com/godotengine/godot/commit/02253b6b91472e251418bd0545afb2b653b5385c
    pub has_fixed_continuation_lines: bool,

    /// Whether literals with uppercase `0x` and `0b` are allowed. Official builds changed this in
    /// [`3be46a69c431519fbe4b6a5d39374585fd994802`] (4.4).
    ///
    /// [`3be46a69c431519fbe4b6a5d39374585fd994802`]: https://github.com/godotengine/godot/commit/3be46a69c431519fbe4b6a5d39374585fd994802
    pub has_uppercase_number_types: bool,

    /// Whether this version has variadic functions and the `...` token. Official builds added this
    /// in [`ee121ef80e36865ac9d5c55ab2ec419f48ef6954`] (4.5).
    ///
    /// [`ee121ef80e36865ac9d5c55ab2ec419f48ef6954`]: https://github.com/godotengine/godot/commit/ee121ef80e36865ac9d5c55ab2ec419f48ef6954
    pub has_variadic_functions: bool,

    /// Whether this version expands tabs to 4 spaces in the span column. Official builds changed this in
    /// [`612475a680178afc6910687c616a054664ccc8f2`] (4.7).
    pub expands_tabs_in_span_column: bool,

    /// Whether a zero-width space (ZWSP) is counted as whitespace. Official builds changed this in
    /// [`a6ff5187637ada695a432b6b43912305734aaff0`] (4.4).
    ///
    /// [`a6ff5187637ada695a432b6b43912305734aaff0`]: https://github.com/godotengine/godot/commit/a6ff5187637ada695a432b6b43912305734aaff0
    pub allow_zwsp_as_whitespace: bool,

    /// Whether mixed indentation is allowed on blank lines. Official builds changed this in
    /// [`00ad9e484e2e8491007bc7d2adfaf0598c970afc`] (4.2).
    ///
    /// [`00ad9e484e2e8491007bc7d2adfaf0598c970afc`]: https://github.com/godotengine/godot/commit/00ad9e484e2e8491007bc7d2adfaf0598c970afc
    pub allow_mixed_indentation_on_blank_lines: bool,

    // Binary tokenizer flags
    /// Whether this version contains an extra unused word in the header for binary format scripts. Official builds removed this
    /// in [`6909309ca018435e8bf0d908282599c5e642bd78`] (4.5).
    ///
    /// [`6909309ca018435e8bf0d908282599c5e642bd78`]: https://github.com/godotengine/godot/commit/6909309ca018435e8bf0d908282599c5e642bd78
    pub has_extra_word_in_binary_script_header: bool,

    // Parser flags
    /// Whether this version only updates multiline state when a '(' token is seen after a super call. Official builds fixed this
    /// in [`3694d22db30d2aa6a93499922d24b2592f3adaae`] (4.7).
    ///
    /// [`3694d22db30d2aa6a93499922d24b2592f3adaae`]: https://github.com/godotengine/godot/commit/3694d22db30d2aa6a93499922d24b2592f3adaae
    pub has_fixed_multiline_handling_in_super_calls: bool,

    /// Whether this version bails early when parsing invalid super calls. Official builds changed this
    /// in [`b67dcb21fda16956859dbb217cbb1e0238af3ef2`] (4.5).
    ///
    /// [`b67dcb21fda16956859dbb217cbb1e0238af3ef2`]: https://github.com/godotengine/godot/commit/b67dcb21fda16956859dbb217cbb1e0238af3ef2
    pub has_early_bail_in_super_calls: bool,

    /// Whether to allow array/dictionary match patterns that span multiple lines without escapes. Official builds changed this in
    /// [`74177d79c9e80616edce2336cd487f9e01c2db08`] (4.3).
    ///
    /// [`74177d79c9e80616edce2336cd487f9e01c2db08`]: https://github.com/godotengine/godot/commit/74177d79c9e80616edce2336cd487f9e01c2db08
    pub allow_multiline_array_dictionary_patterns: bool,

    /// Whether this version allows a trailing comma after the preload method. Official builds added this
    /// in [`a3e0f8dee20cd1c23cff8b7903e71ba2322c4f70`] (4.6).
    ///
    /// [`a3e0f8dee20cd1c23cff8b7903e71ba2322c4f70`]: https://github.com/godotengine/godot/commit/a3e0f8dee20cd1c23cff8b7903e71ba2322c4f70
    pub allow_preload_trailing_comma: bool,

    /// Whether this version has support for static variables and functions. Official builds added this
    /// in [`0ba6048ad3c945e2bd1d0114b5095535c22103ce`] (4.1), but the token existed since the initial rewrite of GDScript for V2.
    ///
    /// [`0ba6048ad3c945e2bd1d0114b5095535c22103ce`]: https://github.com/godotengine/godot/commit/0ba6048ad3c945e2bd1d0114b5095535c22103ce
    pub has_static_variables: bool,

    /// Whether `x is not Type` is a supported syntax construct. Official builds added this in
    /// [`2bf25954b4aaa746b4dd8cf1d5f823ccf646224a`] (4.3).
    ///
    /// [`2bf25954b4aaa746b4dd8cf1d5f823ccf646224a`]: https://github.com/godotengine/godot/commit/2bf25954b4aaa746b4dd8cf1d5f823ccf646224a
    pub has_is_not: bool,

    /// Whether to allow `var x get():` instead of just `var x get:`. Official builds changed this in
    /// [`668ba2d1a5723d1e0d71d10432a6a16b48e63b05`] (4.3).
    ///
    /// [`668ba2d1a5723d1e0d71d10432a6a16b48e63b05`]: https://github.com/godotengine/godot/commit/668ba2d1a5723d1e0d71d10432a6a16b48e63b05
    pub allow_empty_parentheses_in_getter_declaration: bool,

    /// Whether match statements utilize the recovery suite. Official builds added this in
    /// [`4a0e40f6ea0f30e8eaa07414ec9e2642fdac7622`] (4.5).
    ///
    /// [`4a0e40f6ea0f30e8eaa07414ec9e2642fdac7622`]: 4a0e40f6ea0f30e8eaa07414ec9e2642fdac7622
    pub has_match_error_recovery: bool,

    /// Whether dictionaries have improved error recovery. Official builds added this in
    /// [`ca1e444bca01545ffed97e8786de5b30a9ace01e`] (4.5).
    ///
    /// [`ca1e444bca01545ffed97e8786de5b30a9ace01e`]: ca1e444bca01545ffed97e8786de5b30a9ace01e
    pub has_dictionary_error_recovery: bool,

    /// Whether the loop variable in a `for ... in` loop can have type hints. Official builds added this in
    /// [`6c59ed9485bbfadee73a08dfc57224e022626e6e`] (4.2).
    ///
    /// [`6c59ed9485bbfadee73a08dfc57224e022626e6e`]: https://github.com/godotengine/godot/commit/6c59ed9485bbfadee73a08dfc57224e022626e6e
    pub has_typed_for_loops: bool,

    /// Whether expressions like `x.for` are allowed. Official builds added this in
    /// [`ab9f60dd1aa6e1d5b6b24878e9dc6a290d95be8f`] (4.1).
    ///
    /// [`ab9f60dd1aa6e1d5b6b24878e9dc6a290d95be8f`]: https://github.com/godotengine/godot/commit/ab9f60dd1aa6e1d5b6b24878e9dc6a290d95be8f
    pub allow_keywords_as_attributes: bool,

    /// Whether the parsing changes from PR #72979 exist. Official builds changed this in
    /// [`5038a336bed6ccb5901c1437494e34312cfdc4ad`] (4.1).
    ///
    /// [`5038a336bed6ccb5901c1437494e34312cfdc4ad`]: https://github.com/godotengine/godot/commit/5038a336bed6ccb5901c1437494e34312cfdc4ad
    pub has_72979_annotation_parsing: bool,

    /// Whether the suite changes from #77744 exist. Official builds changed this in
    /// [`f3bf75fbb4edf5d73cdedaf196fdcd358e031c82`] (4.1).
    ///
    /// [`f3bf75fbb4edf5d73cdedaf196fdcd358e031c82`]: https://github.com/godotengine/godot/commit/f3bf75fbb4edf5d73cdedaf196fdcd358e031c82
    pub has_77744_suite_changes: bool,
}

pub fn deserialize_tokenizer_version<'de, D>(
    deserializer: D,
) -> Result<Option<Option<u32>>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = i32::deserialize(deserializer)?;

    if value == -1 {
        Ok(Some(None))
    } else if value > 0 {
        Ok(Some(Some(value as u32)))
    } else {
        Err(D::Error::invalid_value(
            Unexpected::Signed(value as i64),
            &"-1 or a positive integer",
        ))
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum GDScriptBuild {
    /// GDScript 1.x (3.x engines).
    V1(GDScriptV1Build),

    /// GDScript 2.x (4.x engines).
    V2(GDScriptV2Build),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
#[serde(deny_unknown_fields)]
pub struct SerializedGDScriptBuild {
    #[serde(default)]
    pub parent: Option<String>,

    #[serde(default)]
    pub version: Option<u32>,

    // Binary tokenizer info
    /// Version number in the bytecode header. Unset if this version doesn't have a bytecode
    /// format.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_tokenizer_version")]
    pub tokenizer_version: Option<Option<u32>>,

    /// Order of tokens in the `Token` enum.
    #[serde(default)]
    pub tokens: Option<Vec<TokenType>>,

    // Text tokenizer flags
    /// Whether the new mixed indentation behavior applies. Official builds changed this in
    /// [`4d38529284120562abec62425b21c9b90b56faa7`] (4.0.3).
    ///
    /// [`4d38529284120562abec62425b21c9b90b56faa7`]: https://github.com/godotengine/godot/commit/4d38529284120562abec62425b21c9b90b56faa7
    #[serde(default)]
    pub allow_mixed_indentation_when_multiline: Option<bool>,

    /// Whether to use the old or new invalid character error text. Official builds changed this in
    /// [`54770ba9c545bd1fd2f3c2b1be52228ab5728a85`] (4.1)
    ///
    /// [`54770ba9c545bd1fd2f3c2b1be52228ab5728a85`]: https://github.com/godotengine/godot/commit/54770ba9c545bd1fd2f3c2b1be52228ab5728a85
    #[serde(default)]
    pub has_improved_invalid_character_error: Option<bool>,

    /// Whether numbers with a leading `+` or `-` are parsed as literals or an operator followed by
    /// a literal. Official builds changed this in [`d15511725acdfe90f9d5967119294b591becd8fa`]
    /// (4.1).
    ///
    /// [`d15511725acdfe90f9d5967119294b591becd8fa`]: https://github.com/godotengine/godot/commit/d15511725acdfe90f9d5967119294b591becd8fa
    #[serde(default)]
    pub has_literal_sign_handling: Option<bool>,

    /// Whether number literal parsing uses new behavior around `_` tokens. Official builds changed
    /// this in [`fba8cbe6dbf17399e06ac9141a862734187dfb65`] (4.1).
    ///
    /// [`fba8cbe6dbf17399e06ac9141a862734187dfb65`]: https://github.com/godotengine/godot/commit/fba8cbe6dbf17399e06ac9141a862734187dfb65
    #[serde(default)]
    pub has_new_number_underscore_parsing: Option<bool>,

    /// Whether raw string literals (`r"text"`) exist. Official builds added this in
    /// [`2964c7d51cbdaa616841c23d03f4a2f9966554b5`] (4.2).
    ///
    /// [`2964c7d51cbdaa616841c23d03f4a2f9966554b5`]: https://github.com/godotengine/godot/commit/2964c7d51cbdaa616841c23d03f4a2f9966554b5
    #[serde(default)]
    pub has_raw_strings: Option<bool>,

    /// Whether `when` exists in this version. Official builds added this in
    /// [`54a1414500ee2f8f87647fc0ffe921498332446f`] (4.2).
    ///
    /// [`54a1414500ee2f8f87647fc0ffe921498332446f`]: https://github.com/godotengine/godot/commit/54a1414500ee2f8f87647fc0ffe921498332446f
    #[serde(default)]
    pub has_when: Option<bool>,

    /// Whether `0x` and `0b` are valid on their own. Official builds changed this in
    /// [`4e5b545c0465c8c007440e21b72c6d0ac35feb4e`] (4.2.2).
    ///
    /// [`4e5b545c0465c8c007440e21b72c6d0ac35feb4e`]: https://github.com/godotengine/godot/commit/4e5b545c0465c8c007440e21b72c6d0ac35feb4e
    #[serde(default)]
    pub need_digits_in_hex_and_binary: Option<bool>,

    /// Whether continuation lines handle whitespace properly. Official builds changed this in
    /// [`02253b6b91472e251418bd0545afb2b653b5385c`] (4.3).
    ///
    /// [`02253b6b91472e251418bd0545afb2b653b5385c`]: https://github.com/godotengine/godot/commit/02253b6b91472e251418bd0545afb2b653b5385c
    #[serde(default)]
    pub has_fixed_continuation_lines: Option<bool>,

    /// Whether literals with uppercase `0x` and `0b` are allowed. Official builds changed this in
    /// [`3be46a69c431519fbe4b6a5d39374585fd994802`] (4.4).
    ///
    /// [`3be46a69c431519fbe4b6a5d39374585fd994802`]: https://github.com/godotengine/godot/commit/3be46a69c431519fbe4b6a5d39374585fd994802
    #[serde(default)]
    pub has_uppercase_number_types: Option<bool>,

    /// Whether this version has variadic functions and the `...` token. Official builds added this
    /// in [`ee121ef80e36865ac9d5c55ab2ec419f48ef6954`] (4.5).
    ///
    /// [`ee121ef80e36865ac9d5c55ab2ec419f48ef6954`]: https://github.com/godotengine/godot/commit/ee121ef80e36865ac9d5c55ab2ec419f48ef6954
    #[serde(default)]
    pub has_variadic_functions: Option<bool>,

    /// Whether this version expands tabs to 4 spaces in the span column. Official builds changed this in
    /// [`612475a680178afc6910687c616a054664ccc8f2`] (4.7).
    #[serde(default)]
    pub expands_tabs_in_span_column: Option<bool>,

    /// Whether a zero-width space (ZWSP) is counted as whitespace. Official builds changed this in
    /// [`a6ff5187637ada695a432b6b43912305734aaff0`] (4.4).
    ///
    /// [`a6ff5187637ada695a432b6b43912305734aaff0`]: https://github.com/godotengine/godot/commit/a6ff5187637ada695a432b6b43912305734aaff0
    #[serde(default)]
    pub allow_zwsp_as_whitespace: Option<bool>,

    /// Whether mixed indentation is allowed on blank lines. Official builds changed this in
    /// [`00ad9e484e2e8491007bc7d2adfaf0598c970afc`] (4.2).
    ///
    /// [`00ad9e484e2e8491007bc7d2adfaf0598c970afc`]: https://github.com/godotengine/godot/commit/00ad9e484e2e8491007bc7d2adfaf0598c970afc
    #[serde(default)]
    pub allow_mixed_indentation_on_blank_lines: Option<bool>,

    // Binary tokenizer flags
    /// Whether this version contains an extra unused word in the header for binary format scripts. Official builds removed this
    /// in [`6909309ca018435e8bf0d908282599c5e642bd78`] (4.5).
    ///
    /// [`6909309ca018435e8bf0d908282599c5e642bd78`]: https://github.com/godotengine/godot/commit/6909309ca018435e8bf0d908282599c5e642bd78
    #[serde(default)]
    pub has_extra_word_in_binary_script_header: Option<bool>,

    // Parser flags
    /// Whether this version only updates multiline state when a '(' token is seen after a super call. Official builds fixed this
    /// in [`3694d22db30d2aa6a93499922d24b2592f3adaae`] (4.7).
    ///
    /// [`3694d22db30d2aa6a93499922d24b2592f3adaae`]: https://github.com/godotengine/godot/commit/3694d22db30d2aa6a93499922d24b2592f3adaae
    #[serde(default)]
    pub has_fixed_multiline_handling_in_super_calls: Option<bool>,

    /// Whether this version bails early when parsing invalid super calls. Official builds changed this
    /// in [`b67dcb21fda16956859dbb217cbb1e0238af3ef2`] (4.5).
    ///
    /// [`b67dcb21fda16956859dbb217cbb1e0238af3ef2`]: https://github.com/godotengine/godot/commit/b67dcb21fda16956859dbb217cbb1e0238af3ef2
    #[serde(default)]
    pub has_early_bail_in_super_calls: Option<bool>,

    /// Whether to allow array/dictionary match patterns that span multiple lines without escapes. Official builds changed this in
    /// [`74177d79c9e80616edce2336cd487f9e01c2db08`] (4.3).
    ///
    /// [`74177d79c9e80616edce2336cd487f9e01c2db08`]: https://github.com/godotengine/godot/commit/74177d79c9e80616edce2336cd487f9e01c2db08
    #[serde(default)]
    pub allow_multiline_array_dictionary_patterns: Option<bool>,

    /// Whether this version allows a trailing comma after the preload method. Official builds added this
    /// in [`a3e0f8dee20cd1c23cff8b7903e71ba2322c4f70`] (4.6).
    ///
    /// [`a3e0f8dee20cd1c23cff8b7903e71ba2322c4f70`]: https://github.com/godotengine/godot/commit/a3e0f8dee20cd1c23cff8b7903e71ba2322c4f70
    #[serde(default)]
    pub allow_preload_trailing_comma: Option<bool>,

    /// Whether this version has support for static variables and functions. Official builds added this
    /// in [`0ba6048ad3c945e2bd1d0114b5095535c22103ce`] (4.1), but the token existed since the initial rewrite of GDScript for V2.
    ///
    /// [`0ba6048ad3c945e2bd1d0114b5095535c22103ce`]: https://github.com/godotengine/godot/commit/0ba6048ad3c945e2bd1d0114b5095535c22103ce
    #[serde(default)]
    pub has_static_variables: Option<bool>,

    /// Whether `x is not Type` is a supported syntax construct. Official builds added this in
    /// [`2bf25954b4aaa746b4dd8cf1d5f823ccf646224a`] (4.3).
    ///
    /// [`2bf25954b4aaa746b4dd8cf1d5f823ccf646224a`]: https://github.com/godotengine/godot/commit/2bf25954b4aaa746b4dd8cf1d5f823ccf646224a
    #[serde(default)]
    pub has_is_not: Option<bool>,

    /// Whether to allow `var x get():` instead of just `var x get:`. Official builds changed this in
    /// [`668ba2d1a5723d1e0d71d10432a6a16b48e63b05`] (4.3).
    ///
    /// [`668ba2d1a5723d1e0d71d10432a6a16b48e63b05`]: https://github.com/godotengine/godot/commit/668ba2d1a5723d1e0d71d10432a6a16b48e63b05
    #[serde(default)]
    pub allow_empty_parentheses_in_getter_declaration: Option<bool>,

    /// Whether match statements utilize the recovery suite. Official builds added this in
    /// [`4a0e40f6ea0f30e8eaa07414ec9e2642fdac7622`] (4.5).
    ///
    /// [`4a0e40f6ea0f30e8eaa07414ec9e2642fdac7622`]: 4a0e40f6ea0f30e8eaa07414ec9e2642fdac7622
    #[serde(default)]
    pub has_match_error_recovery: Option<bool>,

    /// Whether dictionaries have improved error recovery. Official builds added this in
    /// [`ca1e444bca01545ffed97e8786de5b30a9ace01e`] (4.5).
    ///
    /// [`ca1e444bca01545ffed97e8786de5b30a9ace01e`]: ca1e444bca01545ffed97e8786de5b30a9ace01e
    #[serde(default)]
    pub has_dictionary_error_recovery: Option<bool>,

    /// Whether the loop variable in a `for ... in` loop can have type hints. Official builds added this in
    /// [`6c59ed9485bbfadee73a08dfc57224e022626e6e`] (4.2).
    ///
    /// [`6c59ed9485bbfadee73a08dfc57224e022626e6e`]: https://github.com/godotengine/godot/commit/6c59ed9485bbfadee73a08dfc57224e022626e6e
    #[serde(default)]
    pub has_typed_for_loops: Option<bool>,

    /// Whether expressions like `x.for` are allowed. Official builds added this in
    /// [`ab9f60dd1aa6e1d5b6b24878e9dc6a290d95be8f`] (4.1).
    ///
    /// [`ab9f60dd1aa6e1d5b6b24878e9dc6a290d95be8f`]: https://github.com/godotengine/godot/commit/ab9f60dd1aa6e1d5b6b24878e9dc6a290d95be8f
    #[serde(default)]
    pub allow_keywords_as_attributes: Option<bool>,

    /// Whether the parsing changes from PR #72979 exist. Official builds changed this in
    /// [`5038a336bed6ccb5901c1437494e34312cfdc4ad`] (4.1).
    ///
    /// [`5038a336bed6ccb5901c1437494e34312cfdc4ad`]: https://github.com/godotengine/godot/commit/5038a336bed6ccb5901c1437494e34312cfdc4ad
    #[serde(default)]
    pub has_72979_annotation_parsing: Option<bool>,

    /// Whether the suite changes from #77744 exist. Official builds fixed this in
    /// [`f3bf75fbb4edf5d73cdedaf196fdcd358e031c82`] (4.1).
    ///
    /// [`f3bf75fbb4edf5d73cdedaf196fdcd358e031c82`]: https://github.com/godotengine/godot/commit/f3bf75fbb4edf5d73cdedaf196fdcd358e031c82
    #[serde(default)]
    pub has_77744_suite_changes: Option<bool>,
}

macro_rules! expand_resolve {
    (#error $self:ident $struct:ident { $($field:ident),*$(,)? }) => {
        $struct {
            $(
                $field: $self.$field.ok_or_eyre(concat!("missing field `", stringify!($field), "`"))?,
            )*
        }
    };

    ($parent:ident $self:ident $struct:ident { $($field:ident),*$(,)? }) => {
        $struct {
            $(
                $field: $self
                    .$field
                    .unwrap_or_else(|| $parent.$field.clone()),
            )*
        }
    };
}

impl SerializedGDScriptBuild {
    fn resolve(self, parent: Option<&GDScriptBuild>) -> crate::Result<GDScriptBuild, Report> {
        Ok(match (self.version, parent) {
            // version + no parent, parse from scratch
            (Some(1), None) => {
                bail!("GDScript V1 isn't implemented yet");
            }

            (Some(2), None) => GDScriptBuild::V2(expand_resolve!(#error self GDScriptV2Build {
                tokenizer_version,
                tokens,
                allow_mixed_indentation_when_multiline,
                has_improved_invalid_character_error,
                has_literal_sign_handling,
                has_new_number_underscore_parsing,
                has_raw_strings,
                has_when,
                need_digits_in_hex_and_binary,
                has_fixed_continuation_lines,
                has_uppercase_number_types,
                has_variadic_functions,
                expands_tabs_in_span_column,
                allow_zwsp_as_whitespace,
                allow_mixed_indentation_on_blank_lines,
                has_extra_word_in_binary_script_header,
                has_fixed_multiline_handling_in_super_calls,
                has_early_bail_in_super_calls,
                allow_multiline_array_dictionary_patterns,
                allow_preload_trailing_comma,
                has_static_variables,
                has_is_not,
                allow_empty_parentheses_in_getter_declaration,
                has_match_error_recovery,
                has_dictionary_error_recovery,
                has_typed_for_loops,
                allow_keywords_as_attributes,
                has_72979_annotation_parsing,
                has_77744_suite_changes,
            })),

            // inherit from parent
            (None, Some(GDScriptBuild::V1(_parent))) => {
                bail!("GDScript V1 isn't implemented yet");
            }

            (None, Some(GDScriptBuild::V2(parent))) => {
                GDScriptBuild::V2(expand_resolve!(parent self GDScriptV2Build {
                    tokenizer_version,
                    tokens,
                    allow_mixed_indentation_when_multiline,
                    has_improved_invalid_character_error,
                    has_literal_sign_handling,
                    has_new_number_underscore_parsing,
                    has_raw_strings,
                    has_when,
                    need_digits_in_hex_and_binary,
                    has_fixed_continuation_lines,
                    has_uppercase_number_types,
                    has_variadic_functions,
                    expands_tabs_in_span_column,
                    allow_zwsp_as_whitespace,
                    allow_mixed_indentation_on_blank_lines,
                    has_extra_word_in_binary_script_header,
                    has_fixed_multiline_handling_in_super_calls,
                    has_early_bail_in_super_calls,
                    allow_multiline_array_dictionary_patterns,
                    allow_preload_trailing_comma,
                    has_static_variables,
                    has_is_not,
                    allow_empty_parentheses_in_getter_declaration,
                    has_match_error_recovery,
                    has_dictionary_error_recovery,
                    has_typed_for_loops,
                    allow_keywords_as_attributes,
                    has_72979_annotation_parsing,
                    has_77744_suite_changes,
                }))
            }

            // specified a version and a parent
            (Some(_), Some(_)) => {
                bail!("specified a version and a parent at the same time in a GDScript build");
            }

            // no parent or version specified
            (None, None) => {
                bail!("GDScript builds require either a parent or a version number");
            }

            // invalid version specified
            (Some(n), _) => {
                bail!(
                    "invalid GDScript version number {}, only 1 and 2 are valid",
                    n
                );
            }
        })
    }
}

/// Metadata about the engine version to parse for.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct EngineBuild {
    /// Version info.
    pub version: VersionSpecifier,

    /// Commit hash this engine was built from, if known.
    pub commit: Option<String>,

    /// GDScript information (token order + builtin function order for GDScript 1.x)
    pub gdscript: GDScriptBuild,

    /// Whether paths in the pack file no longer contain the `res://` prefix or not. Official builds changed this in
    /// [`2ac562cdf8366876381902a0667fec704e357495`] (4.4).
    ///
    /// [`2ac562cdf8366876381902a0667fec704e357495`]: https://github.com/godotengine/godot/commit/2ac562cdf8366876381902a0667fec704e357495
    pub has_prefixless_pck_paths: bool,
}

impl Display for EngineBuild {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut s = self.version.to_string();

        if let Some(commit) = &self.commit {
            s.push_str(&format!(" ({commit})"));
        }

        f.write_str(&s)
    }
}

/// List of known builds.
#[derive(Debug, Clone)]
pub struct EngineBuilds(BTreeMap<VersionSpecifier, EngineBuild>);

pub enum FindApproximateBuildResult<'builds> {
    /// No builds exist with a matching major version.
    NoMatchingMajor,

    /// A build was found with a correct major but differing minor version.
    DifferingMinor(&'builds EngineBuild),

    /// A build with a matching major and minor version but differing patch version was found.

    /// A build with an exact version match was found.
    Matching(&'builds EngineBuild),
}

impl EngineBuilds {
    /// Resolves parent versions into flat builds, using the existing builds as a base.
    pub fn resolve(
        &self,
        serialized_builds: &mut SerializedEngineBuilds,
    ) -> crate::Result<EngineBuilds, Report> {
        // already resolved entries
        let mut resolved = BTreeMap::new();
        for (version, build) in &self.0 {
            resolved.insert(version.clone(), build.clone());
        }

        // stack of entries that are waiting to resolve due to their parents not being resolved yet
        let mut current_stack = Vec::new();

        fn resolve<'a>(
            serialized_builds: &mut HashMap<VersionSpecifier, SerializedEngineBuild>,
            resolved_builds: &'a mut BTreeMap<VersionSpecifier, EngineBuild>,
            current_stack: &mut Vec<VersionSpecifier>,
            version: VersionSpecifier,
        ) -> crate::Result<&'a EngineBuild, Report> {
            current_stack.push(version.clone());

            // check if already resolved
            if resolved_builds.contains_key(&version) {
                return Ok(&resolved_builds[&version]);
            }

            // remove build from builds list
            let serialized_build = serialized_builds
                .remove(&version)
                .ok_or_else(|| eyre!("unknown build {} in parent dependencies", &version))?;

            // resolve parent first
            let parent = match &serialized_build.parent {
                None => None,
                Some(parent_version) => Some(resolve(
                    serialized_builds,
                    resolved_builds,
                    current_stack,
                    parent_version.clone(),
                )?),
            };

            let build = serialized_build.resolve(version.clone(), parent)?;

            let btree_map::Entry::Vacant(entry) = resolved_builds.entry(version.clone()) else {
                unreachable!()
            };

            let build_ref = entry.insert(build);
            current_stack.pop();

            Ok(&*build_ref)
        }

        // remove random entry from builds and resolve it
        while let Some(version) = serialized_builds.0.keys().next().cloned() {
            resolve(
                &mut serialized_builds.0,
                &mut resolved,
                &mut current_stack,
                version,
            )?;
        }

        Ok(EngineBuilds(resolved))
    }

    /// Finds a build that approximately matches a version specifier.
    ///
    /// Builds are matched by the major version and flavor first (both must be equal to the
    /// specified version), and then by minor and patch versions (must be less than or equal).
    pub fn find_approximate_build(
        &self,
        requested_version: &VersionSpecifier,
    ) -> Option<&EngineBuild> {
        // self.0 is sorted by version, so filtering gives us the right match already
        self.0
            .iter()
            .filter(|entry| {
                entry.0.flavor == requested_version.flavor
                    && entry.0.major == requested_version.major
                    && entry.0.minor <= requested_version.minor
                    && entry.0.patch <= requested_version.patch
                    && entry.0.sub_patch <= requested_version.sub_patch
            })
            .map(|entry| entry.1)
            .next_back()
    }

    /// Finds a build that exactly matches the requested one.
    pub fn find_exact_build(&self, requested_version: &VersionSpecifier) -> Option<&EngineBuild> {
        self.0.get(requested_version)
    }
}

/// A version specifier of the form `X.Y.Z.W-flavor`.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[non_exhaustive]
pub struct VersionSpecifier {
    /// The major version number (first digit).
    pub major: u32,

    /// The minor version number (second digit).
    pub minor: u32,

    /// The patch version number (third digit).
    pub patch: u32,

    /// The "sub-patch" version number (forth digit). Official Godot builds have only used this
    /// once.
    pub sub_patch: u32,

    /// Engine flavor.
    pub flavor: String,
}

impl PartialOrd for VersionSpecifier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VersionSpecifier {
    fn cmp(&self, other: &Self) -> Ordering {
        self.flavor
            .cmp(&other.flavor)
            .then_with(|| self.major.cmp(&other.major))
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
            .then_with(|| self.sub_patch.cmp(&other.sub_patch))
    }
}

impl Display for VersionSpecifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut s = format!("{}.{}", self.major, self.minor);

        if self.patch != 0 {
            s.push_str(&format!(".{}", self.patch));
        }

        if self.sub_patch != 0 {
            s.push_str(&format!(".{}", self.sub_patch));
        }

        s.push('-');
        s.push_str(&self.flavor);
        write!(f, "{}", s)
    }
}

#[derive(Debug, Error)]
#[error("invalid version specifier")]
pub struct VersionSpecifierParseError;

impl FromStr for VersionSpecifier {
    type Err = VersionSpecifierParseError;

    fn from_str(s: &str) -> crate::Result<Self, Self::Err> {
        let (version, flavor) = s.split_once('-').ok_or(VersionSpecifierParseError)?;
        let parts = version.split('.').collect::<Vec<_>>();

        if parts.len() < 2 || parts.len() > 4 {
            return Err(VersionSpecifierParseError);
        }

        let major = u32::from_str(parts[0]).map_err(|_| VersionSpecifierParseError)?;
        let minor = u32::from_str(parts[1]).map_err(|_| VersionSpecifierParseError)?;

        let patch = if let Some(patch) = parts.get(2) {
            u32::from_str(patch).map_err(|_| VersionSpecifierParseError)?
        } else {
            0
        };

        let sub_patch = if let Some(sub_patch) = parts.get(3) {
            u32::from_str(sub_patch).map_err(|_| VersionSpecifierParseError)?
        } else {
            0
        };

        Ok(VersionSpecifier {
            major,
            minor,
            patch,
            sub_patch,
            flavor: flavor.to_string(),
        })
    }
}

impl<'de> Deserialize<'de> for VersionSpecifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <&'de str>::deserialize(deserializer)?;
        VersionSpecifier::from_str(s).map_err(serde::de::Error::custom)
    }
}

impl Serialize for VersionSpecifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self.to_string();
        serializer.serialize_str(&s)
    }
}

impl VersionSpecifier {
    pub fn new(
        major: u32,
        minor: u32,
        patch: u32,
        sub_patch: u32,
        flavor: impl Into<String>,
    ) -> Self {
        Self {
            major,
            minor,
            patch,
            sub_patch,
            flavor: flavor.into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[non_exhaustive]
#[serde(deny_unknown_fields)]
pub struct SerializedEngineBuild {
    /// Parent build ID to inherit from.
    #[serde(default)]
    pub parent: Option<VersionSpecifier>,

    /// Commit hash this engine was built from, if known.
    #[serde(default)]
    pub commit: Option<String>,

    /// GDScript information.
    #[serde(default)]
    pub gdscript: Option<SerializedGDScriptBuild>,

    /// Whether paths in the pack file no longer contain the `res://` prefix or not. Official builds changed this in
    /// [`2ac562cdf8366876381902a0667fec704e357495`] (4.4).
    ///
    /// [`2ac562cdf8366876381902a0667fec704e357495`]: https://github.com/godotengine/godot/commit/2ac562cdf8366876381902a0667fec704e357495
    #[serde(default)]
    pub has_prefixless_pck_paths: Option<bool>,
}

impl SerializedEngineBuild {
    pub fn resolve(
        self,
        version: VersionSpecifier,
        parent: Option<&EngineBuild>,
    ) -> crate::Result<EngineBuild, Report> {
        let gdscript = match (self.gdscript, parent) {
            (None, None) => {
                bail!("build is missing field `gdscript`");
            }
            (None, Some(parent)) => parent.gdscript.clone(),
            (Some(build), parent) => build.clone().resolve(parent.map(|p| &p.gdscript))?,
        };

        Ok(EngineBuild {
            version,
            commit: self.commit,
            gdscript,

            has_prefixless_pck_paths: self
                .has_prefixless_pck_paths
                .or(parent.map(|p| p.has_prefixless_pck_paths))
                .ok_or_eyre(
                    "missing `has_prefixless_pck_paths` in engine build and no parent specified",
                )?,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SerializedEngineBuilds(pub HashMap<VersionSpecifier, SerializedEngineBuild>);

impl SerializedEngineBuilds {
    /// Resolves parent versions into flat builds.
    pub fn resolve(mut self) -> crate::Result<EngineBuilds, Report> {
        // already resolved entries
        let mut resolved = BTreeMap::new();

        // stack of entries that are waiting to resolve due to their parents not being resolved yet
        let mut current_stack = Vec::new();

        fn resolve<'a>(
            serialized_builds: &mut HashMap<VersionSpecifier, SerializedEngineBuild>,
            resolved_builds: &'a mut BTreeMap<VersionSpecifier, EngineBuild>,
            current_stack: &mut Vec<VersionSpecifier>,
            version: VersionSpecifier,
        ) -> crate::Result<&'a EngineBuild, Report> {
            current_stack.push(version.clone());

            // check if already resolved
            if resolved_builds.contains_key(&version) {
                return Ok(&resolved_builds[&version]);
            }

            // remove build from builds list
            let serialized_build = serialized_builds
                .remove(&version)
                .ok_or_else(|| eyre!("unknown build {} in parent dependencies", &version))?;

            // resolve parent first
            let parent = match &serialized_build.parent {
                None => None,
                Some(parent_version) => Some(resolve(
                    serialized_builds,
                    resolved_builds,
                    current_stack,
                    parent_version.clone(),
                )?),
            };

            let build = serialized_build.resolve(version.clone(), parent)?;

            let btree_map::Entry::Vacant(entry) = resolved_builds.entry(version.clone()) else {
                unreachable!()
            };

            let build_ref = entry.insert(build);
            current_stack.pop();

            Ok(&*build_ref)
        }

        // remove random entry from builds and resolve it
        while let Some(version) = self.0.keys().next().cloned() {
            resolve(&mut self.0, &mut resolved, &mut current_stack, version)?;
        }

        Ok(EngineBuilds(resolved))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SerializedBuildsFile {
    pub engine: SerializedEngineBuilds,
}

impl SerializedBuildsFile {
    pub fn resolve(self) -> crate::Result<EngineBuilds, Report> {
        self.engine.resolve()
    }
}

static BUNDLED_BUILDS: LazyLock<EngineBuilds> = LazyLock::new(|| {
    static BUILDS_TEXT: &str = include_str!("../builds.toml");

    let unresolved = toml::from_str::<SerializedBuildsFile>(BUILDS_TEXT)
        .expect("bundled builds.toml is invalid");

    unresolved
        .resolve()
        .expect("bundled builds.toml is invalid")
});

pub fn bundled_builds() -> &'static EngineBuilds {
    &BUNDLED_BUILDS
}

pub fn resolve_approximate_build(
    pack_version: VersionSpecifier,
    custom_engine: Option<SerializedEngineBuild>,
) -> color_eyre::Result<EngineBuild> {
    const CUSTOM_BUILD_FLAVOR: &str = "custom"; // TODO: should we change this?

    // Resolve the pack version ahead of time, so we know what to use as a parent.
    let bundled_builds = bundled_builds();
    let pack_engine_build = bundled_builds
        .find_approximate_build(&pack_version)
        .context("failed to resolve pack engine build")?
        .clone();

    let custom_version = if let Some(parent) = custom_engine.as_ref().and_then(|p| p.parent.clone())
    {
        // Use the engine's parent version, with a custom flavor.
        Some((
            VersionSpecifier::new(
                parent.major,
                parent.minor,
                parent.patch,
                parent.sub_patch,
                CUSTOM_BUILD_FLAVOR,
            ),
            parent,
        ))
    } else if custom_engine.is_some() {
        // Use the pack file's resolved version, with a custom flavor.
        let pack_version = pack_engine_build.version.clone();

        Some((
            VersionSpecifier::new(
                pack_version.major,
                pack_version.minor,
                pack_version.patch,
                pack_version.sub_patch,
                CUSTOM_BUILD_FLAVOR,
            ),
            pack_version,
        ))
    } else {
        None
    };

    if let Some((version, parent)) = custom_version {
        // Add our custom engine build.
        let mut custom_engine = custom_engine.unwrap_or_default();

        // Set the parent engine version if it wasn't set by the user.
        if custom_engine.parent.is_none() {
            custom_engine.parent = Some(parent);
        }

        // Add them to the build catalog and resolve our custom version.
        let mut serialized_builds = SerializedEngineBuilds::default();
        serialized_builds.0.insert(version.clone(), custom_engine);

        let custom_builds = bundled_builds
            .resolve(&mut serialized_builds)
            .wrap_err("failed to resolve custom engine builds")?;

        custom_builds
            .find_exact_build(&version)
            .wrap_err("failed to resolve custom engine build")
            .cloned()
    } else {
        // No custom version to worry about, just return the pack engine build.
        Ok(pack_engine_build)
    }
}
