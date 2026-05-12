use serde::{Deserialize, Serialize};

use crate::view_model::context::ViewContext;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationStatus {
    Applied,
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationResultView {
    pub context: ViewContext,
    pub operation: String,
    pub target: String,
    pub status: MutationStatus,
    pub message: String,
    pub redirect_to: Option<String>,
    pub warnings: Vec<String>,
}

impl MutationResultView {
    pub fn applied(
        context: ViewContext,
        operation: impl Into<String>,
        target: impl Into<String>,
        message: impl Into<String>,
        redirect_to: Option<String>,
    ) -> Self {
        Self {
            context,
            operation: operation.into(),
            target: target.into(),
            status: MutationStatus::Applied,
            message: message.into(),
            redirect_to,
            warnings: Vec::new(),
        }
    }

    pub fn noop(
        context: ViewContext,
        operation: impl Into<String>,
        target: impl Into<String>,
        message: impl Into<String>,
        redirect_to: Option<String>,
    ) -> Self {
        Self {
            context,
            operation: operation.into(),
            target: target.into(),
            status: MutationStatus::Noop,
            message: message.into(),
            redirect_to,
            warnings: Vec::new(),
        }
    }
}
