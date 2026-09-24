use crate::tools::permission::Permission;
use crate::tools::registry::ToolRegistry;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
}

#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub reason: String,
    pub status: ApprovalStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub enum GuardrailResult {
    Allowed { notify: bool },
    RequiresApproval { request_id: String },
}

#[derive(Debug, thiserror::Error)]
pub enum GuardrailError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Approval request not found: {0}")]
    RequestNotFound(String),
    #[error("Already resolved")]
    AlreadyResolved,
}

pub struct Guardrails {
    registry: Arc<ToolRegistry>,
    pending_approvals: Arc<Mutex<HashMap<String, ApprovalRequest>>>,
}

impl Guardrails {
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self {
            registry,
            pending_approvals: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn check(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Result<GuardrailResult, GuardrailError> {
        let perm = self
            .registry
            .permission(tool_name)
            .ok_or_else(|| GuardrailError::ToolNotFound(tool_name.to_string()))?;
        match perm {
            Permission::NoConfirm => Ok(GuardrailResult::Allowed { notify: false }),
            Permission::Notify => Ok(GuardrailResult::Allowed { notify: true }),
            Permission::ExplicitApproval => {
                let request = self.create_approval_request(tool_name, args.clone());
                Ok(GuardrailResult::RequiresApproval {
                    request_id: request.id.clone(),
                })
            }
        }
    }

    fn create_approval_request(&self, tool_name: &str, args: serde_json::Value) -> ApprovalRequest {
        let request = ApprovalRequest {
            id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool_name.to_string(),
            arguments: args,
            reason: format!("Tool '{}' requires explicit approval", tool_name),
            status: ApprovalStatus::Pending,
            created_at: Utc::now(),
        };
        let mut pending = self.pending_approvals.lock().unwrap();
        pending.insert(request.id.clone(), request.clone());
        request
    }

    pub fn resolve_approval(&self, request_id: &str, approved: bool) -> Result<(), GuardrailError> {
        let mut pending = self.pending_approvals.lock().unwrap();
        let request = pending
            .get_mut(request_id)
            .ok_or_else(|| GuardrailError::RequestNotFound(request_id.to_string()))?;
        if !matches!(request.status, ApprovalStatus::Pending) {
            return Err(GuardrailError::AlreadyResolved);
        }
        request.status = if approved {
            ApprovalStatus::Approved
        } else {
            ApprovalStatus::Denied
        };
        Ok(())
    }

    pub fn get_pending_approval(&self, request_id: &str) -> Option<ApprovalRequest> {
        let pending = self.pending_approvals.lock().unwrap();
        pending.get(request_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::permission::Permission;
    use crate::tools::r#trait::{Tool, ToolError as ToolErr, ToolResult};
    use async_trait::async_trait;

    struct NoConfirmTool;
    #[async_trait]
    impl Tool for NoConfirmTool {
        fn name(&self) -> &'static str {
            "read_tool"
        }
        fn description(&self) -> &'static str {
            "Read only"
        }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }
        fn permission(&self) -> Permission {
            Permission::NoConfirm
        }
        async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult, ToolErr> {
            Ok(ToolResult {
                success: true,
                data: serde_json::json!({}),
                message: None,
            })
        }
    }

    struct ApproveTool;
    #[async_trait]
    impl Tool for ApproveTool {
        fn name(&self) -> &'static str {
            "delete_tool"
        }
        fn description(&self) -> &'static str {
            "Deletes things"
        }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }
        fn permission(&self) -> Permission {
            Permission::ExplicitApproval
        }
        async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult, ToolErr> {
            Ok(ToolResult {
                success: true,
                data: serde_json::json!({}),
                message: None,
            })
        }
    }

    fn make_registry() -> Arc<ToolRegistry> {
        let mut reg = ToolRegistry::new();
        reg.register(Box::new(NoConfirmTool));
        reg.register(Box::new(ApproveTool));
        Arc::new(reg)
    }

    #[test]
    fn test_no_confirm_passes() {
        let g = Guardrails::new(make_registry());
        let result = g.check("read_tool", &serde_json::json!({})).unwrap();
        match result {
            GuardrailResult::Allowed { notify } => assert!(!notify),
            _ => panic!("Expected Allowed"),
        }
    }

    #[test]
    fn test_explicit_approval_requires_approval() {
        let g = Guardrails::new(make_registry());
        let result = g.check("delete_tool", &serde_json::json!({})).unwrap();
        match result {
            GuardrailResult::RequiresApproval { request_id } => {
                assert!(!request_id.is_empty());
                let req = g.get_pending_approval(&request_id).unwrap();
                assert_eq!(req.tool_name, "delete_tool");
                assert!(matches!(req.status, ApprovalStatus::Pending));
            }
            _ => panic!("Expected RequiresApproval"),
        }
    }

    #[test]
    fn test_resolve_approval_approved() {
        let g = Guardrails::new(make_registry());
        let result = g.check("delete_tool", &serde_json::json!({})).unwrap();
        if let GuardrailResult::RequiresApproval { request_id } = result {
            g.resolve_approval(&request_id, true).unwrap();
            let req = g.get_pending_approval(&request_id).unwrap();
            assert!(matches!(req.status, ApprovalStatus::Approved));
        }
    }

    #[test]
    fn test_resolve_approval_denied() {
        let g = Guardrails::new(make_registry());
        let result = g.check("delete_tool", &serde_json::json!({})).unwrap();
        if let GuardrailResult::RequiresApproval { request_id } = result {
            g.resolve_approval(&request_id, false).unwrap();
            let req = g.get_pending_approval(&request_id).unwrap();
            assert!(matches!(req.status, ApprovalStatus::Denied));
        }
    }

    #[test]
    fn test_unknown_tool_returns_error() {
        let g = Guardrails::new(make_registry());
        let result = g.check("nonexistent", &serde_json::json!({}));
        assert!(matches!(result, Err(GuardrailError::ToolNotFound(_))));
    }

    #[test]
    fn test_double_resolve_returns_error() {
        let g = Guardrails::new(make_registry());
        let result = g.check("delete_tool", &serde_json::json!({})).unwrap();
        if let GuardrailResult::RequiresApproval { request_id } = result {
            g.resolve_approval(&request_id, true).unwrap();
            let err = g.resolve_approval(&request_id, true).unwrap_err();
            assert!(matches!(err, GuardrailError::AlreadyResolved));
        }
    }

    #[test]
    fn test_notify_allowed() {
        let mut reg = ToolRegistry::new();
        struct NotifyTool;
        #[async_trait]
        impl Tool for NotifyTool {
            fn name(&self) -> &'static str {
                "notify_tool"
            }
            fn description(&self) -> &'static str {
                "Notify tool"
            }
            fn parameters(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }
            fn permission(&self) -> Permission {
                Permission::Notify
            }
            async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult, ToolErr> {
                Ok(ToolResult {
                    success: true,
                    data: serde_json::json!({}),
                    message: None,
                })
            }
        }
        reg.register(Box::new(NotifyTool));
        let g = Guardrails::new(Arc::new(reg));
        let result = g.check("notify_tool", &serde_json::json!({})).unwrap();
        match result {
            GuardrailResult::Allowed { notify } => assert!(notify),
            _ => panic!("Expected Allowed with notify"),
        }
    }
}
