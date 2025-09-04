//! 配置管理模块

use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 应用程序配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 数据库配置
    pub database: DatabaseConfig,
    /// Telegram Bot配置
    pub telegram: TelegramConfig,
    /// 超级管理员ID列表
    pub super_admin_ids: Vec<i64>,
    /// 时间偏移（用于密码生成加密）
    pub time_offset: i64,
}

/// 数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// 数据库文件路径
    pub path: String,
}

/// Telegram配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Bot Token
    pub bot_token: String,
}

impl AppConfig {
    /// 从文件加载配置
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = serde_json::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// 保存配置到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 验证配置有效性
    pub fn validate(&self) -> Result<()> {
        if self.telegram.bot_token.is_empty() {
            return Err(AppError::validation("Telegram bot token不能为空"));
        }

        if self.database.path.is_empty() {
            return Err(AppError::validation("数据库路径不能为空"));
        }

        if self.super_admin_ids.is_empty() {
            return Err(AppError::validation("至少需要配置一个超级管理员"));
        }

        Ok(())
    }

    /// 检查用户是否为超级管理员
    pub fn is_super_admin(&self, user_id: i64) -> bool {
        self.super_admin_ids.contains(&user_id)
    }

    /// 添加超级管理员
    pub fn add_super_admin(&mut self, user_id: i64) {
        if !self.super_admin_ids.contains(&user_id) {
            self.super_admin_ids.push(user_id);
        }
    }

    /// 移除超级管理员
    pub fn remove_super_admin(&mut self, user_id: i64) {
        self.super_admin_ids.retain(|&id| id != user_id);
    }

    /// 获取调整后的时间戳（用于密码生成）
    pub fn get_adjusted_timestamp(&self) -> i64 {
        chrono::Utc::now().timestamp() + self.time_offset
    }

    /// 获取数据库URL
    pub fn get_database_url(&self) -> String {
        format!("sqlite:{}", self.database.path)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                path: "./data/nekohouse.db".to_string(),
            },
            telegram: TelegramConfig {
                bot_token: String::new(),
            },
            super_admin_ids: Vec::new(),
            time_offset: 0,
        }
    }
}

/// 配置管理器
pub struct ConfigManager {
    config: AppConfig,
    config_path: String,
}

impl ConfigManager {
    /// 创建配置管理器
    pub fn new(config_path: &str) -> Result<Self> {
        let config = if Path::new(config_path).exists() {
            AppConfig::from_file(config_path)?
        } else {
            log::warn!("配置文件 {} 不存在，创建默认配置", config_path);
            let default_config = AppConfig::default();
            
            // 创建配置文件目录
            if let Some(parent) = Path::new(config_path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            
            default_config.save_to_file(config_path)?;
            default_config
        };

        Ok(Self {
            config,
            config_path: config_path.to_string(),
        })
    }

    /// 获取配置
    pub fn get_config(&self) -> &AppConfig {
        &self.config
    }

    /// 更新配置
    pub fn update_config<F>(&mut self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut AppConfig),
    {
        updater(&mut self.config);
        self.config.validate()?;
        self.config.save_to_file(&self.config_path)?;
        Ok(())
    }

    /// 重新加载配置
    pub fn reload(&mut self) -> Result<()> {
        self.config = AppConfig::from_file(&self.config_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_validation() {
        let mut config = AppConfig::default();
        
        // 应该验证失败，因为bot_token为空
        assert!(config.validate().is_err());
        
        config.telegram.bot_token = "test_token".to_string();
        config.super_admin_ids.push(123456789);
        
        // 现在应该验证成功
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_super_admin_management() {
        let mut config = AppConfig::default();
        
        assert!(!config.is_super_admin(123));
        
        config.add_super_admin(123);
        assert!(config.is_super_admin(123));
        
        config.remove_super_admin(123);
        assert!(!config.is_super_admin(123));
    }

    #[test]
    fn test_config_file_operations() -> Result<()> {
        let mut config = AppConfig::default();
        config.telegram.bot_token = "test_token".to_string();
        config.super_admin_ids.push(123456789);
        
        let temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path();
        
        // 保存配置
        config.save_to_file(temp_path)?;
        
        // 加载配置
        let loaded_config = AppConfig::from_file(temp_path)?;
        
        assert_eq!(config.telegram.bot_token, loaded_config.telegram.bot_token);
        assert_eq!(config.super_admin_ids, loaded_config.super_admin_ids);
        
        Ok(())
    }

    #[test]
    fn test_config_manager() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path().to_str().unwrap();
        
        // 删除临时文件，让ConfigManager创建新的
        std::fs::remove_file(temp_path).ok();
        
        let mut manager = ConfigManager::new(temp_path)?;
        
        // 更新配置
        manager.update_config(|config| {
            config.telegram.bot_token = "updated_token".to_string();
            config.super_admin_ids.push(987654321);
        })?;
        
        assert_eq!(manager.get_config().telegram.bot_token, "updated_token");
        assert!(manager.get_config().is_super_admin(987654321));
        
        Ok(())
    }
}