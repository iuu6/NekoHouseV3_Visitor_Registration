//! 用户服务模块 - 处理用户身份验证和权限管理

use crate::config::AppConfig;
use crate::database::{AdminRepository, Database};
use crate::error::{AppError, Result};
use crate::types::{Admin, UserInfo, UserRole};
use teloxide::types::User;

/// 用户服务
pub struct UserService {
    database: Database,
    config: AppConfig,
}

impl UserService {
    /// 创建新的用户服务实例
    pub fn new(database: Database, config: AppConfig) -> Self {
        Self { database, config }
    }

    /// 获取用户信息和角色
    pub async fn get_user_info(&self, user: &User) -> Result<UserInfo> {
        let user_id = user.id.0 as i64;
        
        // 检查是否为超级管理员
        if self.config.is_super_admin(user_id) {
            return Ok(UserInfo {
                telegram_id: user_id,
                username: user.username.clone(),
                first_name: user.first_name.clone().into(),
                last_name: user.last_name.clone(),
                role: UserRole::SuperAdmin,
            });
        }

        // 检查是否为普通管理员
        if let Some(_admin) = AdminRepository::find_by_telegram_id(self.database.pool(), user_id).await? {
            return Ok(UserInfo {
                telegram_id: user_id,
                username: user.username.clone(),
                first_name: user.first_name.clone().into(),
                last_name: user.last_name.clone(),
                role: UserRole::Admin,
            });
        }

        // 普通访客
        Ok(UserInfo {
            telegram_id: user_id,
            username: user.username.clone(),
            first_name: user.first_name.clone().into(),
            last_name: user.last_name.clone(),
            role: UserRole::Visitor,
        })
    }

    /// 验证用户是否为超级管理员
    pub fn is_super_admin(&self, user_id: i64) -> bool {
        self.config.is_super_admin(user_id)
    }

    /// 验证用户是否为管理员（包括超级管理员）
    pub async fn is_admin(&self, user_id: i64) -> Result<bool> {
        if self.config.is_super_admin(user_id) {
            return Ok(true);
        }
        
        AdminRepository::exists_by_telegram_id(self.database.pool(), user_id).await
    }

    /// 获取管理员信息（通过Telegram ID）
    pub async fn get_admin_info(&self, user_id: i64) -> Result<Option<Admin>> {
        AdminRepository::find_by_telegram_id(self.database.pool(), user_id).await
    }

    /// 获取管理员信息（通过数据库unique_id）
    pub async fn get_admin_info_by_unique_id(&self, unique_id: i64) -> Result<Option<Admin>> {
        AdminRepository::find_by_unique_id(self.database.pool(), unique_id).await
    }

    /// 创建新管理员（只有超级管理员可以操作）
    pub async fn create_admin(&self, operator_id: i64, target_user_id: i64) -> Result<i64> {
        // 检查操作者权限
        if !self.config.is_super_admin(operator_id) {
            return Err(AppError::permission("只有超级管理员可以添加管理员"));
        }

        // 检查目标用户是否已经是管理员
        if AdminRepository::exists_by_telegram_id(self.database.pool(), target_user_id).await? {
            return Err(AppError::business("该用户已经是管理员"));
        }

        // 创建管理员
        let mut tx = self.database.begin_transaction().await?;
        let admin = Admin::new(target_user_id);
        let admin_id = AdminRepository::create(&mut tx, &admin).await?;
        tx.commit().await?;

        log::info!("超级管理员 {} 添加了新管理员 {}", operator_id, target_user_id);
        Ok(admin_id)
    }

