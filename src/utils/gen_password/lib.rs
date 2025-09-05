//! 密码生成算法库
//!
//! 这个库实现了四种不同的密码生成算法，都基于KeeLoq加密算法：
//! 1. 临时密码 - 基于4秒时间窗口，有效期10分钟
//! 2. 次数密码 - 可使用指定次数，有效期20小时
//! 3. 限时密码 - 指定时长有效
//! 4. 周期密码 - 指定时间段有效
//!
//! 所有时间处理都使用UTC+8时区（北京时间）

// 核心模块
pub mod keeloq_crypto;
pub mod temp_password;
pub mod times_password;
pub mod limited_password;
pub mod period_password;

// 工具模块
pub mod ui_utils;
pub mod demo_framework;
pub mod config;

// 重新导出核心功能
pub use keeloq_crypto::KeeLoqCrypto;

// 密码生成器
pub use temp_password::{TempPasswordGenerator, TempPasswordGeneratorWithOffset};
pub use times_password::{TimesPasswordGenerator, TimesPasswordGeneratorWithOffset};
pub use limited_password::{LimitedPasswordGenerator, LimitedPasswordGeneratorWithOffset};
pub use period_password::{PeriodPasswordGenerator, PeriodPasswordGeneratorWithOffset};

// 便捷函数
pub use temp_password::{generate_temp_password, verify_temp_password};
pub use times_password::{generate_times_password, verify_times_password, check_password_remaining_time};
pub use limited_password::{generate_limited_password, verify_limited_password, check_limited_password_remaining_time};
pub use period_password::{generate_period_password, generate_period_password_from_string, verify_period_password, check_period_password_remaining_time};

// 工具功能
pub use ui_utils::{InputHandler, MenuDisplay, Formatter, ErrorHandler};
pub use demo_framework::DemoFramework;
pub use config::{AppConfig, MenuOption, VerificationResult, TimeFormatter, ValidatorConfig, InputValidator};

/// 密码类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum PasswordType {
    /// 临时密码
    Temporary,
    /// 次数密码
    Times(u32),
    /// 限时密码
    Limited(u32, u32), // hours, minutes
    /// 周期密码
    Period(u32, u32, u32, u32), // year, month, day, hour
}

impl std::fmt::Display for PasswordType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasswordType::Temporary => write!(f, "临时密码"),
            PasswordType::Times(times) => write!(f, "次数密码({}次)", times),
            PasswordType::Limited(hours, 0) => write!(f, "限时密码({}小时)", hours),
            PasswordType::Limited(hours, minutes) => write!(f, "限时密码({}小时{}分钟)", hours, minutes),
            PasswordType::Period(year, month, day, hour) => {
                write!(f, "周期密码(至{:04}-{:02}-{:02} {:02}:00)", year, month, day, hour)
            },
        }
    }
}

/// 密码生成结果
#[derive(Debug, Clone)]
pub struct PasswordResult {
    /// 生成的密码
    pub password: String,
    /// 过期时间
    pub expire_time: String,
    /// 描述消息
    pub message: String,
    /// 密码类型
    pub password_type: PasswordType,
}

/// 统一的密码生成器
pub struct UnifiedPasswordGenerator {
    temp_gen: TempPasswordGenerator,
    times_gen: TimesPasswordGenerator,
    limited_gen: LimitedPasswordGenerator,
    period_gen: PeriodPasswordGenerator,
}

impl UnifiedPasswordGenerator {
    /// 创建新的统一密码生成器
    pub fn new() -> Self {
        UnifiedPasswordGenerator {
            temp_gen: TempPasswordGenerator::new(),
            times_gen: TimesPasswordGenerator::new(),
            limited_gen: LimitedPasswordGenerator::new(),
            period_gen: PeriodPasswordGenerator::new(),
        }
    }

    /// 根据密码类型生成密码
    pub fn generate(&self, admin_pwd: &str, password_type: PasswordType) -> Result<PasswordResult, String> {
        match password_type {
            PasswordType::Temporary => {
                let (password, expire_time, message) = self.temp_gen.generate(admin_pwd)?;
                Ok(PasswordResult {
                    password,
                    expire_time,
                    message,
                    password_type: PasswordType::Temporary,
                })
            }
            PasswordType::Times(count) => {
                let (password, expire_time, message) = self.times_gen.generate(admin_pwd, count)?;
                Ok(PasswordResult {
                    password,
                    expire_time,
                    message,
                    password_type: PasswordType::Times(count),
                })
            }
            PasswordType::Limited(hours, minutes) => {
                let (password, expire_time, message) = self.limited_gen.generate(admin_pwd, hours, minutes)?;
                Ok(PasswordResult {
                    password,
                    expire_time,
                    message,
                    password_type: PasswordType::Limited(hours, minutes),
                })
            }
            PasswordType::Period(year, month, day, hour) => {
                let (password, expire_time, message) = self.period_gen.generate(admin_pwd, year, month, day, hour)?;
                Ok(PasswordResult {
                    password,
                    expire_time,
                    message,
                    password_type: PasswordType::Period(year, month, day, hour),
                })
            }
        }
    }

