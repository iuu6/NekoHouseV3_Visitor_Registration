//! 文本消息处理器

use crate::bot::bot::BotState;
use crate::database::RecordRepository;
use crate::error::Result;
use crate::types::{AuthStatus, AuthType};
use chrono::{DateTime, NaiveDateTime, Utc};
use teloxide::prelude::*;

/// 处理文本消息
pub async fn handle_text(bot: Bot, msg: Message, state: BotState) -> Result<()> {
    let user = match msg.from() {
        Some(user) => user,
        None => return Ok(()),
    };

    let text = match msg.text() {
        Some(text) => text.trim(),
        None => return Ok(()),
    };

    let user_id = user.id.0 as i64;
    log::debug!("收到用户 {} 的文本消息: {}", user_id, text);

    // 检查是否为特殊格式的管理员消息
    if let Some(caps) = parse_period_auth_message(text) {
        handle_period_authorization(&bot, &msg, caps, &state).await?;
        return Ok(());
    }

    if let Some(caps) = parse_longtime_temp_auth_message(text) {
        handle_longtime_temp_authorization(&bot, &msg, caps, &state).await?;
        return Ok(());
    }

    // 其他文本消息暂不处理，可以在这里添加更多处理逻辑
    log::debug!("未处理的文本消息: {}", text);
    Ok(())
}

/// 解析期间授权消息格式：期间 <record_id> YYYY-MM-DD HH
fn parse_period_auth_message(text: &str) -> Option<(i64, String)> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    
    if parts.len() == 4 && parts[0] == "期间" {
        if let Ok(record_id) = parts[1].parse::<i64>() {
            // 组合日期和时间 "YYYY-MM-DD HH"
            let datetime_str = format!("{} {}", parts[2], parts[3]);
            return Some((record_id, datetime_str));
        }
    }
    
    None
}

/// 解析长期临时授权消息格式：长期 <record_id> YYYY-MM-DD HH:MM
fn parse_longtime_temp_auth_message(text: &str) -> Option<(i64, String)> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    
    if parts.len() == 4 && parts[0] == "长期" {
        if let Ok(record_id) = parts[1].parse::<i64>() {
            // 组合日期和时间 "YYYY-MM-DD HH:MM"
            let datetime_str = format!("{} {}", parts[2], parts[3]);
            return Some((record_id, datetime_str));
        }
    }
    
    None
}

/// 处理期间授权
async fn handle_period_authorization(
    bot: &Bot,
    msg: &Message,
    (record_id, datetime_str): (i64, String),
    state: &BotState,
) -> Result<()> {
    let user_id = msg.from().unwrap().id.0 as i64;
    log::info!("管理员 {} 设置期间授权，记录ID: {}, 时间: {}", user_id, record_id, datetime_str);

    // 检查管理员权限
    let user_service = state.user_service.read().await;
    if !user_service.is_admin(user_id).await? {
        bot.send_message(msg.chat.id, "❌ 只有管理员可以批准授权").await?;
        return Ok(());
    }

    // 解析时间格式 YYYY-MM-DD HH
    let end_time = match parse_datetime(&datetime_str) {
        Ok(dt) => dt,
        Err(e) => {
            bot.send_message(
                msg.chat.id,
                format!(
                    "❌ 时间格式错误: {}\n\n\
                     正确格式：YYYY-MM-DD HH\n\
                     例如：2024-12-25 18",
                    e
                )
            ).await?;
            return Ok(());
        }
    };

    // 检查时间是否为未来时间
    if end_time <= Utc::now() {
        bot.send_message(msg.chat.id, "❌ 结束时间必须是未来时间").await?;
        return Ok(());
    }

    // 批准授权
    let mut tx = state.database.begin_transaction().await?;
    let success = RecordRepository::approve_authorization(
        &mut tx,
        record_id,
        AuthType::Period,
        Some(Utc::now()),
        Some(end_time),
        None,
    ).await?;
    tx.commit().await?;

    if success {
        // 立即生成并推送密码给访客
        if let Some(record) = RecordRepository::find_by_id(state.database.pool(), record_id).await? {
            let visitor_chat_id = ChatId(record.vis_id);
            
            // 生成并推送密码
            match crate::handlers::visitor::generate_and_send_password(bot, visitor_chat_id, &record, state).await {
                Ok(password) => {
                    bot.send_message(
                        visitor_chat_id,
                        format!(
                            "✅ 您的授权已被批准！\n\n\
                             📋 授权类型：指定过期时间密码\n\
                             📅 过期时间：{}\n\
                             🆔 批准ID：{}\n\
                             🔑 密码：<code>{}</code>\n\n\
                             💡 密码已自动生成，请妥善保管\n\
                             ⚠️ 密码在过期时间前可重复使用",
                            end_time.format("%Y-%m-%d %H:%M:%S"),
                            record_id,
                            password
                        )
                    ).parse_mode(teloxide::types::ParseMode::Html).await.ok();
                }
                Err(e) => {
                    log::error!("为访客 {} 生成指定过期时间密码失败: {}", record.vis_id, e);
                    bot.send_message(
                        visitor_chat_id,
                        format!(
                            "✅ 您的授权已被批准！\n\n\
                             📋 授权类型：指定过期时间密码\n\
                             📅 过期时间：{}\n\
                             🆔 批准ID：{}\n\n\
                             ❗ 密码生成遇到问题，请使用 /getpassword 获取密码\n\
                             💡 如多次获取失败，请联系管理员",
                            end_time.format("%Y-%m-%d %H:%M:%S"),
                            record_id
                        )
                    ).await.ok();
                }
            }
        }

        // 确认消息给管理员
        let message = format!(
            "✅ 期间授权已批准\n\n\
             📝 记录ID：{}\n\
             📅 过期时间：{}\n\
             🕐 处理时间：{}",
            record_id,
            end_time.format("%Y-%m-%d %H:%M:%S"),
            Utc::now().format("%Y-%m-%d %H:%M:%S")
        );

        bot.send_message(msg.chat.id, message).await?;
        log::info!("管理员 {} 成功批准期间授权 {}", user_id, record_id);
    } else {
        bot.send_message(msg.chat.id, "❌ 授权失败，请检查记录是否存在").await?;
    }

    Ok(())
}

