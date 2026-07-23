mod plugin;
mod provider;
mod providers;
mod query_parser;
mod registry;

pub use plugin::TranslatorPlugin;
pub use provider::{
    LanguageSupport, TranslateRequest, TranslationProvider, TranslationResult,
};
pub use query_parser::{
    parse_search_term, LangCatalog, LanguageCode, ParseError, ParsedQuery,
};
