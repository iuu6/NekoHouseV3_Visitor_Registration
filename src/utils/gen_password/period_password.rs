//! 周期密码生成算法 Rust 实现
//! 生成在指定时间段内有效的密码

use crate::keeloq_crypto::KeeLoqCrypto;
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

/// 周期密码生成器
pub struct PeriodPasswordGenerator {
    crypto: KeeLoqCrypto,
}

impl PeriodPasswordGenerator {
    /// 创建新的周期密码生成器实例
    pub fn new() -> Self {
        PeriodPasswordGenerator {
            crypto: KeeLoqCrypto::new(),
        }
    }

    /// 生成周期密码
    /// 
    /// # 参数
    /// * `admin_pwd` - 管理员密码（至少4位）
    /// * `end_year` - 结束年份
    /// * `end_month` - 结束月份（1-12）
    /// * `end_day` - 结束日期（1-31）
    /// * `end_hour` - 结束小时（0-23）
    /// 
    /// # 返回值
    /// * `Ok((password, expire_time, message))` - 成功时返回密码、过期时间和消息
    /// * `Err(String)` - 失败时返回错误信息
    pub fn generate(&self, admin_pwd: &str, end_year: u32, end_month: u32, end_day: u32, end_hour: u32) -> Result<(String, String, String), String> {
        // 检查管理员密码长度
        if admin_pwd.len() < 4 {
            return Err("管理员密码至少需要4位".to_string());
        }

        // 验证日期参数
        if end_month < 1 || end_month > 12 {
            return Err("月份必须在1-12之间".to_string());
        }
        if end_day < 1 || end_day > 31 {
            return Err("日期必须在1-31之间".to_string());
        }
        if end_hour > 23 {
            return Err("小时必须在0-23之间".to_string());
        }

        // 创建结束时间
        let end_date = match NaiveDate::from_ymd_opt(end_year as i32, end_month, end_day) {
            Some(date) => date,
            None => return Err("无效的日期".to_string()),
        };
        
        let end_time = NaiveTime::from_hms_opt(end_hour, 0, 0).unwrap();
        let end_datetime = NaiveDateTime::new(end_date, end_time);
        
        // 转换为UTC+8时区
        let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
        let end_datetime_tz = beijing_tz.from_local_datetime(&end_datetime).single()
            .ok_or("无法转换到UTC+8时区")?;

        // 获取当前UTC+8时间
        let current_time_ms = KeeLoqCrypto::get_utc8_timestamp();
        let current_datetime = DateTime::from_timestamp_millis(current_time_ms)
            .unwrap()
            .with_timezone(&beijing_tz);

        // 检查结束时间是否晚于当前时间
        if end_datetime_tz <= current_datetime {
            return Err("结束时间必须晚于当前时间".to_string());
        }

        // 计算时间戳（秒） (对应 l = Math.floor(r.getTime() / 1e3) + 28800)
        let current_timestamp_sec = current_time_ms / 1000 + 28800; // 加上UTC+8偏移
        let end_timestamp_sec = end_datetime_tz.timestamp();

        // 计算天数（从1970-01-01开始的天数） (对应 u = Math.floor(l / 86400))
        let current_days = (current_timestamp_sec / 86400) as u32;
        
        // 构造加密输入 (对应 m = 32768 * u + 3221225472)
        let mut crypto_input = current_days * 32768 + 3221225472; // 0xC0000000
        
        // 计算结束时间的小时数 (对应 p = Math.floor((h - 86400 * u) / 3600) + 8)
        let end_day_start_sec = current_days as i64 * 86400;
        let hours_from_day_start = ((end_timestamp_sec - end_day_start_sec) / 3600) + 8;
        
        // 检查小时数是否超出支持范围
        if hours_from_day_start > 32768 {
            return Err("结束时间超出支持范围".to_string());
        }
        
        crypto_input += hours_from_day_start as u32;
        
        // 使用KeeLoq加密算法生成密码
        let encrypted_code = self.crypto.crypt_usercode(crypto_input, admin_pwd);
        
        // 添加前缀 (对应 5e9 + parseInt(encrypted))
        let password_num = 5000000000u64 + encrypted_code.parse::<u64>().unwrap_or(0);
        let password = password_num.to_string();
        
        // 格式化过期时间
        let expire_time_str = end_datetime_tz.format("%Y-%m-%d %H:%M:%S").to_string();
        
        // 生成消息
        let message = format!("周期密码，有效期至 {}", expire_time_str);
        
        Ok((password, expire_time_str, message))
    }

