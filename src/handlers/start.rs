//! /start命令处理器

use crate::bot::bot::BotState;
use crate::error::Result;
use crate::types::UserRole;
use teloxide::{prelude::*, types::User};

/// 处理/start命令
pub async fn start_command(bot: Bot, msg: Message, _cmd: crate::bot::bot::Command, state: BotState) -> Result<()> {
    let user = match msg.from() {
        Some(user) => user,
        None => {
            bot.send_message(msg.chat.id, "无法获取用户信息").await?;
            return Ok(());
        }
    };

    log::info!("用户 {} (@{}) 发送了/start命令", 
        user.id.0, 
        user.username.as_deref().unwrap_or("无用户名")
    );

    // 获取用户信息和角色
    let user_service = state.user_service.read().await;
    let user_info = user_service.get_user_info(user).await?;
    drop(user_service);

    // 根据用户角色发送不同的欢迎消息
    let response_text = match user_info.role {
        UserRole::SuperAdmin => {
            format!(
                "您好，猫窝三期超管！\n\n\
                 欢迎使用NekoHouse V3访客登记系统\n\
                 您的权限：超级管理员\n\n\
                 可用命令：\n\
                 /addadmin <用户ID> - 添加管理员\n\
                 /editpasswd <密码> - 修改密码\n\
                 /geninvite - 生成邀请码\n\
                 /revoke <目标> - 撤销授权\n\
                 /getpassword - 获取临时密码\n\n\
                 用户ID：{}", 
                user.id.0
            )
        }
        UserRole::Admin => {
            format!(
                "您好，猫窝三期管理员！\n\n\
                 欢迎使用NekoHouse V3访客登记系统\n\
                 您的权限：管理员\n\n\
                 可用命令：\n\
                 /editpasswd <密码> - 修改密码\n\
                 /geninvite - 生成邀请码\n\
                 /revoke <目标> - 撤销授权\n\
                 /getpassword - 获取临时密码\n\n\
                 用户ID：{}", 
                user.id.0
            )
        }
        UserRole::Visitor => {
            format!(
                "hi～您好访客：ID{}\n\
                 您似乎尝试到访NekoHouseV3，现在我们为您操作批准，请填写到访邀请码：\n\n\
                 可用命令：\n\
                 /req <邀请码> - 申请访客授权\n\
                 /getpassword - 获取密码（需要先获得授权）\n\n\
                 请联系管理员获取邀请码", 
                user.id.0
            )
        }
    };

    bot.send_message(msg.chat.id, response_text).await?;

    Ok(())
}

/// 生成用户欢迎消息
pub fn generate_welcome_message(user: &User, role: UserRole) -> String {
    let display_name = get_user_display_name(user);
    let user_id = user.id.0;

    match role {
        UserRole::SuperAdmin => format!(
            "🔥 欢迎超级管理员 {}\n\n\
             🏠 NekoHouse V3 访客登记系统\n\
             👑 您拥有最高权限\n\n\
             📋 可用功能：\n\
             • 管理员管理（添加/删除）\n\
             • 密码管理\n\
             • 邀请码生成\n\
             • 授权撤销\n\
             • 临时密码获取\n\n\
             🆔 您的用户ID：{}", 
            display_name, user_id
        ),
        UserRole::Admin => format!(
            "👋 欢迎管理员 {}\n\n\
             🏠 NekoHouse V3 访客登记系统\n\
             🛡️ 您是系统管理员\n\n\
             📋 可用功能：\n\
             • 密码管理\n\
             • 邀请码生成\n\
             • 访客授权管理\n\
             • 临时密码获取\n\n\
             🆔 您的用户ID：{}", 
            display_name, user_id
        ),
        UserRole::Visitor => format!(
            "🎯 您好访客 {}\n\
             🆔 ID: {}\n\n\
             🏠 欢迎来到 NekoHouse V3！\n\
             📝 现在为您处理访问申请\n\n\
             🎫 请填写到访邀请码：\n\
             使用命令 /req <邀请码>\n\n\
             💡 如需帮助，请联系管理员获取邀请码", 
            display_name, user_id
        ),
    }
}

/// 获取用户显示名称
pub fn get_user_display_name(user: &User) -> String {
    if let Some(ref username) = user.username {
        format!("@{}", username)
    } else {
        let last_name = user.last_name.as_deref().unwrap_or("");
        format!("{} {}", user.first_name, last_name).trim().to_string()
    }
}

/// 生成角色描述
pub fn get_role_description(role: UserRole) -> &'static str {
    match role {
        UserRole::SuperAdmin => "超级管理员",
        UserRole::Admin => "管理员", 
        UserRole::Visitor => "访客",
    }
}

