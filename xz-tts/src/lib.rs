pub mod error;
pub mod credential;
pub mod traits;
pub mod types;
pub mod protocol;
#[cfg(feature = "volcengine")]
pub mod client;
#[cfg(feature = "volcengine")]
pub mod pool;
pub mod voices;
pub mod preprocess;
pub mod session;
pub mod observability;
#[cfg(feature = "async-api")]
pub mod async_types;
#[cfg(feature = "async-api")]
pub mod async_client;
#[cfg(feature = "async-api")]
pub(crate) mod http_client;

pub use error::XzTtsError;
pub use credential::{CredentialProvider, ResolvedTtsCredential, StaticCredential};
pub use traits::StreamingTts;
pub use types::{AudioFormat, AudioFrame, TtsSessionConfig, TtsVoiceInfo};
#[cfg(feature = "voice-mix")]
pub use types::MixSpeaker;
#[cfg(feature = "volcengine")]
pub use client::VolcengineTtsClient;
#[cfg(feature = "volcengine")]
pub use pool::VolcengineTtsPool;
pub use voices::{builtin_voices, VoiceRegistry};
pub use preprocess::{NoOpPreprocessor, TextPreprocessor, VoiceCommandExtractor};
pub use session::build_start_session_json;
pub use observability::{TtsMetrics, TtsMetricsSnapshot};
#[cfg(feature = "async-api")]
pub use async_types::*;
#[cfg(feature = "async-api")]
pub use async_client::{AsyncTtsClient, AsyncTtsSubmitConfig};
