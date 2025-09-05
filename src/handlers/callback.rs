//! 回调查询处理器

use crate::bot::bot::BotState;
use crate::database::RecordRepository;
use crate::error::Result;
use crate::handlers::{admin::handle_regenerate_invite_callback, visitor::{handle_approve_callback, handle_reject_callback}};
use crate::types::{AuthType, CallbackData};
use chrono::{Utc, FixedOffset};
use teloxide::{prelude::*, types::InlineKeyboardButton, types::InlineKeyboardMarkup};

/// 格式化为UTC+8时间字符串
fn format_beijing_time(timestamp: chrono::DateTime<Utc>) -> String {
    let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
    timestamp.with_timezone(&beijing_tz).format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 获取当前UTC+8时间字符串
fn current_beijing_time() -> String {
    format_beijing_time(Utc::now())
}

/// 处理回调查询
pub async fn handle_callback_query(
    bot: Bot,
    callback: CallbackQuery,
    state: BotState,
) -> Result<()> {
    let callback_data = match &callback.data {
        Some(data) => data,
        None => {
            bot.answer_callback_query(callback.id).await?;
            return Ok(());
        }
    };

    log::info!("收到回调查询: {}", callback_data);

    // 解析回调数据
    let parsed_data = match CallbackData::from_str(callback_data) {
        Ok(data) => data,
        Err(_) => {
            bot.answer_callback_query(callback.id)
                .text("❌ 无效的回调数据")
                .await?;
            return Ok(());
        }
    };

    // 根据动作类型分发处理
    match parsed_data.action.as_str() {
        // 管理员相关回调
        "regenerate_invite" => {
            handle_regenerate_invite_callback(bot, callback, state).await?;
        }
        
        // 访客授权相关回调
        "approve" => {
            let record_id = parse_record_id(&parsed_data)?;
            handle_approve_callback(bot, callback, record_id, state).await?;
        }
        
        "reject" => {
            let record_id = parse_record_id(&parsed_data)?;
            handle_reject_callback(bot, callback, record_id, state).await?;
        }

        // 授权类型选择回调
        "auth_temp" => {
            let record_id = parse_record_id(&parsed_data)?;
            handle_auth_temp_selection(bot, callback, record_id, state).await?;
        }

        "auth_times" => {
            let record_id = parse_record_id(&parsed_data)?;
            handle_auth_times_selection(bot, callback, record_id, state).await?;
        }

        "auth_limited" => {
            let record_id = parse_record_id(&parsed_data)?;
            handle_auth_limited_selection(bot, callback, record_id, state).await?;
        }

        "auth_period" => {
            let record_id = parse_record_id(&parsed_data)?;
            handle_auth_period_selection(bot, callback, record_id, state).await?;
        }

        "auth_longtime_temp" => {
            let record_id = parse_record_id(&parsed_data)?;
            handle_auth_longtime_temp_selection(bot, callback, record_id, state).await?;
        }

        // 确认回调
        "confirm_times" => {
            if let Some(data) = &parsed_data.data {
                let parts: Vec<&str> = data.split(':').collect();
                if parts.len() == 2 {
                    if let (Ok(record_id), Ok(times)) = (parts[0].parse::<i64>(), parts[1].parse::<i32>()) {
                        handle_confirm_times_callback(bot, callback, record_id, times, state).await?;
                    }
                }
            }
        }

        "confirm_limited" => {
            if let Some(data) = &parsed_data.data {
                let parts: Vec<&str> = data.split(':').collect();
                if parts.len() == 3 {
                    if let (Ok(record_id), Ok(hours), Ok(minutes)) = (
                        parts[0].parse::<i64>(),
                        parts[1].parse::<u32>(),
                        parts[2].parse::<u32>()
                    ) {
                        handle_confirm_limited_callback(bot, callback, record_id, hours, minutes, state).await?;
                    }
                }
            }
        }

        // 返回操作
        "back_to_approve" => {
            let record_id = parse_record_id(&parsed_data)?;
            handle_back_to_approve(bot, callback, record_id, state).await?;
        }

        // 取消操作
        "cancel" => {
            handle_cancel_callback(bot, callback).await?;
        }

        // 未知动作
        _ => {
            bot.answer_callback_query(callback.id)
                .text("❌ 未知操作")
                .await?;
            log::warn!("未知回调动作: {}", parsed_data.action);
        }
    }

    Ok(())
}

/// 解析记录ID
fn parse_record_id(callback_data: &CallbackData) -> Result<i64> {
    callback_data
        .data
        .as_ref()
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| crate::error::AppError::validation("无效的记录ID"))
}