/// 生成命令帮助文本
pub fn generate_command_help(role: UserRole) -> String {
    match role {
        UserRole::SuperAdmin => {
            "🔧 超级管理员命令：\n\
             /addadmin <用户ID> - 添加新管理员\n\
             /editpasswd <密码> - 修改管理密码（4-10位数字）\n\
             /geninvite - 生成/更新邀请码\n\
             /revoke <目标> - 撤销授权（record ID或user ID）\n\
             /getpassword - 获取临时密码\n\n\
             💡 提示：超级管理员拥有所有权限".to_string()
        }
        UserRole::Admin => {
            "🛠️ 管理员命令：\n\
             /editpasswd <密码> - 修改管理密码（4-10位数字）\n\
             /geninvite - 生成/更新邀请码\n\
             /revoke <目标> - 撤销授权（record ID或user ID）\n\
             /getpassword - 获取临时密码\n\n\
             💡 提示：首次使用前请先设置管理密码".to_string()
        }
        UserRole::Visitor => {
            "📋 访客命令：\n\
             /req <邀请码> - 申请访客授权\n\
             /getpassword - 获取密码（需要先获得授权）\n\n\
             💡 提示：请向管理员申请邀请码\n\
             ⚠️ 一个用户同时只能有一个待处理请求或活跃授权".to_string()
        }
    }
}

/// 检查用户输入格式
pub fn validate_user_input(input: &str, expected_type: &str) -> Result<()> {
    match expected_type {
        "user_id" => {
            input.parse::<i64>()
                .map_err(|_| crate::error::AppError::validation("用户ID必须是数字"))?;
        }
        "password" => {
            if input.len() < 4 || input.len() > 10 {
                return Err(crate::error::AppError::validation("密码长度必须在4-10位之间"));
            }
            if !input.chars().all(|c| c.is_ascii_digit()) {
                return Err(crate::error::AppError::validation("密码只能包含数字"));
            }
        }
        "invite_code" => {
            // UUID格式验证
            uuid::Uuid::parse_str(input)
                .map_err(|_| crate::error::AppError::validation("邀请码格式不正确"))?;
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use teloxide::types::{Chat, ChatKind, ChatPrivate, UserId};

    fn create_test_user(id: u64, username: Option<String>, first_name: String) -> User {
        User {
            id: UserId(id),
            is_bot: false,
            first_name,
            last_name: None,
            username,
            language_code: None,
            is_premium: false,
            added_to_attachment_menu: false,
        }
    }

    #[test]
    fn test_get_user_display_name() {
        let user_with_username = create_test_user(123, Some("testuser".to_string()), "Test".to_string());
        assert_eq!(get_user_display_name(&user_with_username), "@testuser");

        let user_without_username = create_test_user(456, None, "Test".to_string());
        assert_eq!(get_user_display_name(&user_without_username), "Test");
    }

    #[test]
    fn test_validate_user_input() {
        // 测试用户ID验证
        assert!(validate_user_input("123456789", "user_id").is_ok());
        assert!(validate_user_input("invalid", "user_id").is_err());

        // 测试密码验证
        assert!(validate_user_input("1234", "password").is_ok());
        assert!(validate_user_input("12345678", "password").is_ok());
        assert!(validate_user_input("123", "password").is_err()); // 太短
        assert!(validate_user_input("12345678901", "password").is_err()); // 太长
        assert!(validate_user_input("12a4", "password").is_err()); // 包含字母

        // 测试邀请码验证
        let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert!(validate_user_input(valid_uuid, "invite_code").is_ok());
        assert!(validate_user_input("invalid-uuid", "invite_code").is_err());
    }

    #[test]
    fn test_generate_welcome_message() {
        let user = create_test_user(123, Some("testuser".to_string()), "Test".to_string());

        let super_admin_msg = generate_welcome_message(&user, UserRole::SuperAdmin);
        assert!(super_admin_msg.contains("超级管理员"));
        assert!(super_admin_msg.contains("@testuser"));
        assert!(super_admin_msg.contains("123"));

        let admin_msg = generate_welcome_message(&user, UserRole::Admin);
        assert!(admin_msg.contains("管理员"));
        assert!(admin_msg.contains("@testuser"));

        let visitor_msg = generate_welcome_message(&user, UserRole::Visitor);
        assert!(visitor_msg.contains("访客"));
        assert!(visitor_msg.contains("邀请码"));
    }

    #[test]
    fn test_generate_command_help() {
        let super_admin_help = generate_command_help(UserRole::SuperAdmin);
        assert!(super_admin_help.contains("/addadmin"));
        assert!(super_admin_help.contains("超级管理员"));

        let admin_help = generate_command_help(UserRole::Admin);
        assert!(admin_help.contains("/editpasswd"));
        assert!(!admin_help.contains("/addadmin")); // 管理员没有添加管理员权限

        let visitor_help = generate_command_help(UserRole::Visitor);
        assert!(visitor_help.contains("/req"));
        assert!(visitor_help.contains("邀请码"));
    }
}