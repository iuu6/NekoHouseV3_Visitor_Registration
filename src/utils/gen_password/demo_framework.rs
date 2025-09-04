//! 演示框架模块
//! 提供统一的密码演示功能

use crate::ui_utils::{InputHandler, Formatter, ErrorHandler};
use crate::{UnifiedPasswordGenerator, PasswordType};
use crate::temp_password::TempPasswordGenerator;
use crate::times_password::TimesPasswordGenerator;
use crate::limited_password::LimitedPasswordGenerator;
use crate::period_password::PeriodPasswordGenerator;
use chrono::Datelike;

/// 演示框架结构
pub struct DemoFramework {
    generator: UnifiedPasswordGenerator,
}

impl DemoFramework {
    /// 创建新的演示框架
    pub fn new() -> Self {
        DemoFramework {
            generator: UnifiedPasswordGenerator::new(),
        }
    }

    /// 执行临时密码演示
    pub fn demo_temp_password(&self, admin_pwd: &str) {
        Formatter::format_title("🕐 临时密码生成");
        
        match self.generator.generate(admin_pwd, PasswordType::Temporary) {
            Ok(result) => {
                Formatter::format_generation_result(&result.password, &result.expire_time, &result.message);
                
                // 显示当前时间窗口信息
                let (window, start, end) = TempPasswordGenerator::get_current_window_info();
                Formatter::format_window_info("当前时间窗口", &format!("{} ({} - {})", window, start, end));
                
                // 验证密码
                self.verify_and_show_result(&result.password, admin_pwd, "临时密码");
            }
            Err(e) => ErrorHandler::handle_generation_error(&e),
        }
    }

    /// 执行次数密码演示
    pub fn demo_times_password(&self, admin_pwd: &str) {
        Formatter::format_title("🔢 次数密码生成");
        
        let times = InputHandler::get_number_input("请输入使用次数 (1-31): ", 1u32, 31u32, 5u32);
        
        match self.generator.generate(admin_pwd, PasswordType::Times(times)) {
            Ok(result) => {
                Formatter::format_generation_result(&result.password, &result.expire_time, &result.message);
                
                // 显示时间窗口信息
                let (current, aligned, start, expire) = TimesPasswordGenerator::get_current_window_info();
                println!("🕰️  时间窗口信息:");
                println!("   当前窗口: {}, 对齐窗口: {}", current, aligned);
                println!("   窗口开始: {}", start);
                println!("   密码过期: {}", expire);
                
                // 验证密码
                self.verify_and_show_result(&result.password, admin_pwd, "次数密码");
                
                // 显示剩余时间
                if let Some(remaining) = self.generator.get_remaining_time(&result.password, admin_pwd) {
                    Formatter::format_remaining_time(&remaining);
                }
            }
            Err(e) => ErrorHandler::handle_generation_error(&e),
        }
    }

    /// 执行限时密码演示
    pub fn demo_limited_password(&self, admin_pwd: &str) {
        Formatter::format_title("⏱️ 限时密码生成");
        
        let hours = InputHandler::get_number_input("请输入小时数 (0-127): ", 0u32, 127u32, 2u32);
        let minutes = match InputHandler::get_input("请输入分钟数 (0或30): ").parse::<u32>() {
            Ok(30) => 30,
            Ok(0) => 0,
            _ => {
                println!("❌ 无效的分钟数，使用默认值0");
                0
            }
        };
        
        match self.generator.generate(admin_pwd, PasswordType::Limited(hours, minutes)) {
            Ok(result) => {
                Formatter::format_generation_result(&result.password, &result.expire_time, &result.message);
                
                // 显示时间窗口信息
                let (window, start, end) = LimitedPasswordGenerator::get_current_window_info();
                Formatter::format_window_info("当前30分钟窗口", &format!("{} ({} - {})", window, start, end));
                
                // 验证密码
                self.verify_and_show_result(&result.password, admin_pwd, "限时密码");
                
                // 显示剩余时间
                if let Some(remaining) = self.generator.get_remaining_time(&result.password, admin_pwd) {
                    Formatter::format_remaining_time(&remaining);
                }
            }
            Err(e) => ErrorHandler::handle_generation_error(&e),
        }
    }

