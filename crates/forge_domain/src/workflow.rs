use std::collections::HashMap;

use merge::Merge;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Agent, AgentId};

#[derive(Default, Debug, Clone, Serialize, Deserialize, Merge)]
pub struct Workflow {
    #[merge(strategy = crate::merge::vec::unify_by_key)]
    pub agents: Vec<Agent>,
    pub variables: Option<HashMap<String, Value>>,
}

impl Workflow {
    fn find_agent(&self, id: &AgentId) -> Option<&Agent> {
        self.agents
            .iter()
            .filter(|a| a.enable)
            .find(|a| a.id == *id)
    }

    pub fn get_agent(&self, id: &AgentId) -> crate::Result<&Agent> {
        self.find_agent(id)
            .ok_or_else(|| crate::Error::AgentUndefined(id.clone()))
    }
}
