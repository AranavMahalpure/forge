use forge_app::{EnvironmentService, Infrastructure};

use crate::embedding::OpenAIEmbeddingService;
use crate::env::ForgeEnvironmentService;
use crate::file_read::ForgeFileService;
use crate::qdrant::QdrantVectorIndex;

pub struct ForgeInfra {
    file_service: ForgeFileService,
    environment_service: ForgeEnvironmentService,
    information_repo: QdrantVectorIndex,
    embedding_service: OpenAIEmbeddingService,
}

impl ForgeInfra {
    pub fn new(restricted: bool) -> Self {
        let _environment_service = ForgeEnvironmentService::new(restricted);
        let env = _environment_service.get_environment();
        Self {
            file_service: ForgeFileService::new(),
            environment_service: _environment_service,
            information_repo: QdrantVectorIndex::new(env.clone(), "user_feedback"),
            embedding_service: OpenAIEmbeddingService::new(env),
        }
    }
}

impl Infrastructure for ForgeInfra {
    type EnvironmentService = ForgeEnvironmentService;
    type FileService = ForgeFileService;
    type VectorIndex = QdrantVectorIndex;
    type EmbeddingService = OpenAIEmbeddingService;

    fn environment_service(&self) -> &Self::EnvironmentService {
        &self.environment_service
    }

    fn file_service(&self) -> &Self::FileService {
        &self.file_service
    }

    fn vector_index(&self) -> &Self::VectorIndex {
        &self.information_repo
    }

    fn embedding_service(&self) -> &Self::EmbeddingService {
        &self.embedding_service
    }
}
