//! NekoHouse V3 访客登记系统 - 主程序入口

use nekohouse_v3_visitor_registration::{
    bot::NekoHouseBot,
    config::{AppConfig, ConfigManager},
    error::Result,
};
use std::env;
use std::error::Error;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    init_logger();

    log::info!("🚀 NekoHouse V3 访客登记系统启动中...");

    // 获取配置文件路径
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "config.json".to_string());
    
    // 检查配置文件是否存在
    if !Path::new(&config_path).exists() {
        log::error!("配置文件 {} 不存在！", config_path);
        log::info!("请创建配置文件，参考格式：");
        print_config_example();
        return Err(nekohouse_v3_visitor_registration::AppError::validation(
            format!("配置文件 {} 不存在", config_path)
        ));
    }

    // 加载配置
    log::info!("📖 加载配置文件: {}", config_path);
    let config_manager = ConfigManager::new(&config_path)?;
    let config = config_manager.get_config().clone();

    // 验证配置
    log::info!("✅ 配置验证通过");
    log::info!("🗄️  数据库路径: {}", config.database.path);
    log::info!("👥 超级管理员数量: {}", config.super_admin_ids.len());
    log::info!("⏰ 时间偏移: {} 秒", config.time_offset);

    // 测试网络连接
    log::info!("🔍 测试网络连接...");
    test_network_connection().await?;

    // 创建并启动Bot
    log::info!("🤖 初始化Telegram Bot...");
    let bot = NekoHouseBot::new(config).await?;
    
    log::info!("🎯 系统准备就绪，开始监听消息...");
    log::info!("📱 Bot信息: {:?}", bot.state().get_bot_info().await);
    
    // 运行Bot (这会阻塞直到收到停止信号)
    bot.run().await?;

    log::info!("👋 NekoHouse V3 访客登记系统已停止");
    Ok(())
}

/// 初始化日志记录器
fn init_logger() {
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&log_level))
        .format_timestamp_secs()
        .init();

    log::info!("📝 日志级别: {}", log_level);
}

/// 打印配置文件示例
fn print_config_example() {
    println!(r#"
{{
  "database": {{
    "path": "./data/nekohouse.db"
  }},
  "telegram": {{
    "bot_token": "YOUR_BOT_TOKEN_HERE"
  }},
  "super_admin_ids": [
    1234567890
  ],
  "time_offset": 3600
}}

配置说明：
- database.path: SQLite数据库文件路径
- telegram.bot_token: 从 @BotFather 获取的Bot Token
- super_admin_ids: 超级管理员的Telegram用户ID列表
- time_offset: 密码生成时间偏移（秒），用于增加安全性

获取用户ID的方法：
1. 发送消息给 @userinfobot
2. 或在Bot对话中发送 /start，系统会显示您的用户ID
"#);
}

/// 测试网络连接
async fn test_network_connection() -> Result<()> {
    log::info!("正在测试 HTTPS 连接到 api.telegram.org...");
    
    // 使用 reqwest 测试连接
    let client = reqwest::Client::new();
    let url = "https://api.telegram.org/";
    
    match client.head(url).send().await {
        Ok(response) => {
            log::info!("✅ 网络连接测试成功");
            log::info!("状态码: {}", response.status());
            log::info!("服务器: {:?}", response.headers().get("server"));
        }
        Err(e) => {
            log::error!("❌ 网络连接测试失败: {}", e);
            log::error!("错误详情: {:?}", e);
            
            // 尝试更详细的错误分析
            if let Some(source) = e.source() {
                log::error!("错误源: {:?}", source);
            }
            
            return Err(nekohouse_v3_visitor_registration::AppError::business(
                format!("网络连接测试失败: {}", e)
            ));
        }
    }
    
    Ok(())
}

/// 处理优雅关闭信号
async fn setup_signal_handlers() -> Result<()> {
    use tokio::signal;

    #[cfg(unix)]
    {
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())?;

        tokio::select! {
            _ = sigterm.recv() => {
                log::info!("收到SIGTERM信号，开始优雅关闭...");
            }
            _ = sigint.recv() => {
                log::info!("收到SIGINT信号，开始优雅关闭...");
            }
        }
    }

    #[cfg(not(unix))]
    {
        signal::ctrl_c().await?;
        log::info!("收到Ctrl+C信号，开始优雅关闭...");
    }

    Ok(())
}

/// 应用程序信息
pub fn print_app_info() {
    println!(r#"
  _   _      _         _   _                        _____ 
 | \ | |    | |       | | | |                      |_   _|
 |  \| | ___| | ___   | |_| | ___  _   _ ___  ___    | |  
 | . ` |/ _ \ |/ / |  |  _  |/ _ \| | | / __|/ _ \   | |  
 | |\  |  __/   <| |__| | | | (_) | |_| \__ \  __/  _| |_ 
 |_| \_|\___|_|\_\\____\_| |_/\___/ \__,_|___/\___| |_____|
                                                          
    NekoHouse V3 访客登记系统
    Visitor Registration System
    
    版本: 0.1.0
    作者: NekoHouse Team
    
    功能特性:
    ✅ 三级权限管理（超级管理员/管理员/访客）
    ✅ 多种授权类型（临时/次数/时效/指定时间/长期临时）
    ✅ 基于KeeLoq算法的安全密码生成
    ✅ SQLite数据库存储
    ✅ 完整的Telegram Bot交互
    
"#);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::NamedTempFile;

    #[test]
    fn test_app_info() {
        // 测试应用信息打印不会panic
        print_app_info();
    }

    #[test] 
    fn test_config_example() {
        // 测试配置示例打印不会panic
        print_config_example();
    }

    #[tokio::test]
    async fn test_main_with_missing_config() {
        // 设置一个不存在的配置文件路径
        let temp_file = NamedTempFile::new().unwrap();
        let non_existent_path = temp_file.path().to_str().unwrap().to_string() + ".missing";
        
        env::set_var("CONFIG_PATH", &non_existent_path);
        
        // 主函数应该返回错误
        let result = main().await;
        assert!(result.is_err());
        
        env::remove_var("CONFIG_PATH");
    }
}