/// 处理临时密码选择
async fn handle_auth_temp_selection(
    bot: Bot,
    callback: CallbackQuery,
    record_id: i64,
    state: BotState,
) -> Result<()> {
    log::info!("管理员选择临时密码授权，记录ID: {}", record_id);

    // 直接批准临时密码授权
    let start_time = Some(Utc::now());
    let end_time = Some(Utc::now() + chrono::Duration::minutes(10));
    
    let mut tx = state.database.begin_transaction().await?;
    let success = RecordRepository::approve_authorization(
        &mut tx,
        record_id,
        AuthType::Temp,
        start_time,
        end_time,
        None,
    ).await?;
    tx.commit().await?;

    if success {
        // 立即为临时密码生成密码并推送给访客
        if let Some(record) = RecordRepository::find_by_id(state.database.pool(), record_id).await? {
            let visitor_chat_id = ChatId(record.vis_id);
            
            // 生成并推送密码
            match crate::handlers::visitor::generate_and_send_password(&bot, visitor_chat_id, &record, &state).await {
                Ok(password) => {
                    bot.send_message(
                        visitor_chat_id,
                        format!(
                            "✅ 您的授权已被批准！\n\n\
                             📋 授权类型：临时密码\n\
                             ⏰ 有效期：10分钟\n\
                             🆔 批准ID：{}\n\
                             📅 过期时间：{}\n\
                             🔑 密码：<code>{}</code>\n\n\
                             💡 密码已自动生成，请妥善保管\n\
                             ⚠️ 此密码10分钟后过期，请及时使用",
                             record_id,
                             record.ended_time.map(|t| format_beijing_time(t))
                                 .unwrap_or("未设置".to_string()),
                             password
                        )
                    ).parse_mode(teloxide::types::ParseMode::Html).await.ok();
                }
                Err(e) => {
                    log::error!("为访客 {} 生成临时密码失败: {}", record.vis_id, e);
                    bot.send_message(
                        visitor_chat_id,
                        format!(
                            "✅ 您的授权已被批准！\n\n\
                             📋 授权类型：临时密码\n\
                             ⏰ 有效期：10分钟\n\
                             🆔 批准ID：{}\n\
                             📅 过期时间：{}\n\n\
                             ❗ 密码生成遇到问题，请使用 /getpassword 获取密码\n\
                             💡 如多次获取失败，请联系管理员",
                             record_id,
                             record.ended_time.map(|t| format_beijing_time(t))
                                 .unwrap_or("未设置".to_string())
                        )
                    ).await.ok();
                }
            }
        }

        // 更新管理员消息
        if let Some(message) = callback.message {
            let updated_message = format!(
                "✅ 授权已批准\n\n\
                 📋 授权类型：临时密码\n\
                 ⏰ 有效期：10分钟\n\
                 🆔 记录ID：{}\n\
                 🕐 处理时间：{}",
                record_id,
                current_beijing_time()
            );

            bot.edit_message_text(message.chat.id, message.id, updated_message).await?;
        }

        bot.answer_callback_query(callback.id)
            .text("✅ 临时密码授权已批准")
            .await?;
    } else {
        bot.answer_callback_query(callback.id)
            .text("❌ 授权失败")
            .await?;
    }

    Ok(())
}

/// 处理次数密码选择
async fn handle_auth_times_selection(
    bot: Bot,
    callback: CallbackQuery,
    record_id: i64,
    _state: BotState,
) -> Result<()> {
    // 创建次数选择键盘
    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("1次", 
                CallbackData::with_data("confirm_times", &format!("{}:1", record_id)).to_callback_string().unwrap()),
            InlineKeyboardButton::callback("3次",
                CallbackData::with_data("confirm_times", &format!("{}:3", record_id)).to_callback_string().unwrap()),
            InlineKeyboardButton::callback("5次",
                CallbackData::with_data("confirm_times", &format!("{}:5", record_id)).to_callback_string().unwrap()),
        ],
        vec![
            InlineKeyboardButton::callback("10次", 
                CallbackData::with_data("confirm_times", &format!("{}:10", record_id)).to_callback_string().unwrap()),
            InlineKeyboardButton::callback("20次",
                CallbackData::with_data("confirm_times", &format!("{}:20", record_id)).to_callback_string().unwrap()),
            InlineKeyboardButton::callback("31次",
                CallbackData::with_data("confirm_times", &format!("{}:31", record_id)).to_callback_string().unwrap()),
        ],
        vec![
            InlineKeyboardButton::callback("返回", 
                CallbackData::with_data("back_to_approve", &record_id.to_string()).to_callback_string().unwrap()),
        ],
    ]);

    if let Some(message) = callback.message {
        bot.edit_message_text(
            message.chat.id,
            message.id,
            "🔢 请选择使用次数（2小时有效期）："
        )
        .reply_markup(keyboard)
        .await?;
    }

    bot.answer_callback_query(callback.id).await?;
    Ok(())
}

