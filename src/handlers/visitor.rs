//! 访客命令处理器

use crate::bot::bot::BotState;
use crate::database::RecordRepository;
use crate::error::Result;
use crate::handlers::start::{get_user_display_name, validate_user_input};
use crate::types::{AuthStatus, AuthType, CallbackData, PasswordRequest, Record, UserRole};
use chrono::{Datelike, Timelike, Utc, FixedOffset};
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
};

/// 格式化为UTC+8时间字符串
fn format_beijing_time(timestamp: chrono::DateTime<Utc>) -> String {
    let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
    timestamp.with_timezone(&beijing_tz).format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 获取当前UTC+8时间字符串
fn current_beijing_time() -> String {
    format_beijing_time(Utc::now())
}

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
            "❌ 您已有待处理的授权请求\n\n\
             请等待管理员处理或联系管理员取消之前的请求\n\
             💡 如果您的请求被拒绝，可以重新申请\n\
             ⏰ 如果长时间无响应，请联系管理员确认"
        ).await?;
        return Ok(());
    }

    // 检查用户是否已有活跃授权
    if RecordRepository::has_active_authorization(state.database.pool(), user_id).await? {
        bot.send_message(
            msg.chat.id,
            "❌ 您当前已有活跃的授权\n\n\
             一个用户同时只能有一个活跃授权\n\
             💡 如需申请新的授权，请联系管理员撤销当前授权\n\
             📋 您可以使用 /getpassword 获取当前授权的密码"
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
        current_beijing_time()
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
        // 检查是否有待处理的请求
        if RecordRepository::has_pending_request(state.database.pool(), user_id).await? {
            bot.send_message(
                msg.chat.id,
                "⏳ 您的授权请求正在等待管理员处理\n\n\
                 请耐心等待管理员审核您的请求\n\
                 💡 如果申请超过24小时无响应，建议：\n\
                 • 联系邀请您的管理员确认\n\
                 • 确认邀请码是否有效\n\
                 • 检查是否遗漏管理员的回复消息"
            ).await?;
        } else {
            bot.send_message(
                msg.chat.id,
                "❌ 您当前没有活跃的授权\n\n\
                 请先使用 /req <邀请码> 申请授权\n\
                 \n\
                 💡 获取邀请码的方式：\n\
                 • 联系管理员获取邀请码\n\
                 • 确认邀请码格式正确\n\
                 • 如曾被拒绝，可重新申请"
            ).await?;
        }
        return Ok(());
    }

    // 处理访客密码请求 - 添加详细的过期检查
    let mut password_generated = false;
    let mut last_error = None;
    let mut expired_count = 0;
    
    for record in active_records {
        // 双重检查记录是否确实活跃
        if !record.is_active() {
            expired_count += 1;
            log::warn!("记录 {} 被标记为活跃但实际已过期", record.unique_id);
            continue;
        }
        
        match generate_password_for_record(&bot, msg.chat.id, &record, &state).await {
            Ok(_) => {
                log::info!("为访客 {} 生成了 {:?} 类型的密码", user_id, record.auth_type);
                password_generated = true;
                break;
            }
            Err(e) => {
                log::error!("为访客 {} 生成密码失败: {}", user_id, e);
                last_error = Some(e);
            }
        }
    }
    
    // 如果所有记录都已过期，发送特殊的过期消息
    if expired_count > 0 && !password_generated {
        bot.send_message(
            msg.chat.id,
            "❌ 您的授权已过期\n\n\
             📅 所有活跃授权都已超过有效期\n\
             💡 请重新申请授权：\n\
             • 联系管理员获取新的邀请码\n\
             • 使用 /req <邀请码> 重新申请\n\
             • 如有疑问请联系管理员确认"
        ).await?;
        return Ok(());
    }
    
    // 如果所有记录都生成失败，发送错误信息
    if !password_generated {
        if let Some(error) = last_error {
            let error_msg = error.to_string();
            
            // 根据错误类型提供更具体的建议
            let (title, solutions) = if error_msg.contains("已过期") || error_msg.contains("结束时间必须晚于当前时间") {
                (
                    "❌ 授权已过期",
                    "📅 您的访问授权已超过有效期\n\n\
                     💡 解决方案：\n\
                     • 联系管理员重新申请授权\n\
                     • 获取新的邀请码后使用 /req <邀请码> 申请\n\
                     • 如有疑问请联系邀请您的管理员"
                )
            } else if error_msg.contains("密码生成错误") {
                (
                    "❌ 密码生成失败",
                    "🔧 技术问题导致密码无法生成\n\n\
                     💡 解决方案：\n\
                     • 等待几分钟后重试\n\
                     • 联系管理员确认授权状态\n\
                     • 如持续失败请联系技术支持"
                )
            } else {
                (
                    "❌ 密码获取失败",
                    "💡 可能的解决方案：\n\
                     • 等待几分钟后重试\n\
                     • 检查您的授权是否有效\n\
                     • 联系管理员确认账户状态\n\
                     • 如多次失败，请联系技术支持"
                )
            };
            
            bot.send_message(
                msg.chat.id,
                format!("{}\n\n错误详情：{}\n\n{}", title, error_msg, solutions)
            ).await?;
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
    // 首先检查授权是否已过期 - 更详细的检查
    if !record.is_active() {
        let expire_info = if let Some(ended_time) = record.ended_time {
            let beijing_tz = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
            let ended_time_beijing = ended_time.with_timezone(&beijing_tz);
            let current_time_beijing = chrono::Utc::now().with_timezone(&beijing_tz);
            format!("授权已于 {} 过期（当前时间：{}）",
                   ended_time_beijing.format("%Y-%m-%d %H:%M:%S"),
                   current_time_beijing.format("%Y-%m-%d %H:%M:%S"))
        } else {
            "授权已过期".to_string()
        };
        return Err(crate::error::AppError::business(&expire_info));
    }

    let user_service = state.user_service.read().await;
    let admin = user_service.get_admin_info_by_unique_id(record.inviter).await?
        .ok_or_else(|| crate::error::AppError::business("管理员信息不存在"))?;

    let admin_password = admin.password
        .ok_or_else(|| crate::error::AppError::business("管理员未设置密码"))?;
    drop(user_service); // 释放用户服务锁

    // 统一处理所有授权类型的限制检查和密码生成
    let mut password_service = state.password_service.write().await;
    
    match record.auth_type {
        AuthType::LongtimeTemp => {
            // 长期临时密码：检查5分钟限制
            if !password_service.can_generate_longtime_temp(record.vis_id) {
                drop(password_service); // 释放锁
                bot.send_message(
                    chat_id,
                    "❌ 长期临时密码获取限制\n\n\
                     ⏰ 每5分钟只能获取一次新密码\n\
                     💡 这是为了安全考虑的限制\n\n\
                     请等待后重试，或使用现有密码：\n\
                     • 如果密码已过期，请等待5分钟\n\
                     • 如果忘记密码，请联系管理员\n\
                     • 每个密码有效期为10分钟"
                ).await?;
                return Ok(());
            }
            password_service.mark_longtime_temp_generated(record.vis_id);
        }
        _ => {
            // 其他类型：检查数据库中是否已生成过密码，如果是则阻止重复生成
            if let Some(existing_password) = password_service.has_generated_password(state.database.pool(), record.unique_id).await? {
                drop(password_service); // 释放锁
                
                // 发送阻止消息，不再返回密码
                let type_description = match record.auth_type {
                    AuthType::Limited => "时效密码",
                    AuthType::Period => "指定过期时间密码",
                    AuthType::Times => "次数密码",
                    AuthType::Temp => "临时密码",
                    AuthType::LongtimeTemp => "长期临时密码",
                };

                let message = format!(
                    "❌ 密码获取限制\n\n\
                     📋 授权类型：{}\n\
                     🔒 此授权类型的密码只能生成一次\n\
                     📱 您已经获取过此授权的密码\n\n\
                     💡 建议操作：\n\
                     • 查看之前收到的密码消息\n\
                     • 如果密码丢失，请联系管理员\n\
                     • 如需新密码，请重新申请授权\n\n\
                     🔐 当前密码：<code>{}</code>\n\
                     ⚠️ 请妥善保管，避免重复获取",
                    type_description,
                    existing_password
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
            // 从结束时间提取年月日时 - 需要转换为UTC+8时区
            if let Some(end) = record.ended_time {
                // 将数据库中的UTC时间转换为UTC+8时区，然后提取时间组件
                let beijing_tz = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
                let end_beijing = end.with_timezone(&beijing_tz);
                (None, None, Some(end_beijing.year() as u32), Some(end_beijing.month()), Some(end_beijing.day()), Some(end_beijing.hour()))
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

    // 生成密码 (使用已获取的password_service锁)
    let result = password_service.generate_password(&password_request, &state.config)?;

    // 格式化消息
    let type_description = match record.auth_type {
        AuthType::Limited => "时效密码",
        AuthType::Period => "指定过期时间密码",
        AuthType::Times => "次数密码",
        AuthType::Temp => "临时密码",
        AuthType::LongtimeTemp => "长期临时密码",
    };

    let usage_tips = match record.auth_type {
        AuthType::Times => format!(
            "📊 使用说明：\n\
             • 可用次数：{} 次\n\
             • 每次使用会消耗一次机会\n\
             • 剩余次数请注意合理使用",
            record.times.unwrap_or(1)
        ),
        AuthType::Limited => format!(
            "⏰ 使用说明：\n\
             • 在有效期内可重复使用\n\
             • 请在过期前完成所需操作\n\
             • 过期后需重新申请授权"
        ),
        AuthType::Period => format!(
            "📅 使用说明：\n\
             • 在过期时间前可重复使用\n\
             • 请合理安排使用时间\n\
             • 过期后需重新申请授权"
        ),
        AuthType::Temp => format!(
            "⚡ 使用说明：\n\
             • 10分钟内可重复使用\n\
             • 请尽快完成相关操作\n\
             • 过期后可重新获取"
        ),
        AuthType::LongtimeTemp => format!(
            "🔄 使用说明：\n\
             • 每次密码有效期10分钟\n\
             • 间隔5分钟可重新获取\n\
             • 请在密码有效期内使用"
        ),
    };

    let message = format!(
        "🔑 访问密码生成成功！\n\n\
         密码：<code>{}</code>\n\
         类型：{}\n\
         过期时间：{}\n\n\
         {}\n\n\
         💡 安全提示：\n\
         • 请妥善保管密码，不要分享给他人\n\
         • 建议复制保存，避免重复获取\n\
         • 密码过期后请及时重新获取\n\
         • 如遇问题请联系管理员",
        result.password,
        type_description,
        result.expire_time,
        usage_tips
    );

    bot.send_message(chat_id, message)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    // 将密码添加到记录中
    let mut tx = state.database.begin_transaction().await?;
    RecordRepository::add_password(&mut tx, record.unique_id, &result.password).await?;
    tx.commit().await?;

    Ok(())
}

/// 生成并发送密码（用于批准后立即推送）
pub async fn generate_and_send_password(
    _bot: &Bot,
    _chat_id: ChatId,
    record: &Record,
    state: &BotState,
) -> Result<String> {
    // 首先检查授权是否已过期
    if !record.is_active() {
        let expire_info = if let Some(ended_time) = record.ended_time {
            let beijing_tz = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
            let ended_time_beijing = ended_time.with_timezone(&beijing_tz);
            let current_time_beijing = chrono::Utc::now().with_timezone(&beijing_tz);
            format!("授权已于 {} 过期（当前时间：{}）",
                   ended_time_beijing.format("%Y-%m-%d %H:%M:%S"),
                   current_time_beijing.format("%Y-%m-%d %H:%M:%S"))
        } else {
            "授权已过期".to_string()
        };
        return Err(crate::error::AppError::business(&expire_info));
    }

    let user_service = state.user_service.read().await;
    let admin = user_service.get_admin_info_by_unique_id(record.inviter).await?
        .ok_or_else(|| crate::error::AppError::business("管理员信息不存在"))?;

    let admin_password = admin.password
        .ok_or_else(|| crate::error::AppError::business("管理员未设置密码"))?;
    drop(user_service); // 释放用户服务锁

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
            // 从结束时间提取年月日时 - 需要转换为UTC+8时区
            if let Some(end) = record.ended_time {
                // 将数据库中的UTC时间转换为UTC+8时区，然后提取时间组件
                let beijing_tz = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
                let end_beijing = end.with_timezone(&beijing_tz);
                (None, None, Some(end_beijing.year() as u32), Some(end_beijing.month()), Some(end_beijing.day()), Some(end_beijing.hour()))
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
    drop(password_service); // 释放密码服务锁

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