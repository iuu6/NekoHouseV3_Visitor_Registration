//! NekoHouse Bot主体结构

use crate::auth::{PasswordService, UserService};
use crate::config::AppConfig;
use crate::database::Database;
use crate::error::{AppError, Result};
use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::Me,
    utils::command::BotCommands,
    dispatching::{UpdateHandler, HandlerExt},
    error_handlers::LoggingErrorHandler
};
use tokio::sync::RwLock;

/// NekoHouse Bot状态
#[derive(Clone)]
pub struct BotState {
    pub database: Database,
    pub config: AppConfig,
    pub user_service: Arc<RwLock<UserService>>,
    pub password_service: Arc<RwLock<PasswordService>>,
    pub bot_info: Arc<RwLock<Option<Me>>>,
}

impl BotState {
    pub fn new(database: Database, config: AppConfig) -> Self {
        let user_service = Arc::new(RwLock::new(UserService::new(database.clone(), config.clone())));
        let password_service = Arc::new(RwLock::new(PasswordService::new()));
        
        Self {
            database,
            config,
            user_service,
            password_service,
            bot_info: Arc::new(RwLock::new(None)),
        }
    }

    /// 设置Bot信息
    pub async fn set_bot_info(&self, me: Me) {
        let mut bot_info = self.bot_info.write().await;
        *bot_info = Some(me);
    }

    /// 获取Bot信息
    pub async fn get_bot_info(&self) -> Option<Me> {
        let bot_info = self.bot_info.read().await;
        bot_info.clone()
    }

    /// 获取Bot用户名
    pub async fn get_bot_username(&self) -> Option<String> {
        let bot_info = self.bot_info.read().await;
        bot_info.as_ref().and_then(|me| me.username.clone())
    }
}

/// NekoHouse Telegram Bot
pub struct NekoHouseBot {
    bot: Bot,
    state: BotState,
}

impl NekoHouseBot {
    /// 创建新的Bot实例
    pub async fn new(config: AppConfig) -> Result<Self> {
        // 创建数据库连接
        let database = Database::new(&config.get_database_url()).await?;

        // 数据库已经初始化，不需要额外的ping操作

        log::info!("数据库连接成功");

        // 创建Bot实例
        let bot = Bot::new(&config.telegram.bot_token);

        // 创建状态
        let state = BotState::new(database, config);

        // 获取Bot信息
        let me = bot.get_me().await?;
        state.set_bot_info(me.clone()).await;

        log::info!("Bot初始化成功: @{}", me.username.as_deref().unwrap_or("unknown"));

        Ok(Self { bot, state })
    }

    /// 运行Bot
    pub async fn run(self) -> Result<()> {
        log::info!("NekoHouse Bot 正在启动...");

        let handler = self.create_handler();

        Dispatcher::builder(self.bot, handler)
            .dependencies(dptree::deps![self.state])
            .default_handler(|upd| async move {
                log::warn!("Unhandled update: {:?}", upd);
            })
            .error_handler(LoggingErrorHandler::with_custom_text(
                "An error has occurred in the dispatcher",
            ))
            .build()
            .dispatch()
            .await;

        log::info!("Bot已停止运行");
        Ok(())
    }

    /// 创建消息处理器
    fn create_handler(&self) -> UpdateHandler<crate::error::AppError> {
        use dptree::case;
        use teloxide::utils::command::BotCommands;

        dptree::entry()
            .branch(
                Update::filter_message()
                    .filter_command::<Command>()
                    .endpoint(handle_command)
            )
            .branch(
                Update::filter_message()
                    .filter(|msg: Message| msg.text().is_some())
                    .endpoint(crate::handlers::handle_text)
            )
            .branch(
                Update::filter_callback_query()
                    .endpoint(crate::handlers::handle_callback_query)
            )
            .branch(
                Update::filter_my_chat_member()
                    .endpoint(crate::handlers::handle_chat_member_update)
            )
            .branch(
                Update::filter_chat_member()
                    .endpoint(crate::handlers::handle_chat_member_update)
            )
    }

