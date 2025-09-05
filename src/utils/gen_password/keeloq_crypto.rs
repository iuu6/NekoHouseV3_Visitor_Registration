//! KeeLoq 加密算法 Rust 实现
//! 用于生成基于时间和管理密码的安全密码

use chrono::{DateTime, Utc, FixedOffset};

/// KeeLoq 加密算法结构体
pub struct KeeLoqCrypto {
    /// S-Box 查找表，用于非线性函数计算
    sbox: [[[[[u8; 2]; 2]; 2]; 2]; 2],
    /// 密钥生成常量 N
    key_n: [u8; 4],
    /// 密钥生成常量 T  
    key_t: [u8; 4],
}

impl KeeLoqCrypto {
    /// 创建新的 KeeLoq 加密实例
    pub fn new() -> Self {
        let mut crypto = KeeLoqCrypto {
            sbox: [[[[[0; 2]; 2]; 2]; 2]; 2],
            key_n: [133, 103, 37, 67],
            key_t: [68, 84, 25, 55],
        };
        
        // 初始化 S-Box
        crypto.init_sbox();
        crypto
    }

    /// 初始化 S-Box 查找表
    fn init_sbox(&mut self) {
        // S-Box 值基于原始 JavaScript 实现
        self.sbox[0][0][0][0][0] = 0; self.sbox[0][0][0][0][1] = 1;
        self.sbox[0][0][0][1][0] = 1; self.sbox[0][0][0][1][1] = 1;
        self.sbox[0][0][1][0][0] = 0; self.sbox[0][0][1][0][1] = 1;
        self.sbox[0][0][1][1][0] = 0; self.sbox[0][0][1][1][1] = 0;
        self.sbox[0][1][0][0][0] = 0; self.sbox[0][1][0][0][1] = 0;
        self.sbox[0][1][0][1][0] = 1; self.sbox[0][1][0][1][1] = 0;
        self.sbox[0][1][1][0][0] = 1; self.sbox[0][1][1][0][1] = 1;
        self.sbox[0][1][1][1][0] = 1; self.sbox[0][1][1][1][1] = 0;
        self.sbox[1][0][0][0][0] = 0; self.sbox[1][0][0][0][1] = 0;
        self.sbox[1][0][0][1][0] = 1; self.sbox[1][0][0][1][1] = 1;
        self.sbox[1][0][1][0][0] = 1; self.sbox[1][0][1][0][1] = 0;
        self.sbox[1][0][1][1][0] = 1; self.sbox[1][0][1][1][1] = 0;
        self.sbox[1][1][0][0][0] = 0; self.sbox[1][1][0][0][1] = 1;
        self.sbox[1][1][0][1][0] = 0; self.sbox[1][1][0][1][1] = 1;
        self.sbox[1][1][1][0][0] = 1; self.sbox[1][1][1][0][1] = 1;
        self.sbox[1][1][1][1][0] = 0; self.sbox[1][1][1][1][1] = 0;
    }

    /// 获取指定位的值
    fn get_bit(&self, value: u32, bit: u8) -> u8 {
        if (value & (1 << bit)) != 0 { 1 } else { 0 }
    }

    /// 右移位操作，支持循环位
    fn shift_right(&self, value: u32, carry: u8) -> u32 {
        if carry != 0 {
            (value >> 1) | 0x80000000
        } else {
            (value >> 1) & 0x7FFFFFFF
        }
    }

    /// 字节数组转换为32位整数
    fn bytes_to_u32(&self, bytes: &[u8]) -> u32 {
        let mut result = 0u32;
        for (i, &byte) in bytes.iter().enumerate() {
            result += ((byte as u32) & 0xFF) << (8 * i);
        }
        result
    }

    /// 32位整数转换为字节数组并返回整数
    fn u32_to_bytes_and_back(&self, value: u32) -> u32 {
        let bytes = [
            ((value >> 24) & 0xFF) as u8,
            ((value >> 16) & 0xFF) as u8,
            ((value >> 8) & 0xFF) as u8,
            (value & 0xFF) as u8,
        ];
        self.bytes_to_u32(&bytes)
    }

