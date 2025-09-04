//! 错误处理模块

use thiserror::Error;

/// 应用程序错误类型
#[derive(Error, Debug)]
pub enum AppError {
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),

    #[error("配置错误: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Telegram Bot错误: {0}")]
    TelegramBot(#[from] teloxide::RequestError),

    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("UUID错误: {0}")]
    Uuid(#[from] uuid::Error),

    #[error("密码生成错误: {0}")]
    PasswordGeneration(String),

    #[error("认证错误: {0}")]
    Authentication(String),

    #[error("权限不足: {0}")]
    Permission(String),

    #[error("验证错误: {0}")]
    Validation(String),

    #[error("业务逻辑错误: {0}")]
    Business(String),

    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("其他错误: {0}")]
    Other(String),
}

impl AppError {
    /// 创建业务逻辑错误
    pub fn business<T: Into<String>>(msg: T) -> Self {
        Self::Business(msg.into())
    }

    /// 创建验证错误
    pub fn validation<T: Into<String>>(msg: T) -> Self {
        Self::Validation(msg.into())
    }

    /// 创建认证错误
    pub fn authentication<T: Into<String>>(msg: T) -> Self {
        Self::Authentication(msg.into())
    }

    /// 创建权限错误
    pub fn permission<T: Into<String>>(msg: T) -> Self {
        Self::Permission(msg.into())
    }

    /// 创建密码生成错误
    pub fn password_generation<T: Into<String>>(msg: T) -> Self {
        Self::PasswordGeneration(msg.into())
    }
}

/// 应用程序Result类型
pub type Result<T> = std::result::Result<T, AppError>;