//! 次数限制密码生成算法 Rust 实现
//! 生成可使用指定次数的密码，有效期20小时

use crate::keeloq_crypto::KeeLoqCrypto;

/// 次数限制密码生成器
pub struct TimesPasswordGenerator {
    crypto: KeeLoqCrypto,
}

impl TimesPasswordGenerator {
    /// 创建新的次数限制密码生成器实例
    pub fn new() -> Self {
        TimesPasswordGenerator {
            crypto: KeeLoqCrypto::new(),
        }
    }

    /// 生成次数限制密码
    /// 
    /// # 参数
    /// * `admin_pwd` - 管理员密码（至少4位）
    /// * `use_times` - 可使用次数（1-31次）
    /// 
    /// # 返回值
    /// * `Ok((password, expire_time, message))` - 成功时返回密码、过期时间和消息
    /// * `Err(String)` - 失败时返回错误信息
    pub fn generate(&self, admin_pwd: &str, use_times: u32) -> Result<(String, String, String), String> {
        // 检查管理员密码长度
        if admin_pwd.len() < 4 {
            return Err("管理员密码至少需要4位".to_string());
        }

        // 检查使用次数范围
        if use_times < 1 || use_times > 31 {
            return Err("使用次数必须在1-31之间".to_string());
        }

        // 获取UTC+8当前时间戳（毫秒）
        let current_time_ms = KeeLoqCrypto::get_utc8_timestamp();
        
        // 转换为4秒时间窗口，并进行位运算处理
        let mut time_window = (current_time_ms / 4000) as u32;
        time_window &= 0xFFFFFFE0; // 相当于 JavaScript 中的 &= 4294967264
        
        // 计算时间戳
        let timestamp = (time_window as u64) * 4000;
        
        // 计算过期时间（当前时间窗口 + 20小时 = 72000秒 = 72000000毫秒）
        let expire_time_ms = timestamp as i64 + 72000000;
        
        // 构造加密输入：时间窗口 + 使用次数 + 特殊常量
        let mut crypto_input = time_window;
        crypto_input += use_times;
        crypto_input += 1073741824; // 0x40000000
        
        // 使用KeeLoq加密算法生成密码
        let encrypted_code = self.crypto.crypt_usercode(crypto_input, admin_pwd);
        
        // 添加前缀 (对应 m = 5e9 + parseInt(m))
        let password_num = 5000000000u64 + encrypted_code.parse::<u64>().unwrap_or(0);
        let password = password_num.to_string();
        
        // 格式化过期时间
        let expire_time_str = KeeLoqCrypto::format_utc8_time(expire_time_ms);
        
        // 生成消息
        let message = format!("次数限制密码，可使用{}次，有效期至 {}", use_times, expire_time_str);
        
        Ok((password, expire_time_str, message))
    }

