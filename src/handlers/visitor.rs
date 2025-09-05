//! 访客命令处理器

use crate::bot::bot::BotState;
use crate::database::RecordRepository;
use crate::error::Result;
use crate::handlers::start::{get_user_display_name, validate_user_input};
use crate::types::{AuthStatus, AuthType, CallbackData, PasswordRequest, Record, UserRole};
use chrono::{Datelike, Timelike, Utc};
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
};

/// 处理/req命令 - 申请访客授权
pub async fn req_command(
    bot: Bot,
    msg: Message,
    cmd: crate::bot::bot::Command,
    state: BotState,
) -> Result<()> {
    // 从命令中提取邀请码
    let invite_code = match cmd {
        crate::bot::bot::Command::Req(code) => code,
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
    log::info!("用户 {} 申请访客授权，邀请码: {}", user_id, invite_code);

    // 验证邀请码格式
    if let Err(e) = validate_user_input(&invite_code, "invite_code") {
        bot.send_message(msg.chat.id, format!("❌ 邀请码格式错误: {}", e))
            .await?;
        return Ok(());
    }

    let user_service = state.user_service.read().await;

    // 检查用户是否已有待处理请求
    if RecordRepository::has_pending_request(state.database.pool(), user_id).await? {
        bot.send_message(
            msg.chat.id,
            "❌ 您已有待处理的授权请求\n请等待管理员处理或联系管理员取消之前的请求"
        ).await?;
        return Ok(());
    }

    // 检查用户是否已有活跃授权
    if RecordRepository::has_active_authorization(state.database.pool(), user_id).await? {
        bot.send_message(
            msg.chat.id,
            "❌ 您当前已有活跃的授权\n一个用户同时只能有一个活跃授权"
        ).await?;
        return Ok(());
    }

    // 验证邀请码并查找对应管理员
    let admin = match user_service.find_admin_by_invite_code(&invite_code).await? {
        Some(admin) => admin,
        None => {
            bot.send_message(
                msg.chat.id,
                "❌ 邀请码无效或已过期\n请联系管理员获取正确的邀请码"
            ).await?;
            return Ok(());
        }
    };

    // 创建访客记录
    let mut tx = state.database.begin_transaction().await?;
    let record = Record::new(user_id, admin.unique_id);
    let record_id = RecordRepository::create(&mut tx, &record).await?;
    tx.commit().await?;

    // 发送确认消息给访客
    let visitor_message = format!(
        "✅ 邀请码验证通过！\n\n\
         👤 邀请管理员：ID {} \n\
         📝 管理员已经收到了您的请求～请您等待批准！\n\n\
         🆔 您的申请ID：{}\n\
         ⏰ 申请时间：{}\n\n\
         💡 请耐心等待管理员审核",
        admin.id,
        record_id,
        Utc::now().format("%Y-%m-%d %H:%M:%S")
    );

    bot.send_message(msg.chat.id, visitor_message).await?;

    // 发送通知给管理员
    send_approval_request_to_admin(&bot, &admin, user, record_id, &state).await?;

    log::info!("用户 {} 成功创建授权请求，记录ID: {}", user_id, record_id);
    Ok(())
}

/// 处理/getpassword命令 - 获取密码
pub async fn get_password_command(bot: Bot, msg: Message, state: BotState) -> Result<()> {
    let user = match msg.from() {
        Some(user) => user,
        None => {
            bot.send_message(msg.chat.id, "无法获取用户信息").await?;
            return Ok(());
        }
    };

    let user_id = user.id.0 as i64;
    log::info!("用户 {} 请求获取密码", user_id);

    let user_service = state.user_service.read().await;
    let user_info = user_service.get_user_info(user).await?;

    // 管理员可以直接获取临时密码
    if matches!(user_info.role, UserRole::Admin | UserRole::SuperAdmin) {
        return handle_admin_get_password(&bot, msg, &state, user_info.telegram_id).await;
    }

    // 访客需要检查授权
    let active_records = RecordRepository::find_active_by_visitor(state.database.pool(), user_id).await?;

    if active_records.is_empty() {
        bot.send_message(
            msg.chat.id,
            "❌ 您当前没有活跃的授权\n\n\
             请先使用 /req <邀请码> 申请授权"
        ).await?;
        return Ok(());
    }

    // 处理访客密码请求
    for record in active_records {
        match generate_password_for_record(&bot, msg.chat.id, &record, &state).await {
            Ok(_) => {
                log::info!("为访客 {} 生成了 {:?} 类型的密码", user_id, record.auth_type);
                return Ok(());
            }
            Err(e) => {
                log::error!("为访客 {} 生成密码失败: {}", user_id, e);
                bot.send_message(msg.chat.id, format!("❌ 密码生成失败: {}", e))
                    .await?;
            }
        }
    }

    Ok(())
}

/// 处理管理员获取密码
async fn handle_admin_get_password(
    bot: &Bot,
    msg: Message,
    state: &BotState,
    admin_id: i64,
) -> Result<()> {
    let user_service = state.user_service.read().await;
    
    // 获取管理员信息
    let admin = match user_service.get_admin_info(admin_id).await? {
        Some(admin) => admin,
        None => {
            bot.send_message(msg.chat.id, "❌ 管理员信息不存在").await?;
            return Ok(());
        }
    };

    // 检查是否设置了密码
    if !user_service.admin_has_password(admin.unique_id).await? {
        bot.send_message(
            msg.chat.id,
            "❌ 请先设置管理密码！\n\n使用命令: /editpasswd <密码>"
        ).await?;
        return Ok(());
    }

    // 管理员获取临时密码
    let password_request = PasswordRequest {
        admin_password: admin.password.clone().unwrap_or_default(),
        auth_type: AuthType::Temp,
        times: None,
        hours: None,
        minutes: None,
        end_year: None,
        end_month: None,
        end_day: None,
        end_hour: None,
        start_time: None,
    };

    let mut password_service = state.password_service.write().await;
    match password_service.generate_password(&password_request, &state.config) {
        Ok(result) => {
            let message = format!(
                "🔑 管理员临时密码\n\n\
                 密码：<code>{}</code>\n\
                 过期时间：{}\n\
                 类型：{}\n\n\
                 💡 {}",
                result.password,
                result.expire_time,
                result.password_type,
                result.message
            );

            bot.send_message(msg.chat.id, message)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        }
        Err(e) => {
            bot.send_message(msg.chat.id, format!("❌ 密码生成失败: {}", e))
                .await?;
        }
    }

    Ok(())
}

/// 为记录生成密码
async fn generate_password_for_record(
    bot: &Bot,
    chat_id: ChatId,
    record: &Record,
    state: &BotState,
) -> Result<()> {
    let user_service = state.user_service.read().await;
    let admin = user_service.get_admin_info_by_unique_id(record.inviter).await?
        .ok_or_else(|| crate::error::AppError::business("管理员信息不存在"))?;

    let admin_password = admin.password
        .ok_or_else(|| crate::error::AppError::business("管理员未设置密码"))?;

    // 检查授权类型和特殊限制
    match record.auth_type {
        AuthType::LongtimeTemp => {
            let mut password_service = state.password_service.write().await;
            if !password_service.can_generate_longtime_temp(record.vis_id) {
                bot.send_message(
                    chat_id,
                    "❌ 长期临时密码在5分钟内只能获取一次\n请稍后再试"
                ).await?;
                return Ok(());
            }
            password_service.mark_longtime_temp_generated(record.vis_id);
        }
        _ => {
            // 对于其他类型，检查是否已经生成过密码
            let password_service = state.password_service.read().await;
            if let Some(existing_password) = password_service.has_generated_password(record.unique_id) {
                // 已经生成过密码，直接返回
                let type_description = match record.auth_type {
                    AuthType::Limited => "时效密码",
                    AuthType::Period => "指定过期时间密码",
                    AuthType::Times => "次数密码",
                    AuthType::Temp => "临时密码",
                    AuthType::LongtimeTemp => "长期临时密码",
                };

                let message = format!(
                    "🔑 您的访问密码\n\n\
                     密码：<code>{}</code>\n\
                     类型：{}\n\
                     过期时间：{}\n\n\
                     💡 此密码已生成，请妥善保管\n\n\
                     ⚠️ 请在有效期内使用，过期后需重新获取",
                    existing_password,
                    type_description,
                    record.ended_time.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or("未设置".to_string())
                );

                bot.send_message(chat_id, message)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
                return Ok(());
            }
        }
    }

    // 根据授权类型构建密码请求参数
    let (hours, minutes, end_year, end_month, end_day, end_hour) = match record.auth_type {
        AuthType::Limited => {
            // 从开始时间和结束时间计算小时数
            if let (Some(start), Some(end)) = (record.start_time, record.ended_time) {
                let duration = end.signed_duration_since(start);
                let total_minutes = duration.num_minutes();
                let hours = (total_minutes / 60) as u32;
                let minutes = (total_minutes % 60) as u32;
                (Some(hours), Some(minutes), None, None, None, None)
            } else {
                // 默认2小时
                (Some(2), Some(0), None, None, None, None)
            }
        },
        AuthType::Period => {
            // 从结束时间提取年月日时
            if let Some(end) = record.ended_time {
                (None, None, Some(end.year() as u32), Some(end.month()), Some(end.day()), Some(end.hour()))
            } else {
                return Err(crate::error::AppError::business("周期密码缺少结束时间"));
            }
        },
        _ => (None, None, None, None, None, None)
    };

    // 构建密码请求
    let password_request = PasswordRequest {
        admin_password,
        auth_type: record.auth_type,
        times: record.times.map(|t| t as u32),
        hours,
        minutes,
        end_year,
        end_month,
        end_day,
        end_hour,
        start_time: record.start_time,
    };

    // 生成密码
    let mut password_service = state.password_service.write().await;
    let result = password_service.generate_password(&password_request, &state.config)?;

    // 格式化消息
    let type_description = match record.auth_type {
        AuthType::Limited => "时效密码",
        AuthType::Period => "指定过期时间密码",
        AuthType::Times => "次数密码",
        AuthType::Temp => "临时密码",
        AuthType::LongtimeTemp => "长期临时密码",
    };

    let message = format!(
        "🔑 您的访问密码\n\n\
         密码：<code>{}</code>\n\
         类型：{}\n\
         过期时间：{}\n\n\
         💡 {}\n\n\
         ⚠️ 请在有效期内使用，过期后需重新获取",
        result.password,
        type_description,
        result.expire_time,
        result.message
    );

    bot.send_message(chat_id, message)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    // 将密码添加到记录中
    let mut tx = state.database.begin_transaction().await?;
    RecordRepository::add_password(&mut tx, record.unique_id, &result.password).await?;
    tx.commit().await?;

    // 对于非longtimetemp类型，缓存生成的密码
    if record.auth_type != AuthType::LongtimeTemp {
        let mut password_service = state.password_service.write().await;
        password_service.mark_password_generated(record.unique_id, result.password.clone());
    }

    Ok(())
}

/// 生成并发送密码（用于批准后立即推送）
pub async fn generate_and_send_password(
    _bot: &Bot,
    _chat_id: ChatId,
    record: &Record,
    state: &BotState,
) -> Result<String> {
    let user_service = state.user_service.read().await;
    let admin = user_service.get_admin_info_by_unique_id(record.inviter).await?
        .ok_or_else(|| crate::error::AppError::business("管理员信息不存在"))?;

    let admin_password = admin.password
        .ok_or_else(|| crate::error::AppError::business("管理员未设置密码"))?;

    // 根据授权类型构建密码请求参数
    let (hours, minutes, end_year, end_month, end_day, end_hour) = match record.auth_type {
        AuthType::Limited => {
            // 从开始时间和结束时间计算小时数
            if let (Some(start), Some(end)) = (record.start_time, record.ended_time) {
                let duration = end.signed_duration_since(start);
                let total_minutes = duration.num_minutes();
                let hours = (total_minutes / 60) as u32;
                let minutes = (total_minutes % 60) as u32;
                (Some(hours), Some(minutes), None, None, None, None)
            } else {
                // 默认2小时
                (Some(2), Some(0), None, None, None, None)
            }
        },
        AuthType::Period => {
            // 从结束时间提取年月日时
            if let Some(end) = record.ended_time {
                (None, None, Some(end.year() as u32), Some(end.month()), Some(end.day()), Some(end.hour()))
            } else {
                return Err(crate::error::AppError::business("周期密码缺少结束时间"));
            }
        },
        _ => (None, None, None, None, None, None)
    };

    // 构建密码请求
    let password_request = PasswordRequest {
        admin_password,
        auth_type: record.auth_type,
        times: record.times.map(|t| t as u32),
        hours,
        minutes,
        end_year,
        end_month,
        end_day,
        end_hour,
        start_time: record.start_time,
    };

    // 生成密码
    let mut password_service = state.password_service.write().await;
    let result = password_service.generate_password(&password_request, &state.config)?;

    // 将密码添加到记录中
    let mut tx = state.database.begin_transaction().await?;
    RecordRepository::add_password(&mut tx, record.unique_id, &result.password).await?;
    tx.commit().await?;

    Ok(result.password)
}

/// 发送审批请求给管理员
async fn send_approval_request_to_admin(
    bot: &Bot,
    admin: &crate::types::Admin,
    visitor: &teloxide::types::User,
    record_id: i64,
    _state: &BotState,
) -> Result<()> {
    let visitor_name = get_user_display_name(visitor);
    let current_time = Utc::now().format("%Y-%m-%d %H:%M:%S");

    // 创建内联键盘
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

    let message = format!(
        "📋 新的访客授权请求\n\n\
         👤 访客：{}\n\
         🆔 用户ID：{}\n\
         🕐 申请时间：{}\n\
         📝 记录ID：{}\n\n\
         ✅ 请您仔细核验访客身份后选择批准或拒绝",
        visitor_name,
        visitor.id.0,
        current_time,
        record_id
    );

    // 发送给管理员
    let admin_chat_id = ChatId(admin.id);
    bot.send_message(admin_chat_id, message)
        .reply_markup(keyboard)
        .await
        .map_err(|e| {
            log::warn!("发送管理员通知失败: {}", e);
            e
        })?;

    Ok(())
}

/// 处理授权批准回调
pub async fn handle_approve_callback(
    bot: Bot,
    callback: CallbackQuery,
    record_id: i64,
    state: BotState,
) -> Result<()> {
    let admin = callback.from.clone();
    let admin_id = admin.id.0 as i64;

    log::info!("管理员 {} 批准授权请求 {}", admin_id, record_id);

    // 检查权限
    let user_service = state.user_service.read().await;
    if !user_service.is_admin(admin_id).await? {
        bot.answer_callback_query(callback.id)
            .text("❌ 权限不足")
            .await?;
        return Ok(());
    }

    // 检查管理员是否设置密码
    let admin_info = user_service.get_admin_info(admin_id).await?
        .ok_or_else(|| crate::error::AppError::business("管理员信息不存在"))?;

    if !user_service.admin_has_password(admin_info.unique_id).await? {
        bot.answer_callback_query(callback.id)
            .text("❌ 请先设置管理密码")
            .await?;
        return Ok(());
    }

    // 获取记录信息
    let record = RecordRepository::find_by_id(state.database.pool(), record_id).await?
        .ok_or_else(|| crate::error::AppError::business("授权记录不存在"))?;

    if record.status != AuthStatus::Pending {
        bot.answer_callback_query(callback.id)
            .text("❌ 该请求已被处理")
            .await?;
        return Ok(());
    }

    // 创建授权类型选择键盘
    let keyboard = create_auth_type_keyboard(record_id);

    let message = format!(
        "✅ 请选择授权类型：\n\n\
         📋 授权类型说明：\n\
         • 时效密码：指定有效时长\n\
         • 指定过期时间：指定具体过期时间\n\
         • 次数密码：2小时内限定使用次数\n\
         • 临时密码：10分钟有效期\n\
         • 长期临时密码：管理员指定有效期"
    );

    // 编辑原消息
    if let Some(message_to_edit) = callback.message {
        bot.edit_message_text(message_to_edit.chat.id, message_to_edit.id, message)
            .reply_markup(keyboard)
            .await?;
    }

    bot.answer_callback_query(callback.id).await?;
    Ok(())
}

/// 处理授权拒绝回调
pub async fn handle_reject_callback(
    bot: Bot,
    callback: CallbackQuery,
    record_id: i64,
    state: BotState,
) -> Result<()> {
    let admin = callback.from.clone();
    let admin_id = admin.id.0 as i64;

    log::info!("管理员 {} 拒绝授权请求 {}", admin_id, record_id);

    // 更新记录状态为撤销
    let mut tx = state.database.begin_transaction().await?;
    RecordRepository::update_status(&mut tx, record_id, AuthStatus::Revoked).await?;
    tx.commit().await?;

    // 获取记录信息通知访客
    if let Some(record) = RecordRepository::find_by_id(state.database.pool(), record_id).await? {
        let visitor_chat_id = ChatId(record.vis_id);
        bot.send_message(
            visitor_chat_id,
            "❌ 您的访客授权请求已被拒绝\n\n如有疑问请联系管理员"
        ).await.ok(); // 忽略发送失败
    }

    // 编辑管理员消息
    if let Some(message) = callback.message {
        let updated_message = format!(
            "❌ 授权请求已拒绝\n\n📝 记录ID：{}\n⏰ 处理时间：{}",
            record_id,
            Utc::now().format("%Y-%m-%d %H:%M:%S")
        );

        bot.edit_message_text(message.chat.id, message.id, updated_message)
            .await?;
    }

    bot.answer_callback_query(callback.id)
        .text("✅ 已拒绝该请求")
        .await?;

    Ok(())
}

/// 创建授权类型选择键盘
pub fn create_auth_type_keyboard(record_id: i64) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback(
                "返回上一步",
                CallbackData::with_data("back_to_approve", &record_id.to_string()).to_callback_string().unwrap()
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                "⏰ 时效密码",
                CallbackData::with_data("auth_limited", &record_id.to_string()).to_callback_string().unwrap()
            ),
            InlineKeyboardButton::callback(
                "📅 指定过期时间",
                CallbackData::with_data("auth_period", &record_id.to_string()).to_callback_string().unwrap()
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                "🔢 次数密码",
                CallbackData::with_data("auth_times", &record_id.to_string()).to_callback_string().unwrap()
            ),
            InlineKeyboardButton::callback(
                "⚡ 临时密码",
                CallbackData::with_data("auth_temp", &record_id.to_string()).to_callback_string().unwrap()
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                "🔄 长期临时密码",
                CallbackData::with_data("auth_longtime_temp", &record_id.to_string()).to_callback_string().unwrap()
            ),
        ],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_type_descriptions() {
        let descriptions = [
            (AuthType::Limited, "时效密码"),
            (AuthType::Period, "指定过期时间密码"),
            (AuthType::Times, "次数密码"),
            (AuthType::Temp, "临时密码"),
            (AuthType::LongtimeTemp, "长期临时密码"),
        ];

        for (auth_type, expected) in descriptions.iter() {
            assert_eq!(auth_type.description(), *expected);
        }
    }
}