    /// 使用便捷的日期时间字符串生成周期密码
    /// 
    /// # 参数
    /// * `admin_pwd` - 管理员密码
    /// * `end_datetime_str` - 结束时间字符串，格式："YYYY-MM-DD HH:MM:SS"
    pub fn generate_from_string(&self, admin_pwd: &str, end_datetime_str: &str) -> Result<(String, String, String), String> {
        // 解析日期时间字符串
        let parts: Vec<&str> = end_datetime_str.split_whitespace().collect();
        if parts.len() != 2 {
            return Err("日期时间格式错误，应为：YYYY-MM-DD HH:MM:SS".to_string());
        }

        let date_parts: Vec<&str> = parts[0].split('-').collect();
        let time_parts: Vec<&str> = parts[1].split(':').collect();
        
        if date_parts.len() != 3 || time_parts.len() != 3 {
            return Err("日期时间格式错误，应为：YYYY-MM-DD HH:MM:SS".to_string());
        }

        let year: u32 = date_parts[0].parse().map_err(|_| "无效的年份")?;
        let month: u32 = date_parts[1].parse().map_err(|_| "无效的月份")?;
        let day: u32 = date_parts[2].parse().map_err(|_| "无效的日期")?;
        let hour: u32 = time_parts[0].parse().map_err(|_| "无效的小时")?;

        self.generate(admin_pwd, year, month, day, hour)
    }