/// 处理长期临时授权
async fn handle_longtime_temp_authorization(
    bot: &Bot,
    msg: &Message,
    (record_id, datetime_str): (i64, String),
    state: &BotState,
) -> Result<()> {
    let user_id = msg.from().unwrap().id.0 as i64;
    log::info!("管理员 {} 设置长期临时授权，记录ID: {}, 时间: {}", user_id, record_id, datetime_str);

    // 检查管理员权限
    let user_service = state.user_service.read().await;
    if !user_service.is_admin(user_id).await? {
        bot.send_message(msg.chat.id, "❌ 只有管理员可以批准授权").await?;
        return Ok(());
    }

    // 解析时间格式 YYYY-MM-DD HH:MM
    let end_time = match parse_datetime_with_minutes(&datetime_str) {
        Ok(dt) => dt,
        Err(e) => {
            bot.send_message(
                msg.chat.id,
                format!(
                    "❌ 时间格式错误: {}\n\n\
                     正确格式：YYYY-MM-DD HH:MM\n\
                     例如：2024-12-31 23:59",
                    e
                )
            ).await?;
            return Ok(());
        }
    };

    // 检查时间是否为未来时间
    if end_time <= Utc::now() {
        bot.send_message(msg.chat.id, "❌ 结束时间必须是未来时间").await?;
        return Ok(());
    }

    // 批准授权
    let mut tx = state.database.begin_transaction().await?;
    let success = RecordRepository::approve_authorization(
        &mut tx,
        record_id,
        AuthType::LongtimeTemp,
        Some(Utc::now()),
        Some(end_time),
        None,
    ).await?;
    tx.commit().await?;

    if success {
        // 通知访客（长期临时密码不自动推送，需要用户主动获取）
        if let Some(record) = RecordRepository::find_by_id(state.database.pool(), record_id).await? {
            let visitor_chat_id = ChatId(record.vis_id);
            bot.send_message(
                visitor_chat_id,
                format!(
                    "✅ 您的授权已被批准！\n\n\
                     📋 授权类型：长期临时密码\n\
                     📅 有效期至：{}\n\
                     🆔 批准ID：{}\n\
                     ⏰ 可在5分钟间隔内重复获取密码\n\n\
                     💡 使用 /getpassword 获取密码\n\
                     ⚠️ 每次获取的密码有效期为10分钟\n\
                     🔄 如需重新获取，请等待5分钟间隔",
                    end_time.format("%Y-%m-%d %H:%M:%S"),
                    record_id
                )
            ).await.ok();
        }

        // 确认消息给管理员
        let message = format!(
            "✅ 长期临时授权已批准\n\n\
             📝 记录ID：{}\n\
             📅 有效期至：{}\n\
             🕐 处理时间：{}",
            record_id,
            end_time.format("%Y-%m-%d %H:%M:%S"),
            Utc::now().format("%Y-%m-%d %H:%M:%S")
        );

        bot.send_message(msg.chat.id, message).await?;
        log::info!("管理员 {} 成功批准长期临时授权 {}", user_id, record_id);
    } else {
        bot.send_message(msg.chat.id, "❌ 授权失败，请检查记录是否存在").await?;
    }

    Ok(())
}

