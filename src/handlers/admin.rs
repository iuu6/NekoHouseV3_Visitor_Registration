//! 管理员命令处理器

use crate::bot::bot::BotState;
use crate::database::{AdminRepository, RecordRepository};
use crate::error::{AppError, Result};
use crate::handlers::start::{get_user_display_name, validate_user_input};
use crate::types::{AuthStatus, AuthType, CallbackData, UserRole};
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
};

/// 处理/addadmin命令 - 添加管理员（仅超级管理员）
pub async fn add_admin_command(
    bot: Bot,
    msg: Message,
    cmd: crate::bot::bot::Command,
    state: BotState,
) -> Result<()> {
    // 从命令中提取用户ID
    let user_id = match cmd {
        crate::bot::bot::Command::AddAdmin(id_str) => {
            match id_str.parse::<i64>() {
                Ok(id) => id,
                Err(_) => {
                    bot.send_message(msg.chat.id, "❌ 用户ID必须是数字").await?;
                    return Ok(());
                }
            }
        }
        _ => {
            bot.send_message(msg.chat.id, "❌ 命令参数错误").await?;
            return Ok(());
        }
    };
    let operator = match msg.from() {
        Some(user) => user,
        None => {
            bot.send_message(msg.chat.id, "无法获取用户信息").await?;
            return Ok(());
        }
    };

    let operator_id = operator.id.0 as i64;

    log::info!("用户 {} 尝试添加管理员 {}", operator_id, user_id);

    // 检查权限
    let user_service = state.user_service.read().await;
    if !user_service.is_super_admin(operator_id) {
        bot.send_message(msg.chat.id, "❌ 只有超级管理员可以添加管理员")
            .await?;
        return Ok(());
    }

    // 验证目标用户ID
    if let Err(e) = validate_user_input(&user_id.to_string(), "user_id") {
        bot.send_message(msg.chat.id, format!("❌ 参数错误: {}", e))
            .await?;
        return Ok(());
    }

    // 尝试添加管理员
    match user_service.create_admin(operator_id, user_id).await {
        Ok(admin_unique_id) => {
            let response = format!(
                "✅ 成功添加管理员！\n\n\
                 👤 目标用户ID: {}\n\
                 🆔 管理员数据库ID: {}\n\n\
                 💡 新管理员需要：\n\
                 1. 发送 /start 激活账户\n\
                 2. 使用 /editpasswd <密码> 设置管理密码\n\
                 3. 使用 /geninvite 生成邀请码",
                user_id, admin_unique_id
            );
            bot.send_message(msg.chat.id, response).await?;
            log::info!("超级管理员 {} 成功添加管理员 {}", operator_id, user_id);
        }
        Err(e) => {
            let error_msg = format!("❌ 添加管理员失败: {}", e);
            bot.send_message(msg.chat.id, error_msg).await?;
            log::warn!("添加管理员失败: {}", e);
        }
    }

    Ok(())
}

/// 处理/editpasswd命令 - 修改管理员密码
pub async fn edit_password_command(
    bot: Bot,
    msg: Message,
    cmd: crate::bot::bot::Command,
    state: BotState,
) -> Result<()> {
    // 从命令中提取密码
    let password = match cmd {
        crate::bot::bot::Command::EditPassword(pwd) => pwd,
        _ => {
            bot.send_message(msg.chat.id, "❌ 命令参数错误").await?;
            return Ok(());
        }
    };
    let user = match msg.from() {
        Some(user) => user,
        None => {
            bot.send_message(msg.chat.id, "无法获取用户信息").await?;
            return Ok(());
        }
    };

    let user_id = user.id.0 as i64;
    log::info!("用户 {} 尝试修改管理密码", user_id);

    // 检查权限
    let user_service = state.user_service.read().await;
    if !user_service.is_admin(user_id).await? {
        bot.send_message(msg.chat.id, "❌ 只有管理员可以修改管理密码")
            .await?;
        return Ok(());
    }

    // 获取管理员信息
    let admin = match user_service.get_admin_info(user_id).await? {
        Some(admin) => admin,
        None => {
            bot.send_message(msg.chat.id, "❌ 管理员信息不存在，请联系超级管理员")
                .await?;
            return Ok(());
        }
    };

    // 验证密码格式
    if let Err(e) = validate_user_input(&password, "password") {
        bot.send_message(msg.chat.id, format!("❌ 密码格式错误: {}", e))
            .await?;
        return Ok(());
    }

    // 更新密码
    match user_service.update_admin_password(admin.unique_id, &password).await {
        Ok(true) => {
            bot.send_message(
                msg.chat.id,
                "✅ 管理密码修改成功！\n\n💡 现在可以使用 /geninvite 生成邀请码"
            ).await?;
            log::info!("管理员 {} 成功修改密码", user_id);
        }
        Ok(false) => {
            bot.send_message(msg.chat.id, "❌ 密码修改失败，请稍后重试")
                .await?;
        }
        Err(e) => {
            bot.send_message(msg.chat.id, format!("❌ 密码修改失败: {}", e))
                .await?;
            log::error!("密码修改失败: {}", e);
        }
    }

    Ok(())
}