/// 处理时效密码选择
async fn handle_auth_limited_selection(
    bot: Bot,
    callback: CallbackQuery,
    record_id: i64,
    _state: BotState,
) -> Result<()> {
    // 创建时长选择键盘
    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("1小时", 
                CallbackData::with_data("confirm_limited", &format!("{}:1:0", record_id)).to_callback_string().unwrap()),
            InlineKeyboardButton::callback("2小时",
                CallbackData::with_data("confirm_limited", &format!("{}:2:0", record_id)).to_callback_string().unwrap()),
            InlineKeyboardButton::callback("4小时",
                CallbackData::with_data("confirm_limited", &format!("{}:4:0", record_id)).to_callback_string().unwrap()),
        ],
        vec![
            InlineKeyboardButton::callback("12小时", 
                CallbackData::with_data("confirm_limited", &format!("{}:12:0", record_id)).to_callback_string().unwrap()),
            InlineKeyboardButton::callback("24小时",
                CallbackData::with_data("confirm_limited", &format!("{}:24:0", record_id)).to_callback_string().unwrap()),
            InlineKeyboardButton::callback("48小时",
                CallbackData::with_data("confirm_limited", &format!("{}:48:0", record_id)).to_callback_string().unwrap()),
        ],
        vec![
            InlineKeyboardButton::callback("返回", 
                CallbackData::with_data("back_to_approve", &record_id.to_string()).to_callback_string().unwrap()),
        ],
    ]);

    if let Some(message) = callback.message {
        bot.edit_message_text(
            message.chat.id,
            message.id,
            "⏰ 请选择有效时长："
        )
        .reply_markup(keyboard)
        .await?;
    }

    bot.answer_callback_query(callback.id).await?;
    Ok(())
}

/// 处理指定时间密码选择
async fn handle_auth_period_selection(
    bot: Bot,
    callback: CallbackQuery,
    _record_id: i64,
    _state: BotState,
) -> Result<()> {
    if let Some(message) = callback.message {
        bot.edit_message_text(
            message.chat.id,
            message.id,
            format!("📅 指定过期时间密码需要管理员输入具体时间\n\n\
             请发送消息格式：\n\
             <code>期间 {} YYYY-MM-DD HH</code>\n\n\
             例如：<code>期间 {} 2024-12-25 18</code>\n\
             (表示2024年12月25日18点过期)", _record_id, _record_id)
        )
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    }

    bot.answer_callback_query(callback.id).await?;
    Ok(())
}

/// 处理长期临时密码选择
async fn handle_auth_longtime_temp_selection(
    bot: Bot,
    callback: CallbackQuery,
    _record_id: i64,
    _state: BotState,
) -> Result<()> {
    if let Some(message) = callback.message {
        bot.edit_message_text(
            message.chat.id,
            message.id,
            format!("🔄 长期临时密码需要管理员指定结束时间\n\n\
             请发送消息格式：\n\
             <code>长期 {} YYYY-MM-DD HH:MM</code>\n\n\
             例如：<code>长期 {} 2024-12-31 23:59</code>\n\
             (表示在此时间前可重复获取临时密码)", _record_id, _record_id)
        )
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    }

    bot.answer_callback_query(callback.id).await?;
    Ok(())
}

