//! Threshold-based approval gate.

use std::collections::HashSet;

use tokio::sync::Mutex;

use synwire_core::BoxFuture;
use synwire_core::sandbox::approval::{
    ApprovalCallback, ApprovalDecision, ApprovalRequest, RiskLevel,
};

/// Auto-approves operations up to a given risk level; delegates higher-risk
/// operations to an inner callback.
pub struct ThresholdGate {
    threshold: RiskLevel,
    inner: Box<dyn ApprovalCallback>,
    /// Operations that have been globally approved via `AllowAlways`.
    always_allowed: Mutex<HashSet<String>>,
}

impl std::fmt::Debug for ThresholdGate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThresholdGate")
            .field("threshold", &self.threshold)
            .finish_non_exhaustive()
    }
}

impl ThresholdGate {
    /// Create a new threshold gate.
    pub fn new(threshold: RiskLevel, inner: impl ApprovalCallback + 'static) -> Self {
        Self {
            threshold,
            inner: Box::new(inner),
            always_allowed: Mutex::new(HashSet::new()),
        }
    }

    async fn is_always_allowed(&self, operation: &str) -> bool {
        self.always_allowed.lock().await.contains(operation)
    }

    async fn record_always_allowed(&self, operation: &str) {
        let _ = self
            .always_allowed
            .lock()
            .await
            .insert(operation.to_string());
    }
}

impl ApprovalCallback for ThresholdGate {
    fn request(&self, req: ApprovalRequest) -> BoxFuture<'_, ApprovalDecision> {
        Box::pin(async move {
            // Check AllowAlways cache first.
            if self.is_always_allowed(&req.operation).await {
                return ApprovalDecision::Allow;
            }

            // Auto-approve if within threshold.
            if req.risk <= self.threshold {
                return ApprovalDecision::Allow;
            }

            // Delegate to inner callback for higher-risk operations.
            let operation = req.operation.clone();
            let decision = self.inner.request(req).await;

            if matches!(decision, ApprovalDecision::AllowAlways) {
                self.record_always_allowed(&operation).await;
                return ApprovalDecision::AllowAlways;
            }

            decision
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use synwire_core::sandbox::approval::AutoDenyCallback;

    fn req(risk: RiskLevel) -> ApprovalRequest {
        ApprovalRequest {
            operation: "test_op".to_string(),
            description: "test".to_string(),
            risk,
            timeout_secs: None,
            context: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn test_auto_approve_below_threshold() {
        let gate = ThresholdGate::new(RiskLevel::Medium, AutoDenyCallback);
        let decision = gate.request(req(RiskLevel::Low)).await;
        assert!(matches!(decision, ApprovalDecision::Allow));
    }

    #[tokio::test]
    async fn test_delegate_above_threshold() {
        let gate = ThresholdGate::new(RiskLevel::Low, AutoDenyCallback);
        let decision = gate.request(req(RiskLevel::High)).await;
        assert!(matches!(decision, ApprovalDecision::Deny));
    }

    #[tokio::test]
    async fn test_allow_always_caching() {
        struct AllowAlwaysCallback;
        impl ApprovalCallback for AllowAlwaysCallback {
            fn request(&self, _req: ApprovalRequest) -> BoxFuture<'_, ApprovalDecision> {
                Box::pin(async { ApprovalDecision::AllowAlways })
            }
        }

        let gate = ThresholdGate::new(RiskLevel::None, AllowAlwaysCallback);

        // First request: above threshold, delegates to inner → AllowAlways.
        let d1 = gate.request(req(RiskLevel::High)).await;
        assert!(matches!(d1, ApprovalDecision::AllowAlways));

        // Second request: should be served from cache without calling inner.
        let d2 = gate.request(req(RiskLevel::Critical)).await;
        assert!(matches!(d2, ApprovalDecision::Allow));
    }
}
