//! 消息处理器模块

pub mod start;
pub mod admin;
pub mod visitor;
pub mod text;
pub mod callback;
pub mod member;

// 重新导出处理器函数
pub use start::*;
pub use admin::*;
pub use visitor::*;
pub use text::*;
pub use callback::*;
pub use member::*;