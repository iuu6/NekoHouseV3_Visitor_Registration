//! 密码服务模块 - 集成密码生成算法

use crate::config::AppConfig;
use crate::error::{AppError, Result};
use crate::types::{AuthType, PasswordRequest};
use crate::utils::gen_password::{
    UnifiedPasswordGenerator, PasswordType, PasswordResult,
    generate_password, verify_password, get_password_remaining_time,
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// 密码服务
pub struct PasswordService {
    generator: UnifiedPasswordGenerator,
    /// 用于跟踪长期临时密码的最后生成时间
    longtime_temp_cache: HashMap<i64, DateTime<Utc>>,
}

impl PasswordService {
    /// 创建新的密码服务实例
    pub fn new() -> Self {
        Self {
            generator: UnifiedPasswordGenerator::new(),
            longtime_temp_cache: HashMap::new(),
        }
    }

    /// 生成密码
    pub fn generate_password(
        &mut self,
        request: &PasswordRequest,
        config: &AppConfig,
    ) -> Result<PasswordResult> {
        // 获取调整后的管理员密码（考虑时间偏移）
        let adjusted_admin_pwd = self.get_adjusted_admin_password(&request.admin_password, config);
        
        match request.auth_type {
            AuthType::Temp => self.generate_temp_password(&adjusted_admin_pwd),
            AuthType::Times => self.generate_times_password(&adjusted_admin_pwd, request.times),
            AuthType::Limited => self.generate_limited_password(&adjusted_admin_pwd, request.hours, request.minutes),
            AuthType::Period => self.generate_period_password(&adjusted_admin_pwd, request),
            AuthType::LongtimeTemp => self.generate_longtime_temp_password(&adjusted_admin_pwd, request),
        }
    }

    /// 验证密码
    pub fn verify_password(
        &self,
        password: &str,
        admin_password: &str,
        config: &AppConfig,
    ) -> Result<bool> {
        let adjusted_admin_pwd = self.get_adjusted_admin_password(admin_password, config);
        Ok(verify_password(password, &adjusted_admin_pwd).is_some())
    }

    /// 获取密码剩余有效时间
    pub fn get_remaining_time(
        &self,
        password: &str,
        admin_password: &str,
        config: &AppConfig,
    ) -> Result<Option<String>> {
        let adjusted_admin_pwd = self.get_adjusted_admin_password(admin_password, config);
        Ok(crate::utils::gen_password::get_password_remaining_time(password, &adjusted_admin_pwd))
    }

    /// 检查长期临时密码是否可以生成（5分钟限制）
    pub fn can_generate_longtime_temp(&mut self, user_id: i64) -> bool {
        if let Some(last_generated) = self.longtime_temp_cache.get(&user_id) {
            let now = Utc::now();
            let elapsed = now.signed_duration_since(*last_generated);
            elapsed.num_minutes() >= 5
        } else {
            true // 第一次生成
        }
    }

    /// 标记长期临时密码已生成
    pub fn mark_longtime_temp_generated(&mut self, user_id: i64) {
        self.longtime_temp_cache.insert(user_id, Utc::now());
    }

    /// 清理过期的长期临时密码缓存
    pub fn cleanup_longtime_temp_cache(&mut self) {
        let now = Utc::now();
        self.longtime_temp_cache.retain(|_, last_time| {
            let elapsed = now.signed_duration_since(*last_time);
            elapsed.num_minutes() < 60 // 保留1小时内的记录
        });
    }

    /// 生成临时密码（10分钟有效）
    fn generate_temp_password(&self, admin_pwd: &str) -> Result<PasswordResult> {
        let result = generate_password(admin_pwd, PasswordType::Temporary)
            .map_err(|e| AppError::password_generation(e))?;
        Ok(result)
    }

    /// 生成次数限制密码（2小时有效）
    fn generate_times_password(&self, admin_pwd: &str, times: Option<u32>) -> Result<PasswordResult> {
        let use_times = times.ok_or_else(|| AppError::validation("次数密码必须指定使用次数"))?;
        
        if use_times < 1 || use_times > 31 {
            return Err(AppError::validation("使用次数必须在1-31之间"));
        }

        let result = generate_password(admin_pwd, PasswordType::Times(use_times))
            .map_err(|e| AppError::password_generation(e))?;
        Ok(result)
    }

    /// 生成限时密码
    fn generate_limited_password(&self, admin_pwd: &str, hours: Option<u32>, minutes: Option<u32>) -> Result<PasswordResult> {
        let hours = hours.ok_or_else(|| AppError::validation("限时密码必须指定小时数"))?;
        let minutes = minutes.unwrap_or(0);

        if hours > 127 {
            return Err(AppError::validation("小时数不能超过127"));
        }

        if minutes != 0 && minutes != 30 {
            return Err(AppError::validation("分钟数只能是0或30"));
        }

        let result = generate_password(admin_pwd, PasswordType::Limited(hours, minutes))
            .map_err(|e| AppError::password_generation(e))?;
        Ok(result)
    }

    /// 生成周期密码（指定过期时间）
    fn generate_period_password(&self, admin_pwd: &str, request: &PasswordRequest) -> Result<PasswordResult> {
        let year = request.end_year.ok_or_else(|| AppError::validation("周期密码必须指定年份"))?;
        let month = request.end_month.ok_or_else(|| AppError::validation("周期密码必须指定月份"))?;
        let day = request.end_day.ok_or_else(|| AppError::validation("周期密码必须指定日期"))?;
        let hour = request.end_hour.ok_or_else(|| AppError::validation("周期密码必须指定小时"))?;

        // 验证时间参数
        if year < 2024 || year > 2099 {
            return Err(AppError::validation("年份必须在2024-2099之间"));
        }
        
        if month < 1 || month > 12 {
            return Err(AppError::validation("月份必须在1-12之间"));
        }
        
        if day < 1 || day > 31 {
            return Err(AppError::validation("日期必须在1-31之间"));
        }
        
        if hour > 23 {
            return Err(AppError::validation("小时必须在0-23之间"));
        }

        let result = generate_password(admin_pwd, PasswordType::Period(year, month, day, hour))
            .map_err(|e| AppError::password_generation(e))?;
        Ok(result)
    }

    /// 生成长期临时密码（使用临时密码算法）
    fn generate_longtime_temp_password(&self, admin_pwd: &str, _request: &PasswordRequest) -> Result<PasswordResult> {
        // 长期临时密码实际上就是一个临时密码，但有特殊的使用限制
        // 在5分钟内只能生成一次，超过5分钟可以重新申请
        let result = generate_password(admin_pwd, PasswordType::Temporary)
            .map_err(|e| AppError::password_generation(e))?;
        Ok(result)
    }

    /// 获取调整后的管理员密码（加入时间偏移）
    fn get_adjusted_admin_password(&self, admin_password: &str, config: &AppConfig) -> String {
        if config.time_offset == 0 {
            admin_password.to_string()
        } else {
            // 将时间偏移融入到密码中，增加安全性
            format!("{}{}", admin_password, config.time_offset.abs() % 10000)
        }
    }

    /// 根据授权类型获取默认过期时间
    pub fn get_default_expiry_time(&self, auth_type: AuthType) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        match auth_type {
            AuthType::Temp => Some(now + chrono::Duration::minutes(10)),
            AuthType::Times => Some(now + chrono::Duration::hours(2)),
            AuthType::Limited => None, // 需要指定
            AuthType::Period => None,  // 需要指定
            AuthType::LongtimeTemp => None, // 管理员指定
        }
    }

    /// 验证授权请求参数
    pub fn validate_request(&self, request: &PasswordRequest) -> Result<()> {
        match request.auth_type {
            AuthType::Times => {
                if request.times.is_none() {
                    return Err(AppError::validation("次数密码必须指定使用次数"));
                }
                let times = request.times.unwrap();
                if times < 1 || times > 31 {
                    return Err(AppError::validation("使用次数必须在1-31之间"));
                }
            }
            AuthType::Limited => {
                if request.hours.is_none() {
                    return Err(AppError::validation("限时密码必须指定小时数"));
                }
                let hours = request.hours.unwrap();
                if hours > 127 {
                    return Err(AppError::validation("小时数不能超过127"));
                }
                if let Some(minutes) = request.minutes {
                    if minutes != 0 && minutes != 30 {
                        return Err(AppError::validation("分钟数只能是0或30"));
                    }
                }
            }
            AuthType::Period => {
                if request.end_year.is_none() || request.end_month.is_none() 
                    || request.end_day.is_none() || request.end_hour.is_none() {
                    return Err(AppError::validation("周期密码必须指定完整的结束时间"));
                }
            }
            AuthType::Temp | AuthType::LongtimeTemp => {
                // 临时密码不需要额外参数验证
            }
        }

        Ok(())
    }
}