    /// 执行周期密码演示
    pub fn demo_period_password(&self, admin_pwd: &str) {
        Formatter::format_title("📅 周期密码生成");
        
        println!("请输入结束时间:");
        let year = InputHandler::get_number_input("年份 (例: 2024): ", 2024u32, 2030u32, 2024u32);
        let month = InputHandler::get_number_input("月份 (1-12): ", 1u32, 12u32, chrono::Utc::now().month());
        let day = InputHandler::get_number_input("日期 (1-31): ", 1u32, 31u32, chrono::Utc::now().day() + 1);
        let hour = InputHandler::get_number_input("小时 (0-23): ", 0u32, 23u32, 18u32);
        
        match self.generator.generate(admin_pwd, PasswordType::Period(year, month, day, hour)) {
            Ok(result) => {
                Formatter::format_generation_result(&result.password, &result.expire_time, &result.message);
                
                // 验证密码
                self.verify_and_show_result(&result.password, admin_pwd, "周期密码");
                
                // 显示剩余时间
                if let Some(remaining) = self.generator.get_remaining_time(&result.password, admin_pwd) {
                    Formatter::format_remaining_time(&remaining);
                }
                
                // 演示字符串格式生成
                self.demo_period_password_from_string(admin_pwd, year, month, day, hour);
            }
            Err(e) => ErrorHandler::handle_generation_error(&e),
        }
    }

    /// 演示从字符串生成周期密码
    fn demo_period_password_from_string(&self, admin_pwd: &str, year: u32, month: u32, day: u32, hour: u32) {
        println!("\n📝 也可以使用字符串格式生成:");
        let datetime_str = format!("{:04}-{:02}-{:02} {:02}:00:00", year, month, day, hour);
        
        use crate::period_password::generate_period_password_from_string;
        match generate_period_password_from_string(admin_pwd, &datetime_str) {
            Ok((password, expire_time, _)) => {
                println!("🔐 字符串格式生成的密码: {}", password);
                println!("⏰ 过期时间: {}", expire_time);
            }
            Err(e) => println!("❌ 字符串格式生成失败: {}", e),
        }
    }

    /// 通用验证并显示结果
    fn verify_and_show_result(&self, password: &str, admin_pwd: &str, password_type: &str) {
        match self.generator.verify(password, admin_pwd) {
            Some(_) => println!("✅ {}验证成功", password_type),
            None => println!("❌ {}验证失败", password_type),
        }
    }

    /// 执行密码验证演示
    pub fn demo_verify_password(&self, admin_pwd: &str) {
        Formatter::format_title("🔍 密码验证");
        
        let password = InputHandler::get_input("请输入要验证的密码: ");
        
        // 使用统一生成器验证密码
        match self.generator.verify(&password, admin_pwd) {
            Some(password_type) => {
                println!("✅ 密码验证成功!");
                println!("🏷️  密码类型: {:?}", password_type);
                
                // 获取剩余时间
                if let Some(remaining) = self.generator.get_remaining_time(&password, admin_pwd) {
                    Formatter::format_remaining_time(&remaining);
                }
            }
            None => {
                println!("❌ 密码验证失败");
            }
        }
        
        // 详细验证所有类型
        self.detailed_verification(&password, admin_pwd);
        
        // 如果验证失败，显示可能原因
        if self.generator.verify(&password, admin_pwd).is_none() {
            ErrorHandler::show_failure_reasons();
        }
    }

    /// 详细验证所有类型
    fn detailed_verification(&self, password: &str, admin_pwd: &str) {
        println!("\n🔍 详细验证结果:");
        
        self.verify_temp_password_detailed(password, admin_pwd);
        self.verify_times_password_detailed(password, admin_pwd);
        self.verify_limited_password_detailed(password, admin_pwd);
        self.verify_period_password_detailed(password, admin_pwd);
    }

