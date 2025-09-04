//! 临时密码生成算法 Rust 实现
//! 基于4秒时间窗口生成临时密码，有效期10分钟

use super::keeloq_crypto::KeeLoqCrypto;

/// 临时密码生成器
pub struct TempPasswordGenerator {
    crypto: KeeLoqCrypto,
}

impl TempPasswordGenerator {
    /// 创建新的临时密码生成器实例
    pub fn new() -> Self {
        TempPasswordGenerator {
            crypto: KeeLoqCrypto::new(),
        }
    }

    /// 生成临时密码
    /// 
    /// # 参数
    /// * `admin_pwd` - 管理员密码（至少4位）
    /// 
    /// # 返回值
    /// * `Ok((password, expire_time, message))` - 成功时返回密码、过期时间和消息
    /// * `Err(String)` - 失败时返回错误信息
    pub fn generate(&self, admin_pwd: &str) -> Result<(String, String, String), String> {
        // 检查管理员密码长度
        if admin_pwd.len() < 4 {
            return Err("管理员密码至少需要4位".to_string());
        }

        // 获取UTC+8当前时间戳（毫秒）
        let current_time_ms = KeeLoqCrypto::get_utc8_timestamp();
        
        // 转换为4秒时间窗口 (对应 t = Math.floor(t / 4e3))
        let time_window = (current_time_ms / 4000) as u32;
        
        // 计算过期时间 (对应 l = 4e3 * t + 6e5)
        let expire_time_ms = (time_window as i64) * 4000 + 600000;
        
        // 使用KeeLoq加密算法生成密码 (对应 o = a.CRYPT_USERCODE(t, s))
        let encrypted_code = self.crypto.crypt_usercode(time_window, admin_pwd);
        
        // 添加前缀 (对应 o = 5e9 + parseInt(o))
        let password_num = 5000000000u64 + encrypted_code.parse::<u64>().unwrap_or(0);
        let password = password_num.to_string();
        
        // 格式化过期时间
        let expire_time_str = KeeLoqCrypto::format_utc8_time(expire_time_ms);
        
        // 生成消息
        let message = format!("临时密码有效期至 {}", expire_time_str);
        
        Ok((password, expire_time_str, message))
    }

    /// 验证临时密码是否有效
    /// 
    /// # 参数
    /// * `password` - 要验证的密码
    /// * `admin_pwd` - 管理员密码
    /// * `tolerance_windows` - 容忍的时间窗口数量（默认为1，即允许前后4秒的误差）
    /// 
    /// # 返回值
    /// * `true` - 密码有效
    /// * `false` - 密码无效
    pub fn verify(&self, password: &str, admin_pwd: &str, tolerance_windows: u32) -> bool {
        if admin_pwd.len() < 4 {
            return false;
        }

        // 移除前缀，获取实际的加密代码 (对应 5e9 前缀)
        let password_num = match password.parse::<u64>() {
            Ok(num) if num >= 5000000000 => num - 5000000000,
            _ => return false,
        };

        let current_time_ms = KeeLoqCrypto::get_utc8_timestamp();
        let current_window = (current_time_ms / 4000) as u32;

        // 在容忍范围内检查时间窗口
        for offset in 0..=tolerance_windows {
            // 检查当前和之前的时间窗口
            for &direction in &[0i32, -1i32] {
                let check_window = if direction < 0 && offset > 0 {
                    current_window.wrapping_sub(offset)
                } else if direction == 0 {
                    current_window
                } else {
                    continue;
                };

                // 直接使用时间窗口数值，不乘以4000 (对应原JS中的 t)
                let expected_code = self.crypto.crypt_usercode(check_window, admin_pwd);
                
                if password_num == expected_code.parse::<u64>().unwrap_or(0) {
                    return true;
                }
            }
        }

        false
    }