    /// KeeLoq 核心加密函数
    fn keeloq_encrypt(&self, data: u32, key1: u32, key2: u32) -> u32 {
        let mut n = data;
        
        // 528 轮加密
        for round in 0..528 {
            let bit_pos = round % 64;
            
            // 非线性函数计算
            let b31 = self.get_bit(n, 31);
            let b26 = self.get_bit(n, 26);
            let b20 = self.get_bit(n, 20);
            let b9 = self.get_bit(n, 9);
            let b1 = self.get_bit(n, 1);
            
            let sbox_output = self.sbox[b31 as usize][b26 as usize][b20 as usize][b9 as usize][b1 as usize];
            
            let b16 = self.get_bit(n, 16);
            let b0 = self.get_bit(n, 0);
            
            let key_bit = if bit_pos < 32 {
                self.get_bit(key1, bit_pos as u8)
            } else {
                self.get_bit(key2, (bit_pos - 32) as u8)
            };
            
            let feedback = sbox_output ^ b16 ^ b0 ^ key_bit;
            
            n = self.shift_right(n, feedback);
        }
        
        n
    }

    /// 主要加密用户代码函数
    pub fn crypt_usercode(&self, timestamp: u32, admin_pwd: &str) -> String {
        // 处理管理密码，确保长度为8位
        let mut pwd = admin_pwd.to_string();
        if pwd.len() >= 8 {
            pwd = pwd[..8].to_string();
        } else {
            while pwd.len() < 8 {
                pwd.push('0');
            }
        }

        // 将密码转换为数字数组
        let mut pwd_digits = Vec::new();
        for ch in pwd.chars() {
            pwd_digits.push(ch.to_digit(10).unwrap_or(0) as u8);
        }

        // 生成密钥
        let mut key_s = Vec::new();
        let mut key_g = Vec::new();
        
        for i in 0..self.key_n.len() {
            key_s.push(self.key_n[i] ^ pwd_digits[i]);
        }
        
        for i in 0..self.key_t.len() {
            key_g.push(self.key_t[i] ^ pwd_digits[i + 4]);
        }

        let key1 = self.bytes_to_u32(&key_s);
        let key2 = self.bytes_to_u32(&key_g);
        
        // 转换时间戳
        let converted_timestamp = self.u32_to_bytes_and_back(timestamp);
        
        // 执行加密
        let encrypted = self.keeloq_encrypt(converted_timestamp, key1, key2);
        let result = self.u32_to_bytes_and_back(encrypted);
        
        // 处理负数（JavaScript 兼容性）
        let final_result = if (result as i32) < 0 {
            (result as u32).wrapping_add(0)
        } else {
            result
        };

        // 格式化为10位字符串
        format!("{:010}", final_result)
    }

    /// 获取UTC+8时间戳（毫秒）
    pub fn get_utc8_timestamp() -> i64 {
        let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
        let beijing_time: DateTime<FixedOffset> = Utc::now().with_timezone(&beijing_tz);
        beijing_time.timestamp_millis()
    }

    /// 获取带时间偏移的UTC+8时间戳（毫秒）
    /// time_offset_seconds: 时间偏移秒数，用于密码锁防重放攻击
    pub fn get_utc8_timestamp_with_offset(time_offset_seconds: i32) -> i64 {
        let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
        let beijing_time: DateTime<FixedOffset> = Utc::now().with_timezone(&beijing_tz);
        beijing_time.timestamp_millis() + (time_offset_seconds as i64 * 1000)
    }

    /// 获取UTC+8时间戳（秒）
    pub fn get_utc8_timestamp_sec() -> i64 {
        Self::get_utc8_timestamp() / 1000
    }

    /// 获取带时间偏移的UTC+8时间戳（秒）
    pub fn get_utc8_timestamp_sec_with_offset(time_offset_seconds: i32) -> i64 {
        Self::get_utc8_timestamp_with_offset(time_offset_seconds) / 1000
    }

    /// 格式化UTC+8时间为字符串
    pub fn format_utc8_time(timestamp_ms: i64) -> String {
        let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
        let datetime = DateTime::from_timestamp_millis(timestamp_ms)
            .unwrap()
            .with_timezone(&beijing_tz);
        
        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
    }
}

impl Default for KeeLoqCrypto {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keeloq_crypto() {
        let crypto = KeeLoqCrypto::new();
        let result = crypto.crypt_usercode(1234567890, "12345678");
        println!("KeeLoq 加密结果: {}", result);
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_utc8_time() {
        let timestamp = KeeLoqCrypto::get_utc8_timestamp();
        let formatted = KeeLoqCrypto::format_utc8_time(timestamp);
        println!("UTC+8 时间: {}", formatted);
    }
}