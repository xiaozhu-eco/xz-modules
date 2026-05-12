pub mod mock;

#[cfg(feature = "openai")]
pub mod openai;

pub use mock::MockEmbedder;
#[cfg(feature = "openai")]
pub use openai::OpenAiEmbedder;