    /// 验证密码（自动识别类型）
    pub fn verify(&self, password: &str, admin_pwd: &str) -> Option<PasswordType> {
        // 尝试验证临时密码 - 使用更大的容错窗口
        if self.temp_gen.verify(password, admin_pwd, 150) {
            return Some(PasswordType::Temporary);
        }

        // 尝试验证次数密码
        if let Some(times) = self.times_gen.verify(password, admin_pwd, 0, 2) {
            return Some(PasswordType::Times(times));
        }

        // 尝试验证限时密码
        if let Some((hours, minutes)) = self.limited_gen.verify(password, admin_pwd, 2) {
            return Some(PasswordType::Limited(hours, minutes));
        }

        // 尝试验证周期密码
        if let Some(_expire_time) = self.period_gen.verify(password, admin_pwd, 1) {
            // 周期密码无法从密码本身推断原始参数，返回通用标识
            return Some(PasswordType::Period(0, 0, 0, 0));
        }

        None
    }

    /// 获取密码剩余有效时间
    pub fn get_remaining_time(&self, password: &str, admin_pwd: &str) -> Option<String> {
        // 尝试不同类型的剩余时间检查
        if let Some(remaining) = self.temp_gen.check_remaining_time(password, admin_pwd) {
            let remaining_minutes = remaining / (1000 * 60);
            return Some(format!("{}分钟", remaining_minutes));
        }

        if let Some((remaining_str, _times)) = check_password_remaining_time(password, admin_pwd) {
            return Some(remaining_str);
        }

        if let Some((remaining_str, _hours, _minutes)) = check_limited_password_remaining_time(password, admin_pwd) {
            return Some(remaining_str);
        }

        if let Some((remaining_str, _expire_time)) = check_period_password_remaining_time(password, admin_pwd) {
            return Some(remaining_str);
        }

        None
    }
}

impl Default for UnifiedPasswordGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// 便捷函数：创建统一生成器并生成密码
pub fn generate_password(admin_pwd: &str, password_type: PasswordType) -> Result<PasswordResult, String> {
    let generator = UnifiedPasswordGenerator::new();
    generator.generate(admin_pwd, password_type)
}

/// 便捷函数：验证任意类型的密码
pub fn verify_password(password: &str, admin_pwd: &str) -> Option<PasswordType> {
    let generator = UnifiedPasswordGenerator::new();
    generator.verify(password, admin_pwd)
}

/// 便捷函数：获取任意密码的剩余时间
pub fn get_password_remaining_time(password: &str, admin_pwd: &str) -> Option<String> {
    let generator = UnifiedPasswordGenerator::new();
    generator.get_remaining_time(password, admin_pwd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_generator() {
        let generator = UnifiedPasswordGenerator::new();
        let admin_pwd = "123456";

        // 测试临时密码
        let temp_result = generator.generate(admin_pwd, PasswordType::Temporary).unwrap();
        println!("临时密码: {}", temp_result.password);
        assert!(matches!(temp_result.password_type, PasswordType::Temporary));

        // 测试次数密码
        let times_result = generator.generate(admin_pwd, PasswordType::Times(5)).unwrap();
        println!("次数密码: {}", times_result.password);
        assert!(matches!(times_result.password_type, PasswordType::Times(5)));

        // 测试限时密码
        let limited_result = generator.generate(admin_pwd, PasswordType::Limited(2, 30)).unwrap();
        println!("限时密码: {}", limited_result.password);
        assert!(matches!(limited_result.password_type, PasswordType::Limited(2, 30)));
    }

    #[test]
    fn test_password_verification() {
        let generator = UnifiedPasswordGenerator::new();
        let admin_pwd = "123456";

        // 生成临时密码并验证
        let temp_result = generator.generate(admin_pwd, PasswordType::Temporary).unwrap();
        let verified_type = generator.verify(&temp_result.password, admin_pwd);
        assert!(verified_type.is_some());
        println!("验证临时密码成功: {:?}", verified_type);
    }

    #[test]
    fn test_convenience_functions() {
        let admin_pwd = "123456";

        // 测试便捷函数
        let result = generate_password(admin_pwd, PasswordType::Times(3)).unwrap();
        println!("便捷函数生成的密码: {}", result.password);

        let verified = verify_password(&result.password, admin_pwd);
        println!("便捷函数验证结果: {:?}", verified);

        let remaining = get_password_remaining_time(&result.password, admin_pwd);
        println!("便捷函数剩余时间: {:?}", remaining);
    }
}