/// 处理/geninvite命令 - 生成邀请码
pub async fn gen_invite_command(bot: Bot, msg: Message, state: BotState) -> Result<()> {
    let user = match msg.from() {
        Some(user) => user,
        None => {
            bot.send_message(msg.chat.id, "无法获取用户信息").await?;
            return Ok(());
        }
    };

    let user_id = user.id.0 as i64;
    log::info!("用户 {} 尝试生成邀请码", user_id);

    let user_service = state.user_service.read().await;

    // 检查权限
    if !user_service.is_admin(user_id).await? {
        bot.send_message(msg.chat.id, "❌ 只有管理员可以生成邀请码")
            .await?;
        return Ok(());
    }

    // 获取管理员信息
    let admin = match user_service.get_admin_info(user_id).await? {
        Some(admin) => admin,
        None => {
            bot.send_message(msg.chat.id, "❌ 管理员信息不存在")
                .await?;
            return Ok(());
        }
    };

    // 检查是否已设置密码
    if !user_service.admin_has_password(admin.unique_id).await? {
        bot.send_message(
            msg.chat.id,
            "❌ 请先设置管理密码！\n\n使用命令: /editpasswd <密码>\n密码要求: 4-10位数字"
        ).await?;
        return Ok(());
    }

    // 检查是否已有邀请码
    if let Some(ref existing_code) = admin.invite_code {
        // 已有邀请码，询问是否更换
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("🔄 更换邀请码",
                    CallbackData::new("regenerate_invite").to_callback_string().unwrap()),
                InlineKeyboardButton::callback("❌ 取消",
                    CallbackData::new("cancel").to_callback_string().unwrap()),
            ]
        ]);

        let message = format!(
            "🎫 您当前的邀请码：\n`{}`\n\n\
             ⚠️ 是否要生成新的邀请码？\n\
             注意：原邀请码将失效！",
            existing_code
        );

        bot.send_message(msg.chat.id, message)
            .reply_markup(keyboard)
            .parse_mode(teloxide::types::ParseMode::Markdown)
            .await?;
    } else {
        // 第一次生成邀请码
        generate_new_invite_code(&bot, msg.chat.id, &user_service, admin.unique_id).await?;
    }

    Ok(())
}

/// 处理/revoke命令 - 撤销授权
pub async fn revoke_command(
    bot: Bot,
    msg: Message,
    cmd: crate::bot::bot::Command,
    state: BotState,
) -> Result<()> {
    // 从命令中提取目标
    let target = match cmd {
        crate::bot::bot::Command::Revoke(t) => t,
        _ => {
            bot.send_message(msg.chat.id, "❌ 命令参数错误").await?;
            return Ok(());
        }
    };
    let user = match msg.from() {
        Some(user) => user,
        None => {
            bot.send_message(msg.chat.id, "无法获取用户信息").await?;
            return Ok(());
        }
    };

    let user_id = user.id.0 as i64;
    log::info!("用户 {} 尝试撤销授权: {}", user_id, target);

    let user_service = state.user_service.read().await;

    // 检查权限
    if !user_service.is_admin(user_id).await? {
        bot.send_message(msg.chat.id, "❌ 只有管理员可以撤销授权")
            .await?;
        return Ok(());
    }

    // 解析目标：可能是 record ID 或 user ID
    let revoked_count = if target.starts_with("record") || target.starts_with("r") {
        // 撤销特定记录
        let record_id = target.trim_start_matches("record").trim_start_matches("r");
        match record_id.parse::<i64>() {
            Ok(id) => {
                let mut tx = state.database.begin_transaction().await?;
                let success = RecordRepository::revoke_by_id(&mut tx, id).await?;
                tx.commit().await?;
                if success { 1 } else { 0 }
            }
            Err(_) => {
                bot.send_message(msg.chat.id, "❌ 无效的记录ID格式")
                    .await?;
                return Ok(());
            }
        }
    } else if target.starts_with("user") || target.starts_with("u") {
        // 撤销用户的所有授权
        let user_id_str = target.trim_start_matches("user").trim_start_matches("u");
        match user_id_str.parse::<i64>() {
            Ok(target_user_id) => {
                let mut tx = state.database.begin_transaction().await?;
                let count = RecordRepository::revoke_all_by_visitor(&mut tx, target_user_id).await?;
                tx.commit().await?;
                count
            }
            Err(_) => {
                bot.send_message(msg.chat.id, "❌ 无效的用户ID格式")
                    .await?;
                return Ok(());
            }
        }
    } else {
        // 尝试解析为数字 - 默认当作用户ID处理
        match target.parse::<i64>() {
            Ok(target_user_id) => {
                let mut tx = state.database.begin_transaction().await?;
                let count = RecordRepository::revoke_all_by_visitor(&mut tx, target_user_id).await?;
                tx.commit().await?;
                count
            }
            Err(_) => {
                bot.send_message(
                    msg.chat.id,
                    "❌ 目标格式错误\n\n\
                     正确格式：\n\
                     • /revoke 123456789 (撤销用户所有授权)\n\
                     • /revoke user 123456789 (撤销用户所有授权)\n\
                     • /revoke record 1 (撤销指定记录)"
                ).await?;
                return Ok(());
            }
        }
    };

    // 发送结果消息
    let result_msg = if revoked_count > 0 {
        format!("✅ 成功撤销 {} 条授权记录", revoked_count)
    } else {
        "❌ 没有找到可撤销的授权记录".to_string()
    };

    bot.send_message(msg.chat.id, result_msg).await?;
    log::info!("管理员 {} 撤销了 {} 条授权", user_id, revoked_count);

    Ok(())
}