    /// 验证次数限制密码是否有效
    /// 
    /// # 参数
    /// * `password` - 要验证的密码
    /// * `admin_pwd` - 管理员密码
    /// * `expected_times` - 预期的使用次数
    /// * `tolerance_windows` - 容忍的时间窗口数量
    /// 
    /// # 返回值
    /// * `Some(times)` - 密码有效，返回实际的使用次数
    /// * `None` - 密码无效
    pub fn verify(&self, password: &str, admin_pwd: &str, _expected_times: u32, tolerance_windows: u32) -> Option<u32> {
        if admin_pwd.len() < 4 {
            return None;
        }

        // 移除前缀，获取实际的加密代码 (对应 5e9 前缀)
        let password_num = match password.parse::<u64>() {
            Ok(num) if num >= 5000000000 => num - 5000000000,
            _ => return None,
        };

        let current_time_ms = KeeLoqCrypto::get_utc8_timestamp();
        let current_window = (current_time_ms / 4000) as u32;
        let aligned_window = current_window & 0xFFFFFFE0;

        // 在容忍范围内检查时间窗口
        for window_offset in 0..=tolerance_windows {
            let check_window = if window_offset == 0 {
                aligned_window
            } else {
                aligned_window.wrapping_sub(window_offset * 32) // 每个对齐窗口是32个基本窗口
            };

            // 检查不同的使用次数
            for times in 1..=31 {
                let mut crypto_input = check_window;
                crypto_input += times;
                crypto_input += 1073741824;
                
                let expected_code = self.crypto.crypt_usercode(crypto_input, admin_pwd);
                
                if password_num == expected_code.parse::<u64>().unwrap_or(0) {
                    // 检查是否在有效期内（20小时）
                    let password_expire_time = (check_window as i64) * 4000 + 72000000;
                    if current_time_ms <= password_expire_time {
                        return Some(times);
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
    /// * `Some((remaining_ms, times))` - 密码有效，返回剩余毫秒数和使用次数
    /// * `None` - 密码无效或已过期
    pub fn check_remaining_time(&self, password: &str, admin_pwd: &str) -> Option<(i64, u32)> {
        if let Some(times) = self.verify(password, admin_pwd, 0, 5) {
            let current_time_ms = KeeLoqCrypto::get_utc8_timestamp();
            let current_window = (current_time_ms / 4000) as u32;
            let aligned_window = current_window & 0xFFFFFFE0;
            
            // 计算密码的过期时间
            let expire_time_ms = (aligned_window as i64) * 4000 + 72000000;
            let remaining_ms = expire_time_ms - current_time_ms;
            
            if remaining_ms > 0 {
                return Some((remaining_ms, times));
            }
        }
        None
    }

    /// 获取当前时间窗口信息
    pub fn get_current_window_info() -> (u32, u32, String, String) {
        let current_time_ms = KeeLoqCrypto::get_utc8_timestamp();
        let current_window = (current_time_ms / 4000) as u32;
        let aligned_window = current_window & 0xFFFFFFE0;
        
        let window_start_ms = (aligned_window as i64) * 4000;
        let window_expire_ms = window_start_ms + 72000000;
        
        let start_time = KeeLoqCrypto::format_utc8_time(window_start_ms);
        let expire_time = KeeLoqCrypto::format_utc8_time(window_expire_ms);
        
        (current_window, aligned_window, start_time, expire_time)
    }
}

impl Default for TimesPasswordGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// 便捷函数：生成次数限制密码
pub fn generate_times_password(admin_pwd: &str, use_times: u32) -> Result<(String, String, String), String> {
    let generator = TimesPasswordGenerator::new();
    generator.generate(admin_pwd, use_times)
}

/// 便捷函数：验证次数限制密码
pub fn verify_times_password(password: &str, admin_pwd: &str) -> Option<u32> {
    let generator = TimesPasswordGenerator::new();
    generator.verify(password, admin_pwd, 0, 2) // 默认容忍2个时间窗口
}

/// 便捷函数：检查密码剩余时间
pub fn check_password_remaining_time(password: &str, admin_pwd: &str) -> Option<(String, u32)> {
    let generator = TimesPasswordGenerator::new();
    if let Some((remaining_ms, times)) = generator.check_remaining_time(password, admin_pwd) {
        let remaining_hours = remaining_ms / (1000 * 60 * 60);
        let remaining_minutes = (remaining_ms % (1000 * 60 * 60)) / (1000 * 60);
        let remaining_str = format!("{}小时{}分钟", remaining_hours, remaining_minutes);
        Some((remaining_str, times))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_times_password() {
        let generator = TimesPasswordGenerator::new();
        
        // 测试正常生成
        match generator.generate("123456", 5) {
            Ok((password, expire_time, message)) => {
                println!("次数限制密码: {}", password);
                println!("过期时间: {}", expire_time);
                println!("消息: {}", message);
                assert!(password.len() >= 10);
                assert!(message.contains("5次"));
            }
            Err(e) => panic!("生成次数限制密码失败: {}", e),
        }
    }

    #[test]
    fn test_invalid_parameters() {
        let generator = TimesPasswordGenerator::new();
        
        // 测试无效的管理员密码
        assert!(generator.generate("123", 5).is_err());
        
        // 测试无效的使用次数
        assert!(generator.generate("123456", 0).is_err());
        assert!(generator.generate("123456", 32).is_err());
    }

    #[test]
    fn test_password_verification() {
        let generator = TimesPasswordGenerator::new();
        let admin_pwd = "123456";
        let use_times = 3;
        
        // 生成密码
        if let Ok((password, _, _)) = generator.generate(admin_pwd, use_times) {
            // 验证密码应该成功并返回正确的次数
            if let Some(verified_times) = generator.verify(&password, admin_pwd, use_times, 2) {
                println!("验证成功，使用次数: {}", verified_times);
                assert_eq!(verified_times, use_times);
            } else {
                panic!("密码验证失败");
            }
            
            // 错误的管理员密码应该验证失败
            assert!(generator.verify(&password, "654321", use_times, 2).is_none());
        }
    }

    #[test]
    fn test_remaining_time() {
        let generator = TimesPasswordGenerator::new();
        let admin_pwd = "123456";
        
        if let Ok((password, _, _)) = generator.generate(admin_pwd, 10) {
            if let Some((remaining_ms, times)) = generator.check_remaining_time(&password, admin_pwd) {
                println!("剩余时间: {}毫秒, 使用次数: {}", remaining_ms, times);
                assert!(remaining_ms > 0);
                assert_eq!(times, 10);
            }
        }
    }

    #[test]
    fn test_window_info() {
        let (current, aligned, start, expire) = TimesPasswordGenerator::get_current_window_info();
        println!("当前窗口: {}, 对齐窗口: {}", current, aligned);
        println!("开始时间: {}", start);
        println!("过期时间: {}", expire);
    }

    #[test]
    fn test_convenience_functions() {
        // 测试便捷函数
        match generate_times_password("123456", 7) {
            Ok((password, _, _)) => {
                println!("便捷函数生成的次数限制密码: {}", password);
                if let Some(times) = verify_times_password(&password, "123456") {
                    assert_eq!(times, 7);
                    println!("便捷函数验证成功，次数: {}", times);
                }
                
                if let Some((remaining, times)) = check_password_remaining_time(&password, "123456") {
                    println!("剩余时间: {}, 使用次数: {}", remaining, times);
                }
            }
            Err(e) => panic!("便捷函数失败: {}", e),
        }
    }
}