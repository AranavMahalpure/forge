use std::path::Path;
use std::sync::Arc;

use anyhow::Context;
use forge_app::{FsReadService, Infrastructure};
use forge_domain::Workflow;
use merge::Merge;

// Default forge.yaml content embedded in the binary
const DEFAULT_FORGE_WORKFLOW: &str = include_str!("../../../forge.default.yaml");

/// Represents the possible sources of a workflow configuration
enum WorkflowSource<'a> {
    /// Explicitly provided path
    ExplicitPath(&'a Path),
    /// Default configuration embedded in the binary
    Default,
    /// Project-specific configuration in the current directory
    ProjectConfig,
}

/// A workflow loader to load the workflow from the given path.
/// It also resolves the internal paths specified in the workflow.
pub struct ForgeLoaderService<F>(Arc<F>);

impl<F> ForgeLoaderService<F> {
    pub fn new(app: Arc<F>) -> Self {
        Self(app)
    }
}

impl<F: Infrastructure> ForgeLoaderService<F> {
    /// Loads the workflow from the given path.
    /// If a path is provided, uses that workflow directly without merging.
    /// If no path is provided:
    ///   - Loads from current directory's forge.yaml merged with defaults (if
    ///     forge.yaml exists)
    ///   - Falls back to embedded default if forge.yaml doesn't exist
    ///
    /// When merging, the project's forge.yaml values take precedence over
    /// defaults.
    pub async fn load(&self, path: Option<&Path>) -> anyhow::Result<Workflow> {
        // Determine the workflow source
        let source = match path {
            Some(path) => WorkflowSource::ExplicitPath(path),
            None if Path::new("forge.yaml").exists() => WorkflowSource::ProjectConfig,
            None => WorkflowSource::Default,
        };

        // Load the workflow based on its source
        match source {
            WorkflowSource::ExplicitPath(path) => self.load_from_explicit_path(path).await,
            WorkflowSource::Default => self.load_default_workflow(),
            WorkflowSource::ProjectConfig => self.load_with_project_config().await,
        }
    }

    /// Loads a workflow from a specific file path
    async fn load_from_explicit_path(&self, path: &Path) -> anyhow::Result<Workflow> {
        let content = String::from_utf8(self.0.file_read_service().read(path).await?.to_vec())?;
        let workflow: Workflow = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse workflow from {}", path.display()))?;
        Ok(workflow)
    }

    /// Loads the default workflow from embedded content
    fn load_default_workflow(&self) -> anyhow::Result<Workflow> {
        let workflow: Workflow = serde_yaml::from_str(DEFAULT_FORGE_WORKFLOW)
            .with_context(|| "Failed to parse default workflow")?;
        Ok(workflow)
    }

    /// Loads workflow by merging project config with default workflow
    async fn load_with_project_config(&self) -> anyhow::Result<Workflow> {
        let default_workflow = self.load_default_workflow()?;
        let project_path = Path::new("forge.yaml");

        let project_content = String::from_utf8(
            self.0
                .file_read_service()
                .read(project_path)
                .await?
                .to_vec(),
        )?;

        let project_workflow: Workflow = serde_yaml::from_str(&project_content)
            .with_context(|| "Failed to parse project workflow")?;

        // Merge workflows with project taking precedence
        let mut merged_workflow = default_workflow;
        merged_workflow.merge(project_workflow);

        Ok(merged_workflow)
    }
}