/// 处理返回批准选择
async fn handle_back_to_approve(
    bot: Bot,
    callback: CallbackQuery,
    record_id: i64,
    state: BotState,
) -> Result<()> {
    // 获取记录信息以显示用户详情
    let record = RecordRepository::find_by_id(state.database.pool(), record_id).await?
        .ok_or_else(|| crate::error::AppError::business("授权记录不存在"))?;

    // 获取用户服务以查找管理员信息
    let user_service = state.user_service.read().await;
    let admin = user_service.get_admin_info_by_unique_id(record.inviter).await?
        .ok_or_else(|| crate::error::AppError::business("管理员信息不存在"))?;

    // 创建原始的批准/拒绝键盘
    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback(
                "✅ 批准",
                CallbackData::with_data("approve", &record_id.to_string()).to_callback_string().unwrap()
            ),
            InlineKeyboardButton::callback(
                "❌ 拒绝",
                CallbackData::with_data("reject", &record_id.to_string()).to_callback_string().unwrap()
            ),
        ]
    ]);

    // 重新显示原始的批准请求消息
    let message = format!(
        "📋 访客授权请求\n\n\
         👤 用户ID：{}\n\
         🕐 申请时间：{}\n\
         📝 记录ID：{}\n\
         👨‍💼 管理员ID：{}\n\n\
         请您仔细核验后批准",
        record.vis_id,
        format_beijing_time(record.update_at),
        record_id,
        admin.id
    );

    if let Some(message_to_edit) = callback.message {
        bot.edit_message_text(message_to_edit.chat.id, message_to_edit.id, message)
        .reply_markup(keyboard)
        .await?;
    }

    bot.answer_callback_query(callback.id)
        .text("↩️ 已返回批准选择")
        .await?;
    Ok(())
}

/// 处理取消操作
async fn handle_cancel_callback(bot: Bot, callback: CallbackQuery) -> Result<()> {
    if let Some(message) = callback.message {
        bot.delete_message(message.chat.id, message.id).await.ok();
    }

    bot.answer_callback_query(callback.id)
        .text("✅ 操作已取消")
        .await?;

    Ok(())
}

/// 确认次数密码授权
pub async fn handle_confirm_times_callback(
    bot: Bot,
    callback: CallbackQuery,
    record_id: i64,
    times: i32,
    state: BotState,
) -> Result<()> {
    let start_time = Some(Utc::now());
    let end_time = Some(Utc::now() + chrono::Duration::hours(2));

    let mut tx = state.database.begin_transaction().await?;
    let success = RecordRepository::approve_authorization(
        &mut tx,
        record_id,
        AuthType::Times,
        start_time,
        end_time,
        Some(times),
    ).await?;
    tx.commit().await?;

    if success {
        // 立即为次数密码生成密码并推送给访客
        if let Some(record) = RecordRepository::find_by_id(state.database.pool(), record_id).await? {
            let visitor_chat_id = ChatId(record.vis_id);
            
            // 生成并推送密码
            match crate::handlers::visitor::generate_and_send_password(&bot, visitor_chat_id, &record, &state).await {
                Ok(password) => {
                    bot.send_message(
                        visitor_chat_id,
                        format!(
                            "✅ 您的授权已被批准！\n\n\
                             📋 授权类型：次数密码\n\
                             🔢 可用次数：{} 次\n\
                             ⏰ 有效期：2小时\n\
                             🆔 批准ID：{}\n\
                             📅 过期时间：{}\n\
                             🔑 密码：<code>{}</code>\n\n\
                             💡 密码已自动生成，请妥善保管\n\
                             ⚠️ 每次使用都会消耗一次使用次数",
                             times,
                             record_id,
                             record.ended_time.map(|t| format_beijing_time(t))
                                 .unwrap_or("未设置".to_string()),
                             password
                        )
                    ).parse_mode(teloxide::types::ParseMode::Html).await.ok();
                }
                Err(e) => {
                    log::error!("为访客 {} 生成次数密码失败: {}", record.vis_id, e);
                    bot.send_message(
                        visitor_chat_id,
                        format!(
                            "✅ 您的授权已被批准！\n\n\
                             📋 授权类型：次数密码\n\
                             🔢 可用次数：{} 次\n\
                             ⏰ 有效期：2小时\n\
                             🆔 批准ID：{}\n\
                             📅 过期时间：{}\n\n\
                             ❗ 密码生成遇到问题，请使用 /getpassword 获取密码\n\
                             💡 如多次获取失败，请联系管理员",
                             times,
                             record_id,
                             record.ended_time.map(|t| format_beijing_time(t))
                                 .unwrap_or("未设置".to_string())
                        )
                    ).await.ok();
                }
            }
        }

        // 更新管理员消息
        if let Some(message) = callback.message {
            let updated_message = format!(
                "✅ 授权已批准\n\n\
                 📋 授权类型：次数密码\n\
                 🔢 使用次数：{} 次\n\
                 ⏰ 有效期：2小时\n\
                 🆔 记录ID：{}\n\
                 🕐 处理时间：{}",
                times,
                record_id,
                current_beijing_time()
            );

            bot.edit_message_text(message.chat.id, message.id, updated_message).await?;
        }

        bot.answer_callback_query(callback.id)
            .text(&format!("✅ {} 次密码授权已批准", times))
            .await?;
    } else {
        bot.answer_callback_query(callback.id)
            .text("❌ 授权失败")
            .await?;
    }

    Ok(())
}