impl Default for PasswordService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    #[test]
    fn test_password_generation() {
        let mut service = PasswordService::new();
        let config = AppConfig::default();

        // 测试临时密码
        let request = PasswordRequest {
            admin_password: "123456".to_string(),
            auth_type: AuthType::Temp,
            times: None,
            hours: None,
            minutes: None,
            end_year: None,
            end_month: None,
            end_day: None,
            end_hour: None,
            start_time: None,
        };

        let result = service.generate_password(&request, &config);
        assert!(result.is_ok());
        
        let password_result = result.unwrap();
        assert!(!password_result.password.is_empty());
        assert!(password_result.password.starts_with('5'));
    }

    #[test]
    fn test_request_validation() {
        let service = PasswordService::new();

        // 测试次数密码验证
        let mut request = PasswordRequest {
            admin_password: "123456".to_string(),
            auth_type: AuthType::Times,
            times: None,
            hours: None,
            minutes: None,
            end_year: None,
            end_month: None,
            end_day: None,
            end_hour: None,
            start_time: None,
        };

        // 缺少times参数应该失败
        assert!(service.validate_request(&request).is_err());

        // 设置有效的times参数应该成功
        request.times = Some(5);
        assert!(service.validate_request(&request).is_ok());

        // 无效的times值应该失败
        request.times = Some(50);
        assert!(service.validate_request(&request).is_err());
    }

    #[test]
    fn test_longtime_temp_cache() {
        let mut service = PasswordService::new();
        let user_id = 123456789;

        // 第一次应该可以生成
        assert!(service.can_generate_longtime_temp(user_id));

        // 标记已生成
        service.mark_longtime_temp_generated(user_id);

        // 立即再次请求应该被拒绝
        assert!(!service.can_generate_longtime_temp(user_id));
    }
}