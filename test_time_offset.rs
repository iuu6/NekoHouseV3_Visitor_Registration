//! 测试时间偏移逻辑的修复

use nekohouse_v3_visitor_registration::utils::gen_password::*;

fn main() {
    println!("=== 测试时间偏移密码生成逻辑修复 ===\n");
    
    let admin_pwd = "123456";
    let time_offset = 180i32; // 3分钟偏移
    
    println!("管理员密码: {}", admin_pwd);
    println!("时间偏移: {}秒\n", time_offset);
    
    // 测试临时密码
    test_temp_password(admin_pwd, time_offset);
    
    // 测试次数密码
    test_times_password(admin_pwd, time_offset);
    
    // 测试限时密码
    test_limited_password(admin_pwd, time_offset);
    
    // 测试周期密码
    test_period_password(admin_pwd, time_offset);
    
    println!("=== 所有测试完成 ===");
}

fn test_temp_password(admin_pwd: &str, time_offset: i32) {
    println!("--- 测试临时密码（带时间偏移） ---");
    
    // 使用带偏移的生成器
    let gen_with_offset = TempPasswordGeneratorWithOffset::new(time_offset);
    let gen_normal = TempPasswordGenerator::new();
    
    match gen_with_offset.generate(admin_pwd) {
        Ok((password_offset, expire_time, message)) => {
            println!("带偏移的密码: {}", password_offset);
            println!("过期时间: {}", expire_time);
            println!("消息: {}", message);
            
            // 生成正常密码作为对比
            if let Ok((password_normal, _, _)) = gen_normal.generate(admin_pwd) {
                println!("正常密码: {}", password_normal);
                
                // 验证带偏移的密码能否被带偏移的生成器验证
                let verify_result = gen_with_offset.verify(&password_offset, admin_pwd, 1);
                println!("带偏移生成器验证结果: {}", verify_result);
                
                // 验证正常密码是否与带偏移的密码不同
                if password_offset != password_normal {
                    println!("✅ 成功：带偏移的密码与正常密码不同，偏移逻辑生效");
                } else {
                    println!("❌ 失败：带偏移的密码与正常密码相同，偏移逻辑未生效");
                }
            }
        }
        Err(e) => println!("❌ 生成带偏移临时密码失败: {}", e),
    }
    println!();
}

fn test_times_password(admin_pwd: &str, time_offset: i32) {
    println!("--- 测试次数密码（带时间偏移） ---");
    
    let gen_with_offset = TimesPasswordGeneratorWithOffset::new(time_offset);
    let gen_normal = TimesPasswordGenerator::new();
    let use_times = 5u32;
    
    match gen_with_offset.generate(admin_pwd, use_times) {
        Ok((password_offset, expire_time, message)) => {
            println!("带偏移的次数密码: {}", password_offset);
            println!("过期时间: {}", expire_time);
            println!("消息: {}", message);
            
            if let Ok((password_normal, _, _)) = gen_normal.generate(admin_pwd, use_times) {
                println!("正常次数密码: {}", password_normal);
                
                let verify_result = gen_with_offset.verify(&password_offset, admin_pwd, 0, 2);
                println!("验证结果: {:?}", verify_result);
                
                if password_offset != password_normal {
                    println!("✅ 成功：带偏移的次数密码与正常密码不同");
                } else {
                    println!("❌ 失败：带偏移的次数密码与正常密码相同");
                }
            }
        }
        Err(e) => println!("❌ 生成带偏移次数密码失败: {}", e),
    }
    println!();
}

fn test_limited_password(admin_pwd: &str, time_offset: i32) {
    println!("--- 测试限时密码（带时间偏移） ---");
    
    let gen_with_offset = LimitedPasswordGeneratorWithOffset::new(time_offset);
    let gen_normal = LimitedPasswordGenerator::new();
    let hours = 2u32;
    let minutes = 30u32;
    
    match gen_with_offset.generate(admin_pwd, hours, minutes) {
        Ok((password_offset, expire_time, message)) => {
            println!("带偏移的限时密码: {}", password_offset);
            println!("过期时间: {}", expire_time);
            println!("消息: {}", message);
            
            if let Ok((password_normal, _, _)) = gen_normal.generate(admin_pwd, hours, minutes) {
                println!("正常限时密码: {}", password_normal);
                
                let verify_result = gen_with_offset.verify(&password_offset, admin_pwd, 2);
                println!("验证结果: {:?}", verify_result);
                
                if password_offset != password_normal {
                    println!("✅ 成功：带偏移的限时密码与正常密码不同");
                } else {
                    println!("❌ 失败：带偏移的限时密码与正常密码相同");
                }
            }
        }
        Err(e) => println!("❌ 生成带偏移限时密码失败: {}", e),
    }
    println!();
}

fn test_period_password(admin_pwd: &str, time_offset: i32) {
    println!("--- 测试周期密码（带时间偏移） ---");
    
    let gen_with_offset = PeriodPasswordGeneratorWithOffset::new(time_offset);
    let gen_normal = PeriodPasswordGenerator::new();
    
    // 设置一个未来的时间
    let year = 2024u32;
    let month = 12u32;
    let day = 31u32;
    let hour = 23u32;
    
    match gen_with_offset.generate(admin_pwd, year, month, day, hour) {
        Ok((password_offset, expire_time, message)) => {
            println!("带偏移的周期密码: {}", password_offset);
            println!("过期时间: {}", expire_time);
            println!("消息: {}", message);
            
            if let Ok((password_normal, _, _)) = gen_normal.generate(admin_pwd, year, month, day, hour) {
                println!("正常周期密码: {}", password_normal);
                
                let verify_result = gen_with_offset.verify(&password_offset, admin_pwd, 1);
                println!("验证结果: {:?}", verify_result);
                
                if password_offset != password_normal {
                    println!("✅ 成功：带偏移的周期密码与正常密码不同");
                } else {
                    println!("❌ 失败：带偏移的周期密码与正常密码相同");
                }
            }
        }
        Err(e) => println!("❌ 生成带偏移周期密码失败: {}", e),
    }
    println!();
}