/// 确认时效密码授权
pub async fn handle_confirm_limited_callback(
    bot: Bot,
    callback: CallbackQuery,
    record_id: i64,
    hours: u32,
    minutes: u32,
    state: BotState,
) -> Result<()> {
    let start_time = Some(Utc::now());
    let duration = chrono::Duration::hours(hours as i64) + chrono::Duration::minutes(minutes as i64);
    let end_time = Some(Utc::now() + duration);

    let mut tx = state.database.begin_transaction().await?;
    let success = RecordRepository::approve_authorization(
        &mut tx,
        record_id,
        AuthType::Limited,
        start_time,
        end_time,
        None,
    ).await?;
    tx.commit().await?;

    if success {
        let duration_str = if minutes == 0 {
            format!("{}小时", hours)
        } else {
            format!("{}小时{}分钟", hours, minutes)
        };

        // 立即为时效密码生成密码并推送给访客
        if let Some(record) = RecordRepository::find_by_id(state.database.pool(), record_id).await? {
            let visitor_chat_id = ChatId(record.vis_id);
            
            // 生成并推送密码
            match crate::handlers::visitor::generate_and_send_password(&bot, visitor_chat_id, &record, &state).await {
                Ok(password) => {
                    bot.send_message(
                        visitor_chat_id,
                        format!(
                            "✅ 您的授权已被批准！\n\n\
                             📋 授权类型：时效密码\n\
                             ⏰ 有效时长：{}\n\
                             📅 过期时间：{}\n\
                             🆔 批准ID：{}\n\
                             🔑 密码：<code>{}</code>\n\n\
                             💡 密码已自动生成，请妥善保管\n\
                             ⚠️ 密码在有效时长内可重复使用",
                             duration_str,
                             format_beijing_time(end_time.unwrap()),
                             record_id,
                             password
                        )
                    ).parse_mode(teloxide::types::ParseMode::Html).await.ok();
                }
                Err(e) => {
                    log::error!("为访客 {} 生成时效密码失败: {}", record.vis_id, e);
                    bot.send_message(
                        visitor_chat_id,
                        format!(
                            "✅ 您的授权已被批准！\n\n\
                             📋 授权类型：时效密码\n\
                             ⏰ 有效时长：{}\n\
                             📅 过期时间：{}\n\
                             🆔 批准ID：{}\n\n\
                             ❗ 密码生成遇到问题，请使用 /getpassword 获取密码\n\
                             💡 如多次获取失败，请联系管理员",
                             duration_str,
                             format_beijing_time(end_time.unwrap()),
                             record_id
                        )
                    ).await.ok();
                }
            }
        }

        // 更新管理员消息
        if let Some(message) = callback.message {
            let updated_message = format!(
                "✅ 授权已批准\n\n\
                 📋 授权类型：时效密码\n\
                 ⏰ 有效时长：{}\n\
                 📅 过期时间：{}\n\
                 🆔 记录ID：{}\n\
                 🕐 处理时间：{}",
                duration_str,
                format_beijing_time(end_time.unwrap()),
                record_id,
                current_beijing_time()
            );

            bot.edit_message_text(message.chat.id, message.id, updated_message).await?;
        }

        bot.answer_callback_query(callback.id)
            .text(&format!("✅ {} 时效密码授权已批准", duration_str))
            .await?;
    } else {
        bot.answer_callback_query(callback.id)
            .text("❌ 授权失败")
            .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_record_id() {
        let data = CallbackData::with_data("test", "123");
        assert_eq!(parse_record_id(&data).unwrap(), 123);

        let data = CallbackData::new("test");
        assert!(parse_record_id(&data).is_err());
    }

    #[test]
    fn test_callback_data_parsing() {
        let data = r#"{"action":"approve","data":"123"}"#;
        let parsed = CallbackData::from_str(data).unwrap();
        assert_eq!(parsed.action, "approve");
        assert_eq!(parsed.data, Some("123".to_string()));
    }
}