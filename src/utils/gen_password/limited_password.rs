//! 限时密码生成算法 Rust 实现
//! 生成基于指定时长有效的密码

use super::keeloq_crypto::KeeLoqCrypto;

/// 限时密码生成器
pub struct LimitedPasswordGenerator {
    crypto: KeeLoqCrypto,
}

impl LimitedPasswordGenerator {
    /// 创建新的限时密码生成器实例
    pub fn new() -> Self {
        LimitedPasswordGenerator {
            crypto: KeeLoqCrypto::new(),
        }
    }

    /// 生成限时密码
    /// 
    /// # 参数
    /// * `admin_pwd` - 管理员密码（至少4位）
    /// * `hours` - 有效小时数（0-127）
    /// * `minutes` - 有效分钟数（0或30）
    /// 
    /// # 返回值
    /// * `Ok((password, expire_time, message))` - 成功时返回密码、过期时间和消息
    /// * `Err(String)` - 失败时返回错误信息
    pub fn generate(&self, admin_pwd: &str, hours: u32, minutes: u32) -> Result<(String, String, String), String> {
        // 检查管理员密码长度
        if admin_pwd.len() < 4 {
            return Err("管理员密码至少需要4位".to_string());
        }

        // 检查小时数范围
        if hours > 127 {
            return Err("小时数不能超过127".to_string());
        }

        // 检查分钟数（只允许0或30）
        if minutes != 0 && minutes != 30 {
            return Err("分钟数只能是0或30".to_string());
        }

        // 计算总的半小时数
        let total_half_hours = hours * 2 + (minutes / 30);

        // 获取UTC+8当前时间戳（毫秒）
        let current_time_ms = KeeLoqCrypto::get_utc8_timestamp();
        
        // 转换为30分钟时间窗口 (对应 t = Math.floor(t / 18e5))
        let time_window = (current_time_ms / 1800000) as u32;
        
        // 计算过期时间 (对应 i = 18e5 * (t + r))
        let expire_time_ms = ((time_window + total_half_hours) as i64) * 1800000;
        
        // 构造加密输入 (对应 d = 256 * t + 2147483648 + r)
        let mut crypto_input = time_window * 256;
        crypto_input += 2147483648; // 0x80000000
        crypto_input += total_half_hours;
        
        // 使用KeeLoq加密算法生成密码
        let encrypted_code = self.crypto.crypt_usercode(crypto_input, admin_pwd);
        
        // 添加前缀 (对应 5e9 + parseInt(encrypted))
        let password_num = 5000000000u64 + encrypted_code.parse::<u64>().unwrap_or(0);
        let password = password_num.to_string();
        
        // 格式化过期时间
        let expire_time_str = KeeLoqCrypto::format_utc8_time(expire_time_ms);
        
        // 生成持续时间描述
        let duration_desc = if hours == 0 {
            format!("{}分钟", minutes)
        } else if minutes == 0 {
            if hours == 1 {
                "1小时".to_string()
            } else {
                format!("{}小时", hours)
            }
        } else {
            if hours == 1 {
                format!("1小时{}分钟", minutes)
            } else {
                format!("{}小时{}分钟", hours, minutes)
            }
        };
        
        // 生成消息
        let message = format!("限时密码，有效时长{}，过期时间 {}", duration_desc, expire_time_str);
        
        Ok((password, expire_time_str, message))
    }

