use std::collections::HashMap;
use std::sync::Arc;

use crate::llm::provider::ToolDef;
use crate::tools::permission::Permission;
use crate::tools::r#trait::{Tool, ToolError, ToolResult};

/// Registry of available tools.
///
/// Tools are stored as `Arc<dyn Tool>` so that the registry can be cheaply
/// cloned (shared via Arc) while still allowing registration at init time.
#[derive(Clone)]
pub struct ToolRegistry {
    tools: Arc<HashMap<String, Arc<dyn Tool>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(HashMap::new()),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        let arc_tool: Arc<dyn Tool> = Arc::from(tool);
        // Arc::make_mut requires Clone on the inner HashMap values, which
        // Arc<dyn Tool> satisfies. This will clone the HashMap only when
        // there are multiple references (i.e. after clone).
        let tools = Arc::make_mut(&mut self.tools);
        tools.insert(name, arc_tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn definitions(&self) -> Vec<ToolDef> {
        self.tools
            .values()
            .map(|t| ToolDef {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters(),
            })
            .collect()
    }

    pub async fn execute(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        match self.tools.get(name) {
            Some(tool) => tool.execute(args).await,
            None => Err(ToolError::NotFound(name.to_string())),
        }
    }

    pub fn permission(&self, name: &str) -> Option<Permission> {
        self.tools.get(name).map(|t| t.permission())
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct DummyTool;

    #[async_trait]
    impl Tool for DummyTool {
        fn name(&self) -> &'static str {
            "dummy"
        }

        fn description(&self) -> &'static str {
            "Dummy tool"
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }

        fn permission(&self) -> Permission {
            Permission::NoConfirm
        }

        async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult, ToolError> {
            Ok(ToolResult {
                success: true,
                data: serde_json::json!({"ok": true}),
                message: None,
            })
        }
    }

    #[test]
    fn test_empty_registry() {
        let reg = ToolRegistry::new();
        assert!(reg.get("nonexistent").is_none());
        assert!(reg.definitions().is_empty());
    }

    #[test]
    fn test_register_and_get() {
        let mut reg = ToolRegistry::new();
        reg.register(Box::new(DummyTool));
        assert!(reg.get("dummy").is_some());
    }

    #[test]
    fn test_definitions_returns_all() {
        let mut reg = ToolRegistry::new();
        reg.register(Box::new(DummyTool));
        let defs = reg.definitions();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "dummy");
    }

    #[tokio::test]
    async fn test_execute_registered_tool() {
        let mut reg = ToolRegistry::new();
        reg.register(Box::new(DummyTool));
        let result = reg.execute("dummy", serde_json::json!({})).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_execute_unknown_tool_returns_error() -> Result<(), Box<dyn std::error::Error>> {
        let reg = ToolRegistry::new();
        let result = reg.execute("unknown", serde_json::json!({})).await;
        assert!(matches!(result, Err(ToolError::NotFound(_))));
        Ok(())
    }

    #[test]
    fn test_permission_of_unknown_tool() {
        let reg = ToolRegistry::new();
        assert!(reg.permission("unknown").is_none());
    }
}
