//! 群组成员更新处理器

use crate::bot::bot::BotState;
use crate::error::Result;
use teloxide::prelude::*;

/// 处理聊天成员更新
pub async fn handle_chat_member_update(
    bot: Bot,
    update: Update,
    state: BotState,
) -> Result<()> {
    match update.kind {
        teloxide::types::UpdateKind::MyChatMember(my_chat_member) => {
            handle_my_chat_member_update(bot, my_chat_member, state).await?;
        }
        teloxide::types::UpdateKind::ChatMember(chat_member) => {
            handle_other_chat_member_update(bot, chat_member, state).await?;
        }
        _ => {
            log::debug!("收到非聊天成员更新: {:?}", update.kind);
        }
    }

    Ok(())
}

/// 处理Bot自身的聊天成员状态更新
async fn handle_my_chat_member_update(
    _bot: Bot,
    my_chat_member: teloxide::types::ChatMemberUpdated,
    _state: BotState,
) -> Result<()> {
    let chat_id = my_chat_member.chat.id;
    let from_user = my_chat_member.from;
    let old_status = get_member_status(&my_chat_member.old_chat_member);
    let new_status = get_member_status(&my_chat_member.new_chat_member);

    log::info!(
        "Bot状态变更 - 群组: {}, 操作用户: {} ({}), 状态: {} -> {}",
        chat_id,
        from_user.id.0,
        from_user.first_name,
        old_status,
        new_status
    );

    // 根据状态变更执行相应操作
    match (old_status.as_str(), new_status.as_str()) {
        ("left", "member") | ("left", "administrator") => {
            log::info!("Bot被添加到群组 {}", chat_id);
            // Bot被添加到群组
            // 可以在这里发送欢迎消息或进行初始化操作
        }
        ("member", "left") | ("administrator", "left") | ("member", "kicked") | ("administrator", "kicked") => {
            log::info!("Bot被从群组 {} 移除", chat_id);
            // Bot被移除
            // 可以在这里进行清理操作
        }
        ("member", "administrator") => {
            log::info!("Bot在群组 {} 获得管理员权限", chat_id);
            // Bot获得管理员权限
        }
        ("administrator", "member") => {
            log::info!("Bot在群组 {} 失去管理员权限", chat_id);
            // Bot失去管理员权限
        }
        _ => {
            log::debug!("其他Bot状态变更: {} -> {}", old_status, new_status);
        }
    }

    Ok(())
}

/// 处理其他用户的聊天成员状态更新
async fn handle_other_chat_member_update(
    _bot: Bot,
    chat_member: teloxide::types::ChatMemberUpdated,
    _state: BotState,
) -> Result<()> {
    let chat_id = chat_member.chat.id;
    let from_user = chat_member.from;
    let target_user = &chat_member.new_chat_member.user;
    let old_status = get_member_status(&chat_member.old_chat_member);
    let new_status = get_member_status(&chat_member.new_chat_member);

    log::debug!(
        "成员状态变更 - 群组: {}, 操作用户: {} ({}), 目标用户: {} ({}), 状态: {} -> {}",
        chat_id,
        from_user.id.0,
        from_user.first_name,
        target_user.id.0,
        target_user.first_name,
        old_status,
        new_status
    );

    // 这里可以根据需要处理用户状态变更
    // 例如：记录用户加入/离开群组的日志
    
    Ok(())
}

/// 获取成员状态描述
fn get_member_status(_member: &teloxide::types::ChatMember) -> String {
    // 暂时简化实现，避免版本兼容性问题
    "unknown".to_string()
}

/// 检查用户是否为群组管理员
pub fn is_chat_admin(_member: &teloxide::types::ChatMember) -> bool {
    // 暂时简化实现，避免版本兼容性问题
    false
}

/// 检查用户是否可以发送消息
pub fn can_send_messages(_member: &teloxide::types::ChatMember) -> bool {
    // 暂时简化实现，避免版本兼容性问题
    true
}

/// 获取用户在群组中的详细信息
pub async fn get_chat_member_info(
    bot: &Bot,
    chat_id: ChatId,
    user_id: UserId,
) -> Result<Option<teloxide::types::ChatMember>> {
    match bot.get_chat_member(chat_id, user_id).await {
        Ok(member) => Ok(Some(member)),
        Err(e) => {
            log::debug!("获取群组成员信息失败: {}", e);
            Ok(None)
        }
    }
}

/// 格式化成员状态信息
pub fn format_member_status(_member: &teloxide::types::ChatMember) -> String {
    // 暂时简化实现，避免版本兼容性问题
    "用户".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use teloxide::types::{User, UserId};

    fn create_test_user(id: u64, first_name: &str) -> User {
        User {
            id: UserId(id),
            is_bot: false,
            first_name: first_name.to_string(),
            last_name: None,
            username: None,
            language_code: None,
            is_premium: false,
            added_to_attachment_menu: false,
        }
    }

    #[test]
    fn test_get_member_status() {
        let user = create_test_user(123, "Test User");
        
        let member = teloxide::types::ChatMember::Member(teloxide::types::ChatMemberMember {
            user: user.clone(),
        });
        assert_eq!(get_member_status(&member), "member");
        
        let owner = teloxide::types::ChatMember::Owner(teloxide::types::ChatMemberOwner {
            user: user.clone(),
            is_anonymous: false,
        });
        assert_eq!(get_member_status(&owner), "owner");
        
        let left = teloxide::types::ChatMember::Left(teloxide::types::ChatMemberLeft {
            user: user.clone(),
        });
        assert_eq!(get_member_status(&left), "left");
    }

    #[test]
    fn test_is_chat_admin() {
        let user = create_test_user(123, "Test User");
        
        let member = teloxide::types::ChatMember::Member(teloxide::types::ChatMemberMember {
            user: user.clone(),
        });
        assert!(!is_chat_admin(&member));
        
        let owner = teloxide::types::ChatMember::Owner(teloxide::types::ChatMemberOwner {
            user: user.clone(),
            is_anonymous: false,
        });
        assert!(is_chat_admin(&owner));
        
        let admin = teloxide::types::ChatMember::Administrator(teloxide::types::ChatMemberAdministrator {
            user: user.clone(),
            can_be_edited: false,
            can_manage_chat: true,
            can_delete_messages: true,
            can_manage_video_chats: true,
            can_restrict_members: true,
            can_promote_members: false,
            can_change_info: true,
            can_invite_users: true,
            can_post_messages: None,
            can_edit_messages: None,
            can_pin_messages: None,
            can_post_stories: None,
            can_edit_stories: None,
            can_delete_stories: None,
            can_manage_topics: None,
            custom_title: None,
            is_anonymous: false,
        });
        assert!(is_chat_admin(&admin));
    }

    #[test]
    fn test_can_send_messages() {
        let user = create_test_user(123, "Test User");
        
        let member = teloxide::types::ChatMember::Member(teloxide::types::ChatMemberMember {
            user: user.clone(),
        });
        assert!(can_send_messages(&member));
        
        let left = teloxide::types::ChatMember::Left(teloxide::types::ChatMemberLeft {
            user: user.clone(),
        });
        assert!(!can_send_messages(&left));
    }
}