/// 解析日期时间字符串 YYYY-MM-DD HH
fn parse_datetime(datetime_str: &str) -> Result<DateTime<Utc>> {
    // 添加默认的分钟和秒
    let full_datetime_str = format!("{}:00:00", datetime_str);
    
    let naive_dt = NaiveDateTime::parse_from_str(&full_datetime_str, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| crate::error::AppError::validation(format!("日期时间解析错误: {}", e)))?;
    
    // 转换为 UTC 时间（假设输入是北京时间 UTC+8）
    let utc_dt = naive_dt - chrono::Duration::hours(8);
    Ok(DateTime::from_naive_utc_and_offset(utc_dt, Utc))
}

/// 解析日期时间字符串 YYYY-MM-DD HH:MM
fn parse_datetime_with_minutes(datetime_str: &str) -> Result<DateTime<Utc>> {
    // 添加默认的秒
    let full_datetime_str = format!("{}:00", datetime_str);
    
    let naive_dt = NaiveDateTime::parse_from_str(&full_datetime_str, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| crate::error::AppError::validation(format!("日期时间解析错误: {}", e)))?;
    
    // 转换为 UTC 时间（假设输入是北京时间 UTC+8）
    let utc_dt = naive_dt - chrono::Duration::hours(8);
    Ok(DateTime::from_naive_utc_and_offset(utc_dt, Utc))
}

/// 检查记录状态
pub async fn check_record_status(
    record_id: i64,
    state: &BotState,
) -> Result<Option<crate::types::Record>> {
    let record = RecordRepository::find_by_id(state.database.pool(), record_id).await?;
    
    if let Some(ref record) = record {
        if record.status != AuthStatus::Pending {
            return Err(crate::error::AppError::business("该请求已被处理"));
        }
    }
    
    Ok(record)
}

/// 格式化时间显示
pub fn format_duration(start: DateTime<Utc>, end: DateTime<Utc>) -> String {
    let duration = end.signed_duration_since(start);
    let days = duration.num_days();
    let hours = duration.num_hours() % 24;
    let minutes = duration.num_minutes() % 60;

    if days > 0 {
        format!("{}天{}小时{}分钟", days, hours, minutes)
    } else if hours > 0 {
        format!("{}小时{}分钟", hours, minutes)
    } else {
        format!("{}分钟", minutes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_period_auth_message() {
        assert_eq!(
            parse_period_auth_message("期间 123 2024-12-25 18"),
            Some((123, "2024-12-25 18".to_string()))
        );
        
        assert_eq!(parse_period_auth_message("期间 invalid 2024-12-25 18"), None);
        assert_eq!(parse_period_auth_message("其他 123 2024-12-25 18"), None);
        assert_eq!(parse_period_auth_message("期间 123"), None);
        assert_eq!(parse_period_auth_message("期间 123 2024-12-25"), None); // Missing hour
    }

    #[test]
    fn test_parse_longtime_temp_auth_message() {
        assert_eq!(
            parse_longtime_temp_auth_message("长期 456 2024-12-31 23:59"),
            Some((456, "2024-12-31 23:59".to_string()))
        );
        
        assert_eq!(parse_longtime_temp_auth_message("长期 invalid 2024-12-31 23:59"), None);
        assert_eq!(parse_longtime_temp_auth_message("期间 456 2024-12-31 23:59"), None);
        assert_eq!(parse_longtime_temp_auth_message("长期 456 2024-12-31"), None); // Missing time
    }

    #[test]
    fn test_parse_datetime() {
        let result = parse_datetime("2024-12-25 18");
        assert!(result.is_ok());
        
        let dt = result.unwrap();
        // 由于转换了时区，应该是 UTC 时间
        assert_eq!(dt.format("%Y-%m-%d %H:%M:%S").to_string(), "2024-12-25 10:00:00");
    }

    #[test]
    fn test_parse_datetime_with_minutes() {
        let result = parse_datetime_with_minutes("2024-12-31 23:59");
        assert!(result.is_ok());
        
        let dt = result.unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M:%S").to_string(), "2024-12-31 15:59:00");
    }

    #[test]
    fn test_invalid_datetime_formats() {
        assert!(parse_datetime("invalid-date").is_err());
        assert!(parse_datetime("2024-13-25 18").is_err()); // 无效月份
        assert!(parse_datetime("2024-12-32 18").is_err()); // 无效日期
        assert!(parse_datetime("2024-12-25 25").is_err()); // 无效小时
    }

    #[test]
    fn test_format_duration() {
        let start = Utc::now();
        let end1 = start + chrono::Duration::hours(2) + chrono::Duration::minutes(30);
        assert_eq!(format_duration(start, end1), "2小时30分钟");

        let end2 = start + chrono::Duration::days(1) + chrono::Duration::hours(3);
        assert_eq!(format_duration(start, end2), "1天3小时0分钟");

        let end3 = start + chrono::Duration::minutes(45);
        assert_eq!(format_duration(start, end3), "45分钟");
    }
}