    /// 删除管理员（只有超级管理员可以操作）
    pub async fn remove_admin(&self, operator_id: i64, target_user_id: i64) -> Result<bool> {
        // 检查操作者权限
        if !self.config.is_super_admin(operator_id) {
            return Err(AppError::permission("只有超级管理员可以删除管理员"));
        }

        // 不能删除超级管理员
        if self.config.is_super_admin(target_user_id) {
            return Err(AppError::permission("不能删除超级管理员"));
        }

        // 查找要删除的管理员
        let admin = AdminRepository::find_by_telegram_id(self.database.pool(), target_user_id).await?;
        
        if let Some(admin) = admin {
            let mut tx = self.database.begin_transaction().await?;
            let removed = AdminRepository::delete(&mut tx, admin.unique_id).await?;
            tx.commit().await?;

            if removed {
                log::info!("超级管理员 {} 删除了管理员 {}", operator_id, target_user_id);
            }
            
            Ok(removed)
        } else {
            Err(AppError::business("目标用户不是管理员"))
        }
    }

    /// 更新管理员密码
    pub async fn update_admin_password(&self, admin_id: i64, new_password: &str) -> Result<bool> {
        // 验证密码格式
        if !Admin::validate_password(new_password) {
            return Err(AppError::validation("密码必须是4-10位数字"));
        }

        let mut tx = self.database.begin_transaction().await?;
        let updated = AdminRepository::update_password(&mut tx, admin_id, new_password).await?;
        tx.commit().await?;

        if updated {
            log::info!("管理员 {} 更新了密码", admin_id);
        }

        Ok(updated)
    }

    /// 验证管理员密码
    pub async fn verify_admin_password(&self, admin_id: i64, password: &str) -> Result<bool> {
        AdminRepository::verify_password(self.database.pool(), admin_id, password).await
    }

    /// 检查管理员是否已设置密码
    pub async fn admin_has_password(&self, admin_id: i64) -> Result<bool> {
        AdminRepository::has_password(self.database.pool(), admin_id).await
    }

    /// 生成管理员邀请码
    pub async fn generate_admin_invite_code(&self, admin_id: i64) -> Result<String> {
        let mut tx = self.database.begin_transaction().await?;
        let invite_code = AdminRepository::generate_invite_code(&mut tx, admin_id).await?;
        tx.commit().await?;

        log::info!("管理员 {} 生成了新的邀请码", admin_id);
        Ok(invite_code)
    }

    /// 通过邀请码查找管理员
    pub async fn find_admin_by_invite_code(&self, invite_code: &str) -> Result<Option<Admin>> {
        AdminRepository::find_by_invite_code(self.database.pool(), invite_code).await
    }

    /// 获取所有管理员列表（只有超级管理员可以查看）
    pub async fn list_all_admins(&self, operator_id: i64) -> Result<Vec<Admin>> {
        if !self.config.is_super_admin(operator_id) {
            return Err(AppError::permission("只有超级管理员可以查看管理员列表"));
        }

        AdminRepository::list_all(self.database.pool()).await
    }

    /// 验证操作权限
    pub async fn check_permission(
        &self,
        user_id: i64,
        required_role: UserRole,
    ) -> Result<UserRole> {
        let actual_role = if self.config.is_super_admin(user_id) {
            UserRole::SuperAdmin
        } else if AdminRepository::exists_by_telegram_id(self.database.pool(), user_id).await? {
            UserRole::Admin
        } else {
            UserRole::Visitor
        };

        // 权限等级检查
        let has_permission = match required_role {
            UserRole::Visitor => true, // 所有用户都有访客权限
            UserRole::Admin => matches!(actual_role, UserRole::Admin | UserRole::SuperAdmin),
            UserRole::SuperAdmin => matches!(actual_role, UserRole::SuperAdmin),
        };

        if has_permission {
            Ok(actual_role)
        } else {
            Err(AppError::permission(format!(
                "需要 {:?} 权限，当前用户权限为 {:?}",
                required_role, actual_role
            )))
        }
    }

    /// 格式化用户显示名称
    pub fn format_user_display_name(&self, user: &User) -> String {
        if let Some(ref username) = user.username {
            format!("@{}", username)
        } else {
            let last_name = user.last_name.as_deref().unwrap_or("");
            format!("{} {}", user.first_name, last_name).trim().to_string()
        }
    }

