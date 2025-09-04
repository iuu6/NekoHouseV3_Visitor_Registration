//! 密码生成模块
//! 
//! 提供多种密码生成算法，包括临时密码、次数密码、时效密码等

use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;

/// 密码类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum PasswordType {
    /// 临时密码（10分钟有效）
    Temp,
    /// 次数密码（指定次数）
    Times(i32),
    /// 时效密码（指定时长）
    Limited(Duration),
    /// 期间密码（指定过期时间）
    Period(DateTime<Utc>),
    /// 长期临时密码（可重复获取直到过期）
    LongtimeTemp(DateTime<Utc>),
}

/// 密码生成结果
#[derive(Debug, Clone)]
pub struct PasswordResult {
    /// 生成的密码
    pub password: String,
    /// 密码类型
    pub password_type: PasswordType,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 过期时间
    pub expires_at: Option<DateTime<Utc>>,
    /// 剩余次数（仅次数密码有效）
    pub remaining_times: Option<i32>,
}

/// 统一密码生成器
pub struct UnifiedPasswordGenerator {
    /// 存储已生成的密码及其状态
    passwords: HashMap<String, PasswordResult>,
}

impl UnifiedPasswordGenerator {
    /// 创建新的密码生成器实例
    pub fn new() -> Self {
        Self {
            passwords: HashMap::new(),
        }
    }

    /// 生成密码
    pub fn generate_password(
        &mut self,
        admin_password: &str,
        password_type: PasswordType,
    ) -> PasswordResult {
        let password = self.generate_base_password(admin_password, &password_type);
        let now = Utc::now();
        
        let (expires_at, remaining_times) = match &password_type {
            PasswordType::Temp => (Some(now + Duration::minutes(10)), None),
            PasswordType::Times(times) => (Some(now + Duration::hours(2)), Some(*times)),
            PasswordType::Limited(duration) => (Some(now + *duration), None),
            PasswordType::Period(end_time) => (Some(*end_time), None),
            PasswordType::LongtimeTemp(end_time) => (Some(*end_time), None),
        };

        let result = PasswordResult {
            password: password.clone(),
            password_type,
            created_at: now,
            expires_at,
            remaining_times,
        };

        self.passwords.insert(password, result.clone());
        result
    }

    /// 验证密码
    pub fn verify_password(&mut self, password: &str) -> Option<&mut PasswordResult> {
        if let Some(result) = self.passwords.get_mut(password) {
            // 检查是否过期
            if let Some(expires_at) = result.expires_at {
                if Utc::now() > expires_at {
                    return None;
                }
            }

            // 处理次数密码
            if let PasswordType::Times(_) = result.password_type {
                if let Some(remaining) = result.remaining_times.as_mut() {
                    if *remaining <= 0 {
                        return None;
                    }
                    *remaining -= 1;
                }
            }

            Some(result)
        } else {
            None
        }
    }

    /// 获取密码剩余时间
    pub fn get_password_remaining_time(&self, password: &str) -> Option<Duration> {
        if let Some(result) = self.passwords.get(password) {
            if let Some(expires_at) = result.expires_at {
                let now = Utc::now();
                if now < expires_at {
                    Some(expires_at - now)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 生成基础密码（简化版KeeLoq算法）
    fn generate_base_password(&self, admin_password: &str, password_type: &PasswordType) -> String {
        // 简化的密码生成逻辑
        // 在实际应用中，这里应该实现真正的KeeLoq算法
        let mut hash = 0u32;
        
        // 将管理员密码转换为数字
        for byte in admin_password.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
        }

        // 根据密码类型调整hash
        match password_type {
            PasswordType::Temp => hash = hash.wrapping_mul(17),
            PasswordType::Times(times) => hash = hash.wrapping_add(*times as u32),
            PasswordType::Limited(duration) => {
                hash = hash.wrapping_add(duration.num_minutes() as u32)
            }
            PasswordType::Period(end_time) => {
                hash = hash.wrapping_add(end_time.timestamp() as u32)
            }
            PasswordType::LongtimeTemp(end_time) => {
                hash = hash.wrapping_add((end_time.timestamp() / 3600) as u32)
            }
        }

        // 添加当前时间的影响
        let now = Utc::now();
        hash = hash.wrapping_add((now.timestamp() / 60) as u32);

        // 生成6位数密码
        format!("{:06}", hash % 1000000)
    }
}

impl Default for UnifiedPasswordGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局函数接口，用于向后兼容

/// 生成密码
pub fn generate_password(admin_password: &str, password_type: PasswordType) -> PasswordResult {
    let mut generator = UnifiedPasswordGenerator::new();
    generator.generate_password(admin_password, password_type)
}

/// 验证密码（注意：这是无状态版本，实际应用中应该使用有状态的版本）
pub fn verify_password(password: &str, admin_password: &str) -> bool {
    // 简化版验证逻辑
    // 实际应用中需要维护密码状态
    !password.is_empty() && !admin_password.is_empty()
}

/// 获取密码剩余时间
pub fn get_password_remaining_time(password: &str, admin_password: &str) -> Option<Duration> {
    // 简化版实现
    // 实际应用中需要从存储中查询密码信息
    if verify_password(password, admin_password) {
        Some(Duration::minutes(10)) // 默认返回10分钟
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_generation() {
        let mut generator = UnifiedPasswordGenerator::new();
        let result = generator.generate_password("admin123", PasswordType::Temp);
        
        assert!(!result.password.is_empty());
        assert_eq!(result.password.len(), 6);
        assert!(result.expires_at.is_some());
    }

    #[test]
    fn test_times_password() {
        let mut generator = UnifiedPasswordGenerator::new();
        let result = generator.generate_password("admin123", PasswordType::Times(3));
        
        assert_eq!(result.remaining_times, Some(3));
        
        // 验证密码
        let password = result.password.clone();
        assert!(generator.verify_password(&password).is_some());
        assert!(generator.verify_password(&password).is_some());
        assert!(generator.verify_password(&password).is_some());
        assert!(generator.verify_password(&password).is_none()); // 第4次应该失败
    }

    #[test]
    fn test_limited_password() {
        let mut generator = UnifiedPasswordGenerator::new();
        let duration = Duration::hours(1);
        let result = generator.generate_password("admin123", PasswordType::Limited(duration));
        
        assert!(result.expires_at.is_some());
        let expires_at = result.expires_at.unwrap();
        let expected_time = Utc::now() + Duration::hours(1);
        assert!((expires_at - expected_time).num_seconds().abs() < 60); // 1分钟误差范围
    }
}