    /// 检查密码剩余有效时间
    ///
    /// # 参数
    /// * `password` - 要检查的密码
    /// * `admin_pwd` - 管理员密码
    ///
    /// # 返回值
    /// * `Some(remaining_ms)` - 密码有效，返回剩余毫秒数
    /// * `None` - 密码无效或已过期
    pub fn check_remaining_time(&self, password: &str, admin_pwd: &str) -> Option<i64> {
        if !self.verify(password, admin_pwd, 1) {
            return None;
        }

        let current_time_ms = KeeLoqCrypto::get_utc8_timestamp();
        let time_window = (current_time_ms / 4000) as u32;
        let expire_time_ms = (time_window as i64) * 4000 + 600000; // 10分钟有效期
        
        let remaining_ms = expire_time_ms - current_time_ms;
        if remaining_ms > 0 {
            Some(remaining_ms)
        } else {
            None
        }
    }

    /// 获取当前时间窗口信息
    pub fn get_current_window_info() -> (u32, String, String) {
        let current_time_ms = KeeLoqCrypto::get_utc8_timestamp();
        let current_window = (current_time_ms / 4000) as u32;
        let window_start_ms = (current_window as i64) * 4000;
        let window_end_ms = window_start_ms + 4000;
        
        let start_time = KeeLoqCrypto::format_utc8_time(window_start_ms);
        let end_time = KeeLoqCrypto::format_utc8_time(window_end_ms);
        
        (current_window, start_time, end_time)
    }
}

impl Default for TempPasswordGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// 便捷函数：生成临时密码
pub fn generate_temp_password(admin_pwd: &str) -> Result<(String, String, String), String> {
    let generator = TempPasswordGenerator::new();
    generator.generate(admin_pwd)
}

/// 便捷函数：验证临时密码
pub fn verify_temp_password(password: &str, admin_pwd: &str) -> bool {
    let generator = TempPasswordGenerator::new();
    generator.verify(password, admin_pwd, 1) // 默认容忍1个时间窗口
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_temp_password() {
        let generator = TempPasswordGenerator::new();
        
        // 测试正常生成
        match generator.generate("123456") {
            Ok((password, expire_time, message)) => {
                println!("临时密码: {}", password);
                println!("过期时间: {}", expire_time);
                println!("消息: {}", message);
                assert!(password.len() >= 10);
            }
            Err(e) => panic!("生成临时密码失败: {}", e),
        }
    }

    #[test]
    fn test_invalid_admin_password() {
        let generator = TempPasswordGenerator::new();
        
        // 测试无效的管理员密码
        match generator.generate("123") {
            Ok(_) => panic!("应该返回错误"),
            Err(e) => {
                println!("预期的错误: {}", e);
                assert!(e.contains("至少需要4位"));
            }
        }
    }

    #[test]
    fn test_password_verification() {
        let generator = TempPasswordGenerator::new();
        let admin_pwd = "123456";
        
        // 生成密码
        if let Ok((password, _, _)) = generator.generate(admin_pwd) {
            // 验证密码应该成功
            assert!(generator.verify(&password, admin_pwd, 1));
            
            // 错误的管理员密码应该验证失败
            assert!(!generator.verify(&password, "654321", 1));
            
            // 错误的密码格式应该验证失败
            assert!(!generator.verify("invalid", admin_pwd, 1));
        }
    }

    #[test]
    fn test_window_info() {
        let (window, start, end) = TempPasswordGenerator::get_current_window_info();
        println!("当前时间窗口: {}", window);
        println!("窗口开始时间: {}", start);
        println!("窗口结束时间: {}", end);
    }

    #[test]
    fn test_convenience_functions() {
        // 测试便捷函数
        match generate_temp_password("123456") {
            Ok((password, _, _)) => {
                println!("便捷函数生成的临时密码: {}", password);
                assert!(verify_temp_password(&password, "123456"));
            }
            Err(e) => panic!("便捷函数失败: {}", e),
        }
    }
}