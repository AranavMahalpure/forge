mod agent;
mod chat_request;
mod chat_response;
mod chat_stream_ext;
mod command;
mod config;
mod context;
mod conversation;
mod environment;
mod errata;
mod error;
mod file;
mod learning;
mod message;
mod model;
mod provider;
mod snapshot;
mod snapshot_provider;
mod stream_ext;
mod tool;
mod tool_call;
mod tool_call_parser;
mod tool_choice;
mod tool_definition;
mod tool_name;
mod tool_result;
mod tool_service;
mod tool_usage;
mod user_interaction;

pub use agent::*;
pub use chat_request::*;
pub use chat_response::*;
pub use chat_stream_ext::*;
pub use command::*;
pub use config::*;
pub use context::*;
pub use conversation::*;
pub use environment::*;
pub use errata::*;
pub use error::*;
pub use file::*;
pub use learning::*;
pub use message::*;
pub use model::*;
pub use provider::*;
pub use snapshot::*;
pub use snapshot_provider::*;
pub use stream_ext::*;
pub use tool::*;
pub use tool_call::*;
pub use tool_call_parser::*;
pub use tool_choice::*;
pub use tool_definition::*;
pub use tool_name::*;
pub use tool_result::*;
pub use tool_service::*;
pub use tool_usage::*;
pub use user_interaction::*;

/// Core domain trait providing access to services and repositories.
/// This trait follows clean architecture principles for dependency management
/// and service/repository composition.
pub trait ForgeDomain {
    /// The concrete type implementing file read service capabilities
    type FileReadService: file::FileReadService;
    /// The concrete type implementing tool service capabilities
    type ToolService: ToolService;
    /// The concrete type implementing provider service capabilities
    type ProviderService: ProviderService;
    /// The concrete type implementing conversation repository
    type ConversationRepo: ConversationRepository;
    /// The concrete type implementing configuration repository
    type ConfigRepo: ConfigRepository;
    /// The concrete type implementing environment repository
    type EnvironmentRepo: EnvironmentRepository;
    /// The concrete type implementing snapshot repository
    type SnapshotRepo: SnapshotRepository;
    /// The concrete type implementing snapshot provider service
    type SnapshotProv: SnapshotProvider;

    /// Get a reference to the tool service instance
    fn tool_service(&self) -> &Self::ToolService;
    /// Get a reference to the provider service instance
    fn provider_service(&self) -> &Self::ProviderService;
    /// Get a reference to the conversation repository instance
    fn conversation_repository(&self) -> &Self::ConversationRepo;
    /// Get a reference to the configuration repository instance
    fn config_repository(&self) -> &Self::ConfigRepo;
    /// Get a reference to the environment repository instance
    fn environment_repository(&self) -> &Self::EnvironmentRepo;
    /// Get a reference to the file read service instance
    fn file_read_service(&self) -> &Self::FileReadService;
    /// Get a reference to the snapshot repository instance
    fn snapshot_repository(&self) -> &Self::SnapshotRepo;
    /// Get a reference to the snapshot provider service instance
    fn snapshot_provider(&self) -> &Self::SnapshotProv;
}
