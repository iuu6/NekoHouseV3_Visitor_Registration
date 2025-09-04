//! 工具模块

// 重新导出gen_password的主要功能
pub use crate::gen_password::{
    UnifiedPasswordGenerator,
    PasswordType,
    PasswordResult,
    generate_password,
    verify_password,
    get_password_remaining_time,
};