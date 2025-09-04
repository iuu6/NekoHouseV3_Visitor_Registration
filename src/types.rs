//! 系统类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 用户角色枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    /// 超级管理员（配置文件中定义）
    SuperAdmin,
    /// 管理员（数据库中的admin表）
    Admin,
    /// 访客
    Visitor,
}

/// 授权状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthStatus {
    /// 待审批
    Pending,
    /// 已授权
    Auth,
    /// 已撤销
    Revoked,
}

impl AuthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthStatus::Pending => "pending",
            AuthStatus::Auth => "auth",
            AuthStatus::Revoked => "revoked",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(AuthStatus::Pending),
            "auth" => Some(AuthStatus::Auth),
            "revoked" => Some(AuthStatus::Revoked),
            _ => None,
        }
    }
}

/// 授权类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    /// 时效密码（指定时长有效）
    Limited,
    /// 指定过期时间密码
    Period,
    /// 两小时内次数密码
    Times,
    /// 临时单次密码（10分钟有效）
    Temp,
    /// 长期单次密码
    LongtimeTemp,
}

impl AuthType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthType::Limited => "limited",
            AuthType::Period => "period",
            AuthType::Times => "times",
            AuthType::Temp => "temp",
            AuthType::LongtimeTemp => "longtime_temp",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "limited" => Some(AuthType::Limited),
            "period" => Some(AuthType::Period),
            "times" => Some(AuthType::Times),
            "temp" => Some(AuthType::Temp),
            "longtime_temp" => Some(AuthType::LongtimeTemp),
            _ => None,
        }
    }

    /// 获取授权类型的中文描述
    pub fn description(&self) -> &'static str {
        match self {
            AuthType::Limited => "时效密码",
            AuthType::Period => "指定过期时间密码",
            AuthType::Times => "两小时内次数密码",
            AuthType::Temp => "临时单次密码",
            AuthType::LongtimeTemp => "长期单次密码",
        }
    }
}

/// 管理员表实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Admin {
    /// 数据库唯一ID
    pub unique_id: i64,
    /// Telegram用户ID
    pub id: i64,
    /// 密码（明文，4-10位数字）
    pub password: Option<String>,
    /// 邀请码（UUID）
    pub invite_code: Option<String>,
}

impl Admin {
    pub fn new(telegram_id: i64) -> Self {
        Self {
            unique_id: 0, // 由数据库自动分配
            id: telegram_id,
            password: None,
            invite_code: None,
        }
    }

    /// 生成新的邀请码
    pub fn generate_invite_code(&mut self) {
        self.invite_code = Some(Uuid::new_v4().to_string());
    }

    /// 验证密码格式（4-10位数字）
    pub fn validate_password(password: &str) -> bool {
        password.len() >= 4 
            && password.len() <= 10 
            && password.chars().all(|c| c.is_ascii_digit())
    }
}

/// 访客记录表实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    /// 数据库唯一ID
    pub unique_id: i64,
    /// 状态
    pub status: AuthStatus,
    /// 访客Telegram ID
    pub vis_id: i64,
    /// 授权类型
    pub auth_type: AuthType,
    /// 使用次数（仅用于times类型）
    pub times: Option<i32>,
    /// 授权开始时间
    pub start_time: Option<DateTime<Utc>>,
    /// 授权结束时间
    pub ended_time: Option<DateTime<Utc>>,
    /// 密码列表（JSON存储）
    pub password: Option<String>, // 存储JSON格式的密码列表
    /// 邀请者（admin表中的unique_id）
    pub inviter: i64,
    /// 最后更新时间
    pub update_at: DateTime<Utc>,
}

impl Record {
    pub fn new(vis_id: i64, inviter: i64) -> Self {
        Self {
            unique_id: 0, // 由数据库自动分配
            status: AuthStatus::Pending,
            vis_id,
            auth_type: AuthType::Temp, // 默认值，后续会根据管理员选择更新
            times: None,
            start_time: None,
            ended_time: None,
            password: None,
            inviter,
            update_at: Utc::now(),
        }
    }

    /// 添加密码到密码列表
    pub fn add_password(&mut self, new_password: &str) -> Result<(), serde_json::Error> {
        let mut passwords: Vec<String> = if let Some(ref password_json) = self.password {
            serde_json::from_str(password_json)?
        } else {
            Vec::new()
        };
        
        passwords.push(new_password.to_string());
        self.password = Some(serde_json::to_string(&passwords)?);
        self.update_at = Utc::now();
        
        Ok(())
    }

    /// 获取密码列表
    pub fn get_passwords(&self) -> Result<Vec<String>, serde_json::Error> {
        if let Some(ref password_json) = self.password {
            serde_json::from_str(password_json)
        } else {
            Ok(Vec::new())
        }
    }

    /// 检查授权是否仍然有效
    pub fn is_active(&self) -> bool {
        match self.status {
            AuthStatus::Auth => {
                // 检查是否过期
                if let Some(ended_time) = self.ended_time {
                    Utc::now() <= ended_time
                } else {
                    true // 没有结束时间限制
                }
            }
            _ => false,
        }
    }

    /// 标记为已撤销
    pub fn revoke(&mut self) {
        self.status = AuthStatus::Revoked;
        self.update_at = Utc::now();
    }

    /// 批准授权
    pub fn approve(&mut self, auth_type: AuthType, start_time: Option<DateTime<Utc>>, ended_time: Option<DateTime<Utc>>, times: Option<i32>) {
        self.status = AuthStatus::Auth;
        self.auth_type = auth_type;
        self.start_time = start_time;
        self.ended_time = ended_time;
        self.times = times;
        self.update_at = Utc::now();
    }
}

/// 密码生成请求
#[derive(Debug, Clone)]
pub struct PasswordRequest {
    pub admin_password: String,
    pub auth_type: AuthType,
    pub times: Option<u32>,
    pub hours: Option<u32>,
    pub minutes: Option<u32>,
    pub end_year: Option<u32>,
    pub end_month: Option<u32>,
    pub end_day: Option<u32>,
    pub end_hour: Option<u32>,
    pub start_time: Option<DateTime<Utc>>,
}

/// 用户信息
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub telegram_id: i64,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: UserRole,
}

impl UserInfo {
    pub fn display_name(&self) -> String {
        if let Some(ref username) = self.username {
            format!("@{}", username)
        } else if let Some(ref first_name) = self.first_name {
            let last_name = self.last_name.as_deref().unwrap_or("");
            format!("{} {}", first_name, last_name).trim().to_string()
        } else {
            format!("用户{}", self.telegram_id)
        }
    }
}

/// Telegram回调数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackData {
    pub action: String,
    pub data: Option<String>,
}

impl CallbackData {
    pub fn new<S: Into<String>>(action: S) -> Self {
        Self {
            action: action.into(),
            data: None,
        }
    }

    pub fn with_data<S: Into<String>>(action: S, data: S) -> Self {
        Self {
            action: action.into(),
            data: Some(data.into()),
        }
    }

    pub fn to_callback_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}