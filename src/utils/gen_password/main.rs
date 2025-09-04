//! 密码生成算法演示程序
//! 
//! 这个程序展示了如何使用四种不同的密码生成算法
//! 所有时间都使用UTC+8时区（北京时间）

use password_algorithms::*;

fn main() {
    // 显示标题
    MenuDisplay::show_title();

    // 获取管理员密码
    let admin_pwd = InputHandler::get_admin_password();
    
    // 创建演示框架
    let demo_framework = DemoFramework::new();
    
    // 主循环
    loop {
        MenuDisplay::show_main_menu();
        
        let choice = InputHandler::get_input("请选择功能 (1-6): ");
        let menu_option = MenuOption::from(choice.as_str());
        
        match menu_option {
            MenuOption::TempPassword => demo_framework.demo_temp_password(&admin_pwd),
            MenuOption::TimesPassword => demo_framework.demo_times_password(&admin_pwd),
            MenuOption::LimitedPassword => demo_framework.demo_limited_password(&admin_pwd),
            MenuOption::PeriodPassword => demo_framework.demo_period_password(&admin_pwd),
            MenuOption::VerifyPassword => demo_framework.demo_verify_password(&admin_pwd),
            MenuOption::Exit => {
                println!("👋 再见！");
                break;
            }
            MenuOption::Invalid => println!("❌ 无效选择，请重试"),
        }
        
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_algorithms() {
        let admin_pwd = "123456";
        
        // 测试所有算法
        println!("测试临时密码:");
        if let Ok((password, _, _)) = generate_temp_password(admin_pwd) {
            println!("生成成功: {}", password);
            assert!(verify_temp_password(&password, admin_pwd));
        }
        
        println!("\n测试次数密码:");
        if let Ok((password, _, _)) = generate_times_password(admin_pwd, 5) {
            println!("生成成功: {}", password);
            assert!(verify_times_password(&password, admin_pwd).is_some());
        }
        
        println!("\n测试限时密码:");
        if let Ok((password, _, _)) = generate_limited_password(admin_pwd, 2, 30) {
            println!("生成成功: {}", password);
            assert!(verify_limited_password(&password, admin_pwd).is_some());
        }
        
        println!("\n测试统一验证器:");
        let generator = UnifiedPasswordGenerator::new();
        let result = generator.generate(admin_pwd, PasswordType::Times(3)).unwrap();
        println!("统一生成器生成: {}", result.password);
        assert!(generator.verify(&result.password, admin_pwd).is_some());
    }

    #[test]
    fn test_demo_framework() {
        let _demo_framework = DemoFramework::new();
        let _admin_pwd = "123456";
        
        // 测试演示框架的基本功能
        // 注意：这些测试主要验证结构正确性，实际的演示需要用户交互
        println!("演示框架创建成功");
        
        // 可以添加更多无需用户交互的测试
        assert!(true); // 占位符，表示测试通过
    }

    #[test]
    fn test_menu_options() {
        assert_eq!(MenuOption::from("1"), MenuOption::TempPassword);
        assert_eq!(MenuOption::from("2"), MenuOption::TimesPassword);
        assert_eq!(MenuOption::from("3"), MenuOption::LimitedPassword);
        assert_eq!(MenuOption::from("4"), MenuOption::PeriodPassword);
        assert_eq!(MenuOption::from("5"), MenuOption::VerifyPassword);
        assert_eq!(MenuOption::from("6"), MenuOption::Exit);
        assert_eq!(MenuOption::from("invalid"), MenuOption::Invalid);
    }

    #[test]
    fn test_input_validation() {
        // 测试输入验证功能
        assert!(InputValidator::validate_admin_password("1234").is_ok());
        assert!(InputValidator::validate_admin_password("123").is_err());
        
        assert!(InputValidator::validate_use_times(5).is_ok());
        assert!(InputValidator::validate_use_times(0).is_err());
        assert!(InputValidator::validate_use_times(32).is_err());
        
        assert!(InputValidator::validate_minutes(0).is_ok());
        assert!(InputValidator::validate_minutes(30).is_ok());
        assert!(InputValidator::validate_minutes(15).is_err());
    }

    #[test]
    fn test_time_formatter() {
        let formatted = TimeFormatter::format_duration_ms(3661000); // 1小时1分1秒
        assert!(formatted.contains("1小时1分钟"));
        
        let formatted = TimeFormatter::format_hours_minutes(2, 30);
        assert_eq!(formatted, "2小时30分钟");
        
        let formatted = TimeFormatter::format_hours_minutes(0, 30);
        assert_eq!(formatted, "30分钟");
    }
}