    /// 获取Bot状态
    pub fn state(&self) -> &BotState {
        &self.state
    }

    /// 获取Bot实例
    pub fn bot(&self) -> &Bot {
        &self.bot
    }
}

/// 统一命令处理分发器
async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: BotState,
) -> Result<()> {
    match cmd {
        Command::Start => crate::handlers::start_command(bot, msg, cmd, state).await,
        Command::AddAdmin(_) => crate::handlers::add_admin_command(bot, msg, cmd, state).await,
        Command::EditPassword(_) => crate::handlers::edit_password_command(bot, msg, cmd, state).await,
        Command::GenInvite => crate::handlers::gen_invite_command(bot, msg, state).await,
        Command::Revoke(_) => crate::handlers::revoke_command(bot, msg, cmd, state).await,
        Command::Req(_) => crate::handlers::req_command(bot, msg, cmd, state).await,
        Command::GetPassword => crate::handlers::get_password_command(bot, msg, state).await,
    }
}

/// Bot命令枚举
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    /// 开始使用Bot
    #[command(description = "开始使用")]
    Start,
    
    /// 添加管理员（超级管理员专用）
    #[command(description = "添加管理员 (超级管理员专用)")]
    AddAdmin(String),
    
    /// 修改管理员密码
    #[command(description = "修改密码")]
    #[command(rename = "editpasswd")]
    EditPassword(String),
    
    /// 生成邀请码
    #[command(description = "生成邀请码")]
    GenInvite,
    
    /// 撤销授权
    #[command(description = "撤销授权")]
    Revoke(String),
    
    /// 申请访客授权
    #[command(description = "申请访客授权")]
    Req(String),
    
    /// 获取密码
    #[command(description = "获取密码")]
    GetPassword,
}

impl Command {
    /// 检查命令是否需要特定权限
    pub fn required_role(&self) -> crate::types::UserRole {
        use crate::types::UserRole;
        
        match self {
            Command::AddAdmin(_) => UserRole::SuperAdmin,
            Command::EditPassword(_) | Command::GenInvite | Command::Revoke(_) => UserRole::Admin,
            Command::Start | Command::Req(_) | Command::GetPassword => UserRole::Visitor,
        }
    }
    
    /// 获取命令描述
    pub fn description(&self) -> &'static str {
        match self {
            Command::Start => "开始使用Bot",
            Command::AddAdmin(_) => "添加管理员",
            Command::EditPassword(_) => "修改密码", 
            Command::GenInvite => "生成邀请码",
            Command::Revoke(_) => "撤销授权",
            Command::Req(_) => "申请访客授权",
            Command::GetPassword => "获取密码",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    async fn create_test_config() -> Result<AppConfig> {
        let temp_file = NamedTempFile::new()?;
        let mut config = AppConfig::default();
        config.database.path = temp_file.path().to_str().unwrap().to_string();
        config.telegram.bot_token = "123456789:ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_string();
        config.super_admin_ids.push(123456789);
        Ok(config)
    }

    #[tokio::test]
    async fn test_bot_state_creation() -> Result<()> {
        let config = create_test_config().await?;
        let database = Database::new(&config.get_database_url()).await?;
        let state = BotState::new(database, config);

        // 测试初始状态
        let bot_info = state.get_bot_info().await;
        assert!(bot_info.is_none());

        let username = state.get_bot_username().await;
        assert!(username.is_none());

        Ok(())
    }

    #[test]
    fn test_command_permissions() {
        assert_eq!(Command::Start.required_role(), crate::types::UserRole::Visitor);
        assert_eq!(Command::AddAdmin(123).required_role(), crate::types::UserRole::SuperAdmin);
        assert_eq!(Command::EditPassword("1234".to_string()).required_role(), crate::types::UserRole::Admin);
        assert_eq!(Command::GenInvite.required_role(), crate::types::UserRole::Admin);
    }
}