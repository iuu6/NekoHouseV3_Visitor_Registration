//! 密码生成模块 - 重新导出utils/gen_password的功能
//!
//! 提供多种基于KeeLoq算法的密码生成功能

// 重新导出utils/gen_password的所有功能
pub use crate::utils::gen_password::*;

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