    /// 详细验证临时密码
    fn verify_temp_password_detailed(&self, password: &str, admin_pwd: &str) {
        let temp_gen = TempPasswordGenerator::new();
        if temp_gen.verify(password, admin_pwd, 150) {
            Formatter::format_verification_success("临时密码", None);
            if let Some(remaining_ms) = temp_gen.check_remaining_time(password, admin_pwd) {
                let remaining_minutes = remaining_ms / (1000 * 60);
                let remaining_seconds = (remaining_ms % (1000 * 60)) / 1000;
                Formatter::format_verification_success("", Some(&format!("剩余时间: {}分{}秒", remaining_minutes, remaining_seconds)));
            }
            let (window, start, end) = TempPasswordGenerator::get_current_window_info();
            Formatter::format_window_info("当前时间窗口", &format!("{} ({} - {})", window, start, end));
        } else {
            Formatter::format_verification_failure("临时密码", "基于4秒时间窗口，有效期10分钟");
        }
    }

    /// 详细验证次数密码
    fn verify_times_password_detailed(&self, password: &str, admin_pwd: &str) {
        let times_gen = TimesPasswordGenerator::new();
        if let Some(times) = times_gen.verify(password, admin_pwd, 0, 5) {
            Formatter::format_verification_success("次数密码", Some(&format!("可使用{}次", times)));
            if let Some((remaining_ms, _)) = times_gen.check_remaining_time(password, admin_pwd) {
                let remaining_hours = remaining_ms / (1000 * 60 * 60);
                let remaining_minutes = (remaining_ms % (1000 * 60 * 60)) / (1000 * 60);
                Formatter::format_verification_success("", Some(&format!("剩余时间: {}小时{}分钟", remaining_hours, remaining_minutes)));
            }
            let (current, aligned, start, expire) = TimesPasswordGenerator::get_current_window_info();
            println!("    🕰️  时间窗口: 当前={}, 对齐={}", current, aligned);
            println!("    📅 窗口开始: {}", start);
            println!("    ⏰ 密码过期: {}", expire);
        } else {
            Formatter::format_verification_failure("次数密码", "可使用1-31次，有效期20小时");
        }
    }

    /// 详细验证限时密码
    fn verify_limited_password_detailed(&self, password: &str, admin_pwd: &str) {
        let limited_gen = LimitedPasswordGenerator::new();
        if let Some((hours, minutes)) = limited_gen.verify(password, admin_pwd, 5) {
            Formatter::format_verification_success("限时密码", Some(&format!("时长{}小时{}分钟", hours, minutes)));
            if let Some((remaining_ms, _, _)) = limited_gen.check_remaining_time(password, admin_pwd) {
                let remaining_hours = remaining_ms / (1000 * 60 * 60);
                let remaining_minutes = (remaining_ms % (1000 * 60 * 60)) / (1000 * 60);
                Formatter::format_verification_success("", Some(&format!("剩余时间: {}小时{}分钟", remaining_hours, remaining_minutes)));
            }
            let (window, start, end) = LimitedPasswordGenerator::get_current_window_info();
            Formatter::format_window_info("当前30分钟窗口", &format!("{} ({} - {})", window, start, end));
        } else {
            Formatter::format_verification_failure("限时密码", "指定时长有效，基于30分钟时间窗口");
        }
    }

    /// 详细验证周期密码
    fn verify_period_password_detailed(&self, password: &str, admin_pwd: &str) {
        let period_gen = PeriodPasswordGenerator::new();
        if let Some(expire_time) = period_gen.verify(password, admin_pwd, 2) {
            Formatter::format_verification_success("周期密码", None);
            println!("    ⏰ 过期时间: {}", expire_time);
            if let Some((remaining_ms, _)) = period_gen.check_remaining_time(password, admin_pwd) {
                let remaining_hours = remaining_ms / (1000 * 60 * 60);
                let remaining_minutes = (remaining_ms % (1000 * 60 * 60)) / (1000 * 60);
                Formatter::format_verification_success("", Some(&format!("剩余时间: {}小时{}分钟", remaining_hours, remaining_minutes)));
            }
        } else {
            Formatter::format_verification_failure("周期密码", "指定时间段有效，精确到小时");
        }
    }
}

impl Default for DemoFramework {
    fn default() -> Self {
        Self::new()
    }
}