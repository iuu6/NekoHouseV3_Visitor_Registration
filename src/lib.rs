//! NekoHouse V3 访客登记系统
//!
//! 这是一个基于Telegram Bot的访客登记系统，支持：
//! - 超级管理员、管理员和访客三级权限管理
//! - 多种授权类型（临时、次数、时效、指定时间等）
//! - 基于KeeLoq算法的安全密码生成
//! - SQLite数据库存储

pub mod config;
pub mod database;
pub mod bot;
pub mod auth;
pub mod gen_password;
pub mod handlers;
pub mod types;
pub mod utils;
pub mod error;

// 重新导出常用类型
pub use config::{AppConfig, ConfigManager};
pub use database::Database;
pub use error::{Result, AppError};
pub use types::{UserRole, AuthStatus, AuthType, UserInfo, Admin, Record};
pub use gen_password::{UnifiedPasswordGenerator, PasswordType, PasswordResult};