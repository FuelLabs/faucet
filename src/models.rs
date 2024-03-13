use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DispenseInfoResponse {
    pub amount: u64,
    pub asset_id: String,
}

#[derive(Deserialize, Debug)]
pub struct DispenseInput {
    pub salt: Option<String>,
    pub nonce: Option<String>,
    pub address: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct DispenseResponse {
    pub status: String,
    pub tokens: u64,
}

#[derive(Debug)]
pub struct DispenseError {
    pub status: StatusCode,
    pub error: String,
}

#[derive(Deserialize, Debug)]
pub struct CreateSessionInput {
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateSessionResponse {
    pub status: String,
    pub salt: String,
    pub difficulty: u8,
}

#[derive(Debug)]
pub struct CreateSessionError {
    pub status: StatusCode,
    pub error: String,
}
