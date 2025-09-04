//! UI工具模块
//! 提供用户界面相关的通用功能

use std::io::{self, Write};

/// 用户输入处理
pub struct InputHandler;

impl InputHandler {
    /// 获取用户输入
    pub fn get_input(prompt: &str) -> String {
        print!("{}", prompt);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input.trim().to_string()
    }

    /// 获取数字输入（带验证）
    pub fn get_number_input<T>(prompt: &str, min: T, max: T, default: T) -> T 
    where 
        T: std::str::FromStr + std::cmp::PartialOrd + std::fmt::Display + Copy,
    {
        loop {
            let input = Self::get_input(prompt);
            match input.parse::<T>() {
                Ok(value) if value >= min && value <= max => return value,
                _ => {
                    println!("❌ 无效输入，使用默认值: {}", default);
                    return default;
                }
            }
        }
    }

    /// 获取管理员密码
    pub fn get_admin_password() -> String {
        loop {
            let pwd = Self::get_input("🔑 请输入管理员密码（至少4位）: ");
            if pwd.len() >= 4 {
                return pwd;
            }
            println!("❌ 管理员密码至少需要4位，请重试");
        }
    }
}

/// 菜单显示
pub struct MenuDisplay;

impl MenuDisplay {
    /// 显示主菜单
    pub fn show_main_menu() {
        println!("📋 选择功能:");
        println!("1. 🕐 生成临时密码 (4秒时间窗口，有效期10分钟)");
        println!("2. 🔢 生成次数密码 (指定使用次数，有效期20小时)");
        println!("3. ⏱️  生成限时密码 (指定时长有效)");
        println!("4. 📅 生成周期密码 (指定时间段有效)");
        println!("5. 🔍 验证密码");
        println!("6. 🚪 退出");
    }

    /// 显示分隔线
    pub fn show_separator(title: &str) {
        println!("\n{}", title);
        println!("{}", "-".repeat(30));
    }

    /// 显示标题
    pub fn show_title() {
        println!("🔐 密码生成算法演示程序");
        println!("📅 使用UTC+8时区（北京时间）");
        println!("{}", "=".repeat(50));
    }
}

/// 格式化输出
pub struct Formatter;

impl Formatter {
    /// 格式化标题
    pub fn format_title(title: &str) {
        println!("\n{}", title);
        println!("{}", "-".repeat(30));
    }

    /// 格式化密码生成结果
    pub fn format_generation_result(password: &str, expire_time: &str, message: &str) {
        println!("✅ 生成成功!");
        println!("🔐 密码: {}", password);
        println!("⏰ 过期时间: {}", expire_time);
        println!("📝 说明: {}", message);
    }

    /// 格式化验证成功结果
    pub fn format_verification_success(password_type: &str, additional_info: Option<&str>) {
        println!("✅ {}验证通过", password_type);
        if let Some(info) = additional_info {
            println!("    📊 {}", info);
        }
    }

    /// 格式化验证失败结果
    pub fn format_verification_failure(password_type: &str, description: &str) {
        println!("❌ {}验证失败", password_type);
        println!("    📝 说明: {}", description);
    }

    /// 格式化时间窗口信息
    pub fn format_window_info(title: &str, info: &str) {
        println!("    🕰️  {}: {}", title, info);
    }

    /// 格式化剩余时间
    pub fn format_remaining_time(remaining: &str) {
        println!("    ⏳ 剩余时间: {}", remaining);
    }
}

/// 错误处理
pub struct ErrorHandler;

impl ErrorHandler {
    /// 处理生成错误
    pub fn handle_generation_error(error: &str) {
        println!("❌ 生成失败: {}", error);
    }

    /// 显示可能的失败原因
    pub fn show_failure_reasons() {
        println!("\n❓ 可能的失败原因:");
        println!("  • 密码格式不正确（应该是10位以上数字，以5开头）");
        println!("  • 管理员密码不匹配");
        println!("  • 密码已过期");
        println!("  • 密码不是由本系统生成");
        println!("  • 时间窗口不匹配（系统时间可能不同步）");
    }
}