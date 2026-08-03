pub mod mock;
pub mod openai_compatible;

pub use mock::{
    mirror_from as mock_mirror_from, placeholder_result as mock_placeholder_result, MockProvider,
    PROVIDER_ID as MOCK_PROVIDER_ID,
};
pub use openai_compatible::{LlmConfig, OpenAiCompatibleProvider, PROVIDER_ID};