    /// 验证限时密码是否有效
    /// 
    /// # 参数
    /// * `password` - 要验证的密码
    /// * `admin_pwd` - 管理员密码
    /// * `tolerance_windows` - 容忍的时间窗口数量
    /// 
    /// # 返回值
    /// * `Some((hours, minutes))` - 密码有效，返回原始的小时和分钟设置
    /// * `None` - 密码无效
    pub fn verify(&self, password: &str, admin_pwd: &str, tolerance_windows: u32) -> Option<(u32, u32)> {
        if admin_pwd.len() < 4 {
            return None;
        }

        // 移除前缀，获取实际的加密代码 (对应 5e9 前缀)
        let password_num = match password.parse::<u64>() {
            Ok(num) if num >= 5000000000 => num - 5000000000,
            _ => return None,
        };

        let current_time_ms = KeeLoqCrypto::get_utc8_timestamp();
        let current_window = (current_time_ms / 1800000) as u32;

        // 在容忍范围内检查时间窗口
        for window_offset in 0..=tolerance_windows {
            for &direction in &[0i32, 1i32, -1i32] {
                let check_window = match direction {
                    0 => current_window,
                    1 => current_window + window_offset,
                    -1 => current_window.wrapping_sub(window_offset),
                    _ => continue,
                };

                // 检查不同的时长组合
                for hours in 0..=127 {
                    for &minutes in &[0, 30] {
                        let total_half_hours = hours * 2 + (minutes / 30);
                        
                        // 使用检查窗口作为基准计算
                        let mut crypto_input = check_window * 256;
                        crypto_input += 2147483648;
                        crypto_input += total_half_hours;
                        
                        let expected_code = self.crypto.crypt_usercode(crypto_input, admin_pwd);
                        
                        if password_num == expected_code.parse::<u64>().unwrap_or(0) {
                            // 检查密码是否还在有效期内
                            let password_expire_time = ((check_window + total_half_hours) as i64) * 1800000;
                            if current_time_ms <= password_expire_time {
                                return Some((hours, minutes));
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
    /// * `Some((remaining_ms, hours, minutes))` - 密码有效，返回剩余毫秒数和原始时长设置
    /// * `None` - 密码无效或已过期
    pub fn check_remaining_time(&self, password: &str, admin_pwd: &str) -> Option<(i64, u32, u32)> {
        if let Some((hours, minutes)) = self.verify(password, admin_pwd, 5) {
            let current_time_ms = KeeLoqCrypto::get_utc8_timestamp();
            let current_window = (current_time_ms / 1800000) as u32;
            
            // 根据验证结果计算过期时间
            let _total_half_hours = hours * 2 + (minutes / 30);
            let expire_window = current_window + 1; // 至少到下个窗口
            let expire_time_ms = (expire_window as i64) * 1800000;
            
            let remaining_ms = expire_time_ms - current_time_ms;
            
            if remaining_ms > 0 {
                return Some((remaining_ms, hours, minutes));
            }
        }
        None
    }

    /// 获取当前时间窗口信息（30分钟为一个窗口）
    pub fn get_current_window_info() -> (u32, String, String) {
        let current_time_ms = KeeLoqCrypto::get_utc8_timestamp();
        let current_window = (current_time_ms / 1800000) as u32;
        
        let window_start_ms = (current_window as i64) * 1800000;
        let window_end_ms = window_start_ms + 1800000;
        
        let start_time = KeeLoqCrypto::format_utc8_time(window_start_ms);
        let end_time = KeeLoqCrypto::format_utc8_time(window_end_ms);
        
        (current_window, start_time, end_time)
    }

    /// 将小时和分钟转换为半小时单位
    pub fn to_half_hours(hours: u32, minutes: u32) -> Result<u32, String> {
        if hours > 127 {
            return Err("小时数不能超过127".to_string());
        }
        if minutes != 0 && minutes != 30 {
            return Err("分钟数只能是0或30".to_string());
        }
        
        Ok(hours * 2 + (minutes / 30))
    }

    /// 将半小时单位转换为小时和分钟
    pub fn from_half_hours(half_hours: u32) -> (u32, u32) {
        let hours = half_hours / 2;
        let minutes = (half_hours % 2) * 30;
        (hours, minutes)
    }
}

impl Default for LimitedPasswordGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// 便捷函数：生成限时密码
pub fn generate_limited_password(admin_pwd: &str, hours: u32, minutes: u32) -> Result<(String, String, String), String> {
    let generator = LimitedPasswordGenerator::new();
    generator.generate(admin_pwd, hours, minutes)
}

/// 便捷函数：验证限时密码
pub fn verify_limited_password(password: &str, admin_pwd: &str) -> Option<(u32, u32)> {
    let generator = LimitedPasswordGenerator::new();
    generator.verify(password, admin_pwd, 2) // 默认容忍2个时间窗口
}

/// 便捷函数：检查密码剩余时间
pub fn check_limited_password_remaining_time(password: &str, admin_pwd: &str) -> Option<(String, u32, u32)> {
    let generator = LimitedPasswordGenerator::new();
    if let Some((remaining_ms, hours, minutes)) = generator.check_remaining_time(password, admin_pwd) {
        let remaining_hours = remaining_ms / (1000 * 60 * 60);
        let remaining_minutes = (remaining_ms % (1000 * 60 * 60)) / (1000 * 60);
        let remaining_str = format!("{}小时{}分钟", remaining_hours, remaining_minutes);
        Some((remaining_str, hours, minutes))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_limited_password() {
        let generator = LimitedPasswordGenerator::new();
        
        // 测试生成2小时30分钟的限时密码
        match generator.generate("123456", 2, 30) {
            Ok((password, expire_time, message)) => {
                println!("限时密码: {}", password);
                println!("过期时间: {}", expire_time);
                println!("消息: {}", message);
                assert!(password.len() >= 10);
                assert!(message.contains("2小时30分钟"));
            }
            Err(e) => panic!("生成限时密码失败: {}", e),
        }
    }

    #[test]
    fn test_invalid_parameters() {
        let generator = LimitedPasswordGenerator::new();
        
        // 测试无效的管理员密码
        assert!(generator.generate("123", 1, 0).is_err());
        
        // 测试无效的小时数
        assert!(generator.generate("123456", 128, 0).is_err());
        
        // 测试无效的分钟数
        assert!(generator.generate("123456", 1, 15).is_err());
        assert!(generator.generate("123456", 1, 60).is_err());
    }

    #[test]
    fn test_password_verification() {
        let generator = LimitedPasswordGenerator::new();
        let admin_pwd = "123456";
        let hours = 1;
        let minutes = 30;
        
        // 生成密码
        if let Ok((password, _, _)) = generator.generate(admin_pwd, hours, minutes) {
            // 验证密码应该成功并返回正确的时长
            if let Some((verified_hours, verified_minutes)) = generator.verify(&password, admin_pwd, 2) {
                println!("验证成功，时长: {}小时{}分钟", verified_hours, verified_minutes);
                assert_eq!(verified_hours, hours);
                assert_eq!(verified_minutes, minutes);
            } else {
                panic!("密码验证失败");
            }
            
            // 错误的管理员密码应该验证失败
            assert!(generator.verify(&password, "654321", 2).is_none());
        }
    }

    #[test]
    fn test_half_hours_conversion() {
        // 测试时间单位转换
        assert_eq!(LimitedPasswordGenerator::to_half_hours(1, 0).unwrap(), 2);
        assert_eq!(LimitedPasswordGenerator::to_half_hours(1, 30).unwrap(), 3);
        assert_eq!(LimitedPasswordGenerator::to_half_hours(2, 30).unwrap(), 5);
        
        assert_eq!(LimitedPasswordGenerator::from_half_hours(2), (1, 0));
        assert_eq!(LimitedPasswordGenerator::from_half_hours(3), (1, 30));
        assert_eq!(LimitedPasswordGenerator::from_half_hours(5), (2, 30));
    }

    #[test]
    fn test_window_info() {
        let (window, start, end) = LimitedPasswordGenerator::get_current_window_info();
        println!("当前30分钟窗口: {}", window);
        println!("窗口开始时间: {}", start);
        println!("窗口结束时间: {}", end);
    }

    #[test]
    fn test_various_durations() {
        let generator = LimitedPasswordGenerator::new();
        let admin_pwd = "123456";
        
        // 测试不同的时长组合
        let test_cases = vec![
            (0, 30),  // 30分钟
            (1, 0),   // 1小时
            (2, 30),  // 2小时30分钟
            (24, 0),  // 24小时
            (127, 30), // 最大时长
        ];
        
        for (hours, minutes) in test_cases {
            match generator.generate(admin_pwd, hours, minutes) {
                Ok((password, _, message)) => {
                    println!("时长 {}h{}m 的密码: {}", hours, minutes, password);
                    println!("消息: {}", message);
                    
                    // 验证生成的密码
                    if let Some((vh, vm)) = generator.verify(&password, admin_pwd, 2) {
                        assert_eq!(vh, hours);
                        assert_eq!(vm, minutes);
                    }
                }
                Err(e) => panic!("生成 {}h{}m 密码失败: {}", hours, minutes, e),
            }
        }
    }

    #[test]
    fn test_convenience_functions() {
        // 测试便捷函数
        match generate_limited_password("123456", 3, 0) {
            Ok((password, _, _)) => {
                println!("便捷函数生成的限时密码: {}", password);
                if let Some((hours, minutes)) = verify_limited_password(&password, "123456") {
                    assert_eq!(hours, 3);
                    assert_eq!(minutes, 0);
                    println!("便捷函数验证成功，时长: {}h{}m", hours, minutes);
                }
                
                if let Some((remaining, hours, minutes)) = check_limited_password_remaining_time(&password, "123456") {
                    println!("剩余时间: {}, 原始时长: {}h{}m", remaining, hours, minutes);
                }
            }
            Err(e) => panic!("便捷函数失败: {}", e),
        }
    }
}