    /// 获取用户角色描述
    pub fn get_role_description(&self, role: UserRole) -> &'static str {
        match role {
            UserRole::SuperAdmin => "超级管理员",
            UserRole::Admin => "管理员",
            UserRole::Visitor => "访客",
        }
    }

    /// 检查用户是否可以执行特定操作
    pub async fn can_perform_action(&self, user_id: i64, action: &str) -> Result<bool> {
        let role = if self.config.is_super_admin(user_id) {
            UserRole::SuperAdmin
        } else if AdminRepository::exists_by_telegram_id(self.database.pool(), user_id).await? {
            UserRole::Admin
        } else {
            UserRole::Visitor
        };

        let can_perform = match action {
            // 超级管理员专用操作
            "addadmin" | "removeadmin" | "listadmins" => {
                matches!(role, UserRole::SuperAdmin)
            }
            // 管理员操作
            "editpasswd" | "geninvite" | "revoke" | "approve" => {
                matches!(role, UserRole::Admin | UserRole::SuperAdmin)
            }
            // 访客操作
            "req" | "getpassword" => true,
            // 其他操作默认拒绝
            _ => false,
        };

        Ok(can_perform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::NamedTempFile;

    async fn setup_test_service() -> Result<UserService> {
        let temp_file = NamedTempFile::new()?;
        let db_url = format!("sqlite:{}", temp_file.path().to_str().unwrap());
        let database = Database::new(&db_url).await?;

        let mut config = AppConfig::default();
        config.telegram.bot_token = "test_token".to_string();
        config.super_admin_ids.push(123456789);

        Ok(UserService::new(database, config))
    }

    #[tokio::test]
    async fn test_user_role_detection() -> Result<()> {
        let service = setup_test_service().await?;

        // 测试超级管理员
        assert!(service.is_super_admin(123456789));
        assert!(!service.is_super_admin(987654321));

        // 测试管理员检查
        let is_admin = service.is_admin(123456789).await?;
        assert!(is_admin); // 超级管理员也算管理员

        let is_admin = service.is_admin(987654321).await?;
        assert!(!is_admin); // 普通用户不是管理员

        Ok(())
    }

    #[tokio::test]
    async fn test_admin_creation() -> Result<()> {
        let service = setup_test_service().await?;
        let super_admin_id = 123456789;
        let new_admin_id = 987654321;

        // 超级管理员创建新管理员
        let admin_unique_id = service.create_admin(super_admin_id, new_admin_id).await?;
        assert!(admin_unique_id > 0);

        // 验证新管理员已创建
        let is_admin = service.is_admin(new_admin_id).await?;
        assert!(is_admin);

        // 尝试重复创建应该失败
        let result = service.create_admin(super_admin_id, new_admin_id).await;
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_permission_check() -> Result<()> {
        let service = setup_test_service().await?;
        let super_admin_id = 123456789;
        let visitor_id = 555666777;

        // 超级管理员权限检查
        let role = service.check_permission(super_admin_id, UserRole::SuperAdmin).await?;
        assert_eq!(role, UserRole::SuperAdmin);

        // 访客尝试执行超级管理员操作应该失败
        let result = service.check_permission(visitor_id, UserRole::SuperAdmin).await;
        assert!(result.is_err());

        // 访客执行访客操作应该成功
        let role = service.check_permission(visitor_id, UserRole::Visitor).await?;
        assert_eq!(role, UserRole::Visitor);

        Ok(())
    }

    #[tokio::test]
    async fn test_action_permissions() -> Result<()> {
        let service = setup_test_service().await?;
        let super_admin_id = 123456789;
        let visitor_id = 555666777;

        // 超级管理员可以执行addadmin
        let can_add_admin = service.can_perform_action(super_admin_id, "addadmin").await?;
        assert!(can_add_admin);

        // 访客不能执行addadmin
        let can_add_admin = service.can_perform_action(visitor_id, "addadmin").await?;
        assert!(!can_add_admin);

        // 所有用户都可以执行req
        let can_req = service.can_perform_action(visitor_id, "req").await?;
        assert!(can_req);

        Ok(())
    }
}