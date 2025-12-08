use crate::models::send::SendType;
use serde::{Deserialize, Serialize};

/// Request model for creating/editing Send
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendRequest {
    #[serde(rename = "type")]
    pub send_type: SendType,

    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<SendTextRequest>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<SendFileRequest>,

    pub key: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_access_count: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<String>,

    pub deletion_date: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    pub disabled: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_email: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendTextRequest {
    pub text: String,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendFileRequest {
    pub file_name: String,
    pub size: u64,
    pub size_name: String,
}
