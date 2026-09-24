use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    NoConfirm,
    Notify,
    ExplicitApproval,
}

#[derive(Debug, Clone)]
pub struct GuardrailCheck {
    pub tool: String,
    pub permission: Permission,
    pub reason: String,
    pub requires_approval: bool,
    pub approved: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_ordering() {
        assert_ne!(Permission::NoConfirm, Permission::ExplicitApproval);
    }

    #[test]
    fn test_guardrail_check_default() {
        let check = GuardrailCheck {
            tool: "get_events".into(),
            permission: Permission::NoConfirm,
            reason: "Read-only operation".into(),
            requires_approval: false,
            approved: None,
        };
        assert!(!check.requires_approval);
        assert!(check.approved.is_none());
    }

    #[test]
    fn test_permission_serialization() {
        let json = serde_json::to_value(&Permission::Notify).unwrap();
        assert_eq!(json, serde_json::json!("Notify"));
    }
}