    /// 验证周期密码是否有效
    /// 
    /// # 参数
    /// * `password` - 要验证的密码
    /// * `admin_pwd` - 管理员密码
    /// * `tolerance_days` - 容忍的天数误差
    /// 
    /// # 返回值
    /// * `Some(expire_datetime)` - 密码有效，返回过期时间
    /// * `None` - 密码无效
    pub fn verify(&self, password: &str, admin_pwd: &str, tolerance_days: u32) -> Option<String> {
        if admin_pwd.len() < 4 {
            return None;
        }

        // 移除前缀，获取实际的加密代码 (对应 5e9 前缀)
        let password_num = match password.parse::<u64>() {
            Ok(num) if num >= 5000000000 => num - 5000000000,
            _ => return None,
        };

        let current_time_ms = KeeLoqCrypto::get_utc8_timestamp();
        let current_timestamp_sec = current_time_ms / 1000 + 28800;
        let current_days = (current_timestamp_sec / 86400) as u32;

        // 在容忍范围内检查天数
        for day_offset in 0..=tolerance_days {
            for &direction in &[0i32, -1i32] {
                let check_days = if direction < 0 && day_offset > 0 {
                    current_days.wrapping_sub(day_offset)
                } else if direction == 0 {
                    current_days
                } else {
                    continue;
                };

                // 检查不同的小时数 (对应原JS中的 p 值)
                for hours in 8..=32776 { // 从UTC+8开始检查
                    let mut crypto_input = check_days * 32768 + 3221225472;
                    crypto_input += hours;
                    
                    let expected_code = self.crypto.crypt_usercode(crypto_input, admin_pwd);
                    
                    if password_num == expected_code.parse::<u64>().unwrap_or(0) {
                        // 使用生成时的逆向计算来获得实际的过期时间
                        // hours 是 p 值，即 Math.floor((h - 86400 * u) / 3600) + 8 的结果
                        // 所以实际的结束时间秒数 h = (hours - 8) * 3600 + 86400 * check_days
                        let actual_end_timestamp = ((hours - 8) * 3600) as i64 + (check_days as i64) * 86400;
                        
                        // 检查是否还在有效期内
                        if (current_time_ms / 1000 + 28800) <= actual_end_timestamp {
                            let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
                            if let Some(expire_datetime) = DateTime::from_timestamp(actual_end_timestamp, 0) {
                                let expire_datetime_tz = expire_datetime.with_timezone(&beijing_tz);
                                return Some(expire_datetime_tz.format("%Y-%m-%d %H:%M:%S").to_string());
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// 检查密码剩余有效时间
    /// 
    /// # 参数
    /// * `password` - 要检查的密码
    /// * `admin_pwd` - 管理员密码
    /// 
    /// # 返回值
    /// * `Some((remaining_ms, expire_time))` - 密码有效，返回剩余毫秒数和过期时间
    /// * `None` - 密码无效或已过期
    pub fn check_remaining_time(&self, password: &str, admin_pwd: &str) -> Option<(i64, String)> {
        if let Some(expire_time_str) = self.verify(password, admin_pwd, 3) {
            // 解析过期时间
            let _beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
            if let Ok(expire_datetime) = DateTime::parse_from_str(&format!("{} +08:00", expire_time_str), "%Y-%m-%d %H:%M:%S %z") {
                let current_time_ms = KeeLoqCrypto::get_utc8_timestamp();
                let expire_time_ms = expire_datetime.timestamp_millis();
                let remaining_ms = expire_time_ms - current_time_ms;
                
                if remaining_ms > 0 {
                    return Some((remaining_ms, expire_time_str));
                }
            }
        }
        None
    }

    /// 获取给定日期的月份天数
    pub fn get_month_days(year: u32, month: u32) -> Result<u32, String> {
        if month < 1 || month > 12 {
            return Err("月份必须在1-12之间".to_string());
        }

        let days = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                // 判断闰年
                if (year % 400 == 0) || (year % 4 == 0 && year % 100 != 0) {
                    29
                } else {
                    28
                }
            }
            _ => unreachable!(),
        };

        Ok(days)
    }
}

impl Default for PeriodPasswordGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// 便捷函数：生成周期密码
pub fn generate_period_password(admin_pwd: &str, end_year: u32, end_month: u32, end_day: u32, end_hour: u32) -> Result<(String, String, String), String> {
    let generator = PeriodPasswordGenerator::new();
    generator.generate(admin_pwd, end_year, end_month, end_day, end_hour)
}

/// 便捷函数：从字符串生成周期密码
pub fn generate_period_password_from_string(admin_pwd: &str, end_datetime_str: &str) -> Result<(String, String, String), String> {
    let generator = PeriodPasswordGenerator::new();
    generator.generate_from_string(admin_pwd, end_datetime_str)
}

/// 便捷函数：验证周期密码
pub fn verify_period_password(password: &str, admin_pwd: &str) -> Option<String> {
    let generator = PeriodPasswordGenerator::new();
    generator.verify(password, admin_pwd, 1) // 默认容忍1天误差
}

/// 便捷函数：检查密码剩余时间
pub fn check_period_password_remaining_time(password: &str, admin_pwd: &str) -> Option<(String, String)> {
    let generator = PeriodPasswordGenerator::new();
    if let Some((remaining_ms, expire_time)) = generator.check_remaining_time(password, admin_pwd) {
        let remaining_days = remaining_ms / (1000 * 60 * 60 * 24);
        let remaining_hours = (remaining_ms % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60);
        let remaining_minutes = (remaining_ms % (1000 * 60 * 60)) / (1000 * 60);
        
        let remaining_str = if remaining_days > 0 {
            format!("{}天{}小时{}分钟", remaining_days, remaining_hours, remaining_minutes)
        } else if remaining_hours > 0 {
            format!("{}小时{}分钟", remaining_hours, remaining_minutes)
        } else {
            format!("{}分钟", remaining_minutes)
        };
        
        Some((remaining_str, expire_time))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Utc, Datelike, Timelike};

    #[test]
    fn test_generate_period_password() {
        let generator = PeriodPasswordGenerator::new();
        
        // 测试生成明天同一时间的周期密码
        let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
        let tomorrow = Utc::now().with_timezone(&beijing_tz).date_naive().succ_opt().unwrap();
        
        match generator.generate("123456", tomorrow.year() as u32, tomorrow.month(), tomorrow.day(), 18) {
            Ok((password, expire_time, message)) => {
                println!("周期密码: {}", password);
                println!("过期时间: {}", expire_time);
                println!("消息: {}", message);
                assert!(password.len() >= 10);
            }
            Err(e) => println!("生成周期密码失败（这可能是预期的）: {}", e),
        }
    }

    #[test]
    fn test_generate_from_string() {
        let generator = PeriodPasswordGenerator::new();
        
        // 测试从字符串生成
        match generator.generate_from_string("123456", "2024-12-31 23:59:59") {
            Ok((password, expire_time, message)) => {
                println!("从字符串生成的周期密码: {}", password);
                println!("过期时间: {}", expire_time);
                println!("消息: {}", message);
            }
            Err(e) => println!("从字符串生成失败: {}", e),
        }
    }

    #[test]
    fn test_invalid_parameters() {
        let generator = PeriodPasswordGenerator::new();
        
        // 测试无效的管理员密码
        assert!(generator.generate("123", 2024, 12, 31, 18).is_err());
        
        // 测试无效的日期参数
        assert!(generator.generate("123456", 2024, 13, 1, 18).is_err()); // 无效月份
        assert!(generator.generate("123456", 2024, 1, 32, 18).is_err()); // 无效日期
        assert!(generator.generate("123456", 2024, 1, 1, 24).is_err()); // 无效小时
    }

    #[test]
    fn test_month_days() {
        // 测试月份天数计算
        assert_eq!(PeriodPasswordGenerator::get_month_days(2024, 1).unwrap(), 31);
        assert_eq!(PeriodPasswordGenerator::get_month_days(2024, 2).unwrap(), 29); // 闰年
        assert_eq!(PeriodPasswordGenerator::get_month_days(2023, 2).unwrap(), 28); // 平年
        assert_eq!(PeriodPasswordGenerator::get_month_days(2024, 4).unwrap(), 30);
        
        // 测试无效月份
        assert!(PeriodPasswordGenerator::get_month_days(2024, 0).is_err());
        assert!(PeriodPasswordGenerator::get_month_days(2024, 13).is_err());
    }

    #[test]
    fn test_password_verification() {
        let generator = PeriodPasswordGenerator::new();
        let admin_pwd = "123456";
        
        // 尝试生成一个未来的密码进行验证测试
        let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
        let future_time = Utc::now().with_timezone(&beijing_tz) + chrono::Duration::hours(2);
        
        match generator.generate(admin_pwd, 
                                future_time.year() as u32, 
                                future_time.month(), 
                                future_time.day(), 
                                future_time.hour()) {
            Ok((password, _, _)) => {
                // 验证密码
                if let Some(expire_time) = generator.verify(&password, admin_pwd, 2) {
                    println!("验证成功，过期时间: {}", expire_time);
                } else {
                    println!("密码验证失败");
                }
                
                // 错误的管理员密码应该验证失败
                assert!(generator.verify(&password, "654321", 2).is_none());
            }
            Err(e) => println!("生成未来密码失败: {}", e),
        }
    }

    #[test]
    fn test_convenience_functions() {
        // 测试便捷函数
        match generate_period_password_from_string("123456", "2024-12-25 18:00:00") {
            Ok((password, expire_time, _message)) => {
                println!("便捷函数生成的周期密码: {}", password);
                println!("过期时间: {}", expire_time);
                
                if let Some(verified_expire_time) = verify_period_password(&password, "123456") {
                    println!("便捷函数验证成功，过期时间: {}", verified_expire_time);
                }
                
                if let Some((remaining, expire)) = check_period_password_remaining_time(&password, "123456") {
                    println!("剩余时间: {}, 过期时间: {}", remaining, expire);
                }
            }
            Err(e) => println!("便捷函数失败: {}", e),
        }
    }

    #[test]
    fn test_string_parsing() {
        let generator = PeriodPasswordGenerator::new();
        
        // 测试各种字符串格式
        let test_cases = vec![
            "2024-12-31 23:59:59",
            "2025-01-01 00:00:00",
            "2024-06-15 12:30:00",
        ];
        
        for datetime_str in test_cases {
            match generator.generate_from_string("123456", datetime_str) {
                Ok((password, expire_time, _)) => {
                    println!("时间 {} 的密码: {}", datetime_str, password);
                    println!("过期时间: {}", expire_time);
                }
                Err(e) => println!("解析时间 {} 失败: {}", datetime_str, e),
            }
        }
    }
}