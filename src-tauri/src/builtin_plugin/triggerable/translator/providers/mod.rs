pub mod mock;
pub mod openai_compatible;

pub use mock::{
    mirror_from as mock_mirror_from, placeholder_result as mock_placeholder_result, MockProvider,
    PROVIDER_ID as MOCK_PROVIDER_ID, PROVIDER_NAME as MOCK_PROVIDER_NAME,
};
pub use openai_compatible::{LlmConfig, OpenAiCompatibleProvider, PROVIDER_ID};