/// 生成新邀请码的辅助函数
async fn generate_new_invite_code(
    bot: &Bot,
    chat_id: ChatId,
    user_service: &crate::auth::UserService,
    admin_unique_id: i64,
) -> Result<()> {
    match user_service.generate_admin_invite_code(admin_unique_id).await {
        Ok(invite_code) => {
            let message = format!(
                "✅ 邀请码生成成功！\n\n\
                 🎫 您的邀请码：\n`{}`\n\n\
                 📋 使用方法：\n\
                 访客使用命令 /req {} 申请授权\n\n\
                 💡 提示：请妥善保管邀请码",
                invite_code, invite_code
            );

            bot.send_message(chat_id, message)
                .parse_mode(teloxide::types::ParseMode::Markdown)
                .await?;
        }
        Err(e) => {
            bot.send_message(chat_id, format!("❌ 生成邀请码失败: {}", e))
                .await?;
            log::error!("生成邀请码失败: {}", e);
        }
    }
    Ok(())
}

/// 获取管理员状态信息
pub async fn get_admin_status(
    bot: Bot,
    msg: Message,
    state: BotState,
) -> Result<()> {
    let user = match msg.from() {
        Some(user) => user,
        None => return Ok(()),
    };

    let user_id = user.id.0 as i64;
    let user_service = state.user_service.read().await;

    if !user_service.is_admin(user_id).await? {
        return Ok(());
    }

    let admin = match user_service.get_admin_info(user_id).await? {
        Some(admin) => admin,
        None => return Ok(()),
    };

    let has_password = user_service.admin_has_password(admin.unique_id).await?;
    let is_super_admin = user_service.is_super_admin(user_id);

    // 获取管理的授权统计
    let managed_records = RecordRepository::find_by_inviter(state.database.pool(), admin.unique_id).await?;
    let pending_count = managed_records.iter().filter(|r| r.status == AuthStatus::Pending).count();
    let active_count = managed_records.iter().filter(|r| r.status == AuthStatus::Auth && r.is_active()).count();
    let total_count = managed_records.len();

    let status_message = format!(
        "📊 管理员状态\n\n\
         👤 用户: {}\n\
         🆔 ID: {}\n\
         👑 角色: {}\n\
         🔑 密码状态: {}\n\
         🎫 邀请码: {}\n\n\
         📈 管理统计:\n\
         • 待处理请求: {} 个\n\
         • 活跃授权: {} 个\n\
         • 总记录数: {} 个",
        get_user_display_name(user),
        user_id,
        if is_super_admin { "超级管理员" } else { "管理员" },
        if has_password { "✅ 已设置" } else { "❌ 未设置" },
        admin.invite_code.as_deref().unwrap_or("❌ 未生成"),
        pending_count,
        active_count,
        total_count
    );

    bot.send_message(msg.chat.id, status_message).await?;

    Ok(())
}

/// 处理邀请码重新生成的回调
pub async fn handle_regenerate_invite_callback(
    bot: Bot,
    callback: CallbackQuery,
    state: BotState,
) -> Result<()> {
    let user = match callback.from.clone() {
        user => user,
    };

    let user_id = user.id.0 as i64;
    let user_service = state.user_service.read().await;

    if let Some(admin) = user_service.get_admin_info(user_id).await? {
        generate_new_invite_code(&bot, 
            callback.message.as_ref().unwrap().chat.id, 
            &user_service, 
            admin.unique_id).await?;
    }

    // 删除原消息
    if let Some(message) = callback.message {
        bot.delete_message(message.chat.id, message.id).await.ok();
    }

    bot.answer_callback_query(callback.id).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_parsing() {
        // 测试不同的撤销目标格式解析
        assert!("record123".starts_with("record"));
        assert!("r123".starts_with("r"));
        assert!("user456".starts_with("user"));
        assert!("u456".starts_with("u"));
        
        // 测试数字解析
        assert!("123456789".parse::<i64>().is_ok());
        assert!("invalid".parse::<i64>().is_err());
    }
}