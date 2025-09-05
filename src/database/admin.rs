//! 管理员表操作模块

use crate::error::{AppError, Result};
use crate::types::Admin;

use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;

/// 管理员数据库操作
pub struct AdminRepository;

impl AdminRepository {
    /// 创建新管理员
    pub async fn create(tx: &mut Transaction<'_, Sqlite>, admin: &Admin) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO admin (id, password, invite_code, updated_at)
            VALUES (?, ?, ?, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(admin.id)
        .bind(&admin.password)
        .bind(&admin.invite_code)
        .execute(&mut **tx)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// 通过Telegram ID查找管理员
    pub async fn find_by_telegram_id(
        pool: &sqlx::Pool<Sqlite>,
        telegram_id: i64,
    ) -> Result<Option<Admin>> {
        let row = sqlx::query(
            r#"
            SELECT unique_id, id, password, invite_code
            FROM admin
            WHERE id = ?
            "#,
        )
        .bind(telegram_id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Admin {
                unique_id: row.get("unique_id"),
                id: row.get("id"),
                password: row.get("password"),
                invite_code: row.get("invite_code"),
            }))
        } else {
            Ok(None)
        }
    }

    /// 通过unique_id查找管理员
    pub async fn find_by_unique_id(
        pool: &sqlx::Pool<Sqlite>,
        unique_id: i64,
    ) -> Result<Option<Admin>> {
        let row = sqlx::query(
            r#"
            SELECT unique_id, id, password, invite_code
            FROM admin
            WHERE unique_id = ?
            "#,
        )
        .bind(unique_id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Admin {
                unique_id: row.get("unique_id"),
                id: row.get("id"),
                password: row.get("password"),
                invite_code: row.get("invite_code"),
            }))
        } else {
            Ok(None)
        }
    }

    /// 通过邀请码查找管理员
    pub async fn find_by_invite_code(
        pool: &sqlx::Pool<Sqlite>,
        invite_code: &str,
    ) -> Result<Option<Admin>> {
        let row = sqlx::query(
            r#"
            SELECT unique_id, id, password, invite_code
            FROM admin
            WHERE invite_code = ?
            "#,
        )
        .bind(invite_code)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Admin {
                unique_id: row.get("unique_id"),
                id: row.get("id"),
                password: row.get("password"),
                invite_code: row.get("invite_code"),
            }))
        } else {
            Ok(None)
        }
    }

    /// 更新管理员密码
    pub async fn update_password(
        tx: &mut Transaction<'_, Sqlite>,
        unique_id: i64,
        password: &str,
    ) -> Result<bool> {
        // 验证密码格式
        if !Admin::validate_password(password) {
            return Err(AppError::validation("密码必须是4-10位数字"));
        }

        let result = sqlx::query(
            r#"
            UPDATE admin
            SET password = ?, updated_at = CURRENT_TIMESTAMP
            WHERE unique_id = ?
            "#,
        )
        .bind(password)
        .bind(unique_id)
        .execute(&mut **tx)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 生成新的邀请码
    pub async fn generate_invite_code(
        tx: &mut Transaction<'_, Sqlite>,
        unique_id: i64,
    ) -> Result<String> {
        let new_invite_code = Uuid::new_v4().to_string();

        let result = sqlx::query(
            r#"
            UPDATE admin
            SET invite_code = ?, updated_at = CURRENT_TIMESTAMP
            WHERE unique_id = ?
            "#,
        )
        .bind(&new_invite_code)
        .bind(unique_id)
        .execute(&mut **tx)
        .await?;

        if result.rows_affected() > 0 {
            Ok(new_invite_code)
        } else {
            Err(AppError::business("管理员不存在"))
        }
    }

    /// 删除管理员
    pub async fn delete(tx: &mut Transaction<'_, Sqlite>, unique_id: i64) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM admin
            WHERE unique_id = ?
            "#,
        )
        .bind(unique_id)
        .execute(&mut **tx)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 获取所有管理员列表
    pub async fn list_all(pool: &sqlx::Pool<Sqlite>) -> Result<Vec<Admin>> {
        let rows = sqlx::query(
            r#"
            SELECT unique_id, id, password, invite_code
            FROM admin
            ORDER BY unique_id ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        let admins = rows
            .iter()
            .map(|row| Admin {
                unique_id: row.get("unique_id"),
                id: row.get("id"),
                password: row.get("password"),
                invite_code: row.get("invite_code"),
            })
            .collect();

        Ok(admins)
    }

    /// 检查管理员是否存在
    pub async fn exists_by_telegram_id(
        pool: &sqlx::Pool<Sqlite>,
        telegram_id: i64,
    ) -> Result<bool> {
        let result = sqlx::query(
            r#"
            SELECT 1 FROM admin WHERE id = ? LIMIT 1
            "#,
        )
        .bind(telegram_id)
        .fetch_optional(pool)
        .await?;

        Ok(result.is_some())
    }

    /// 统计管理员数量
    pub async fn count(pool: &sqlx::Pool<Sqlite>) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM admin")
            .fetch_one(pool)
            .await?;

        Ok(row.get("count"))
    }

    /// 验证管理员密码
    pub async fn verify_password(
        pool: &sqlx::Pool<Sqlite>,
        unique_id: i64,
        password: &str,
    ) -> Result<bool> {
        let row = sqlx::query(
            r#"
            SELECT password FROM admin WHERE unique_id = ?
            "#,
        )
        .bind(unique_id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            let stored_password: Option<String> = row.get("password");
            if let Some(stored_pwd) = stored_password {
                Ok(stored_pwd == password)
            } else {
                Ok(false) // 没有设置密码
            }
        } else {
            Err(AppError::business("管理员不存在"))
        }
    }

    /// 检查管理员是否已设置密码
    pub async fn has_password(pool: &sqlx::Pool<Sqlite>, unique_id: i64) -> Result<bool> {
        let row = sqlx::query(
            r#"
            SELECT password FROM admin WHERE unique_id = ?
            "#,
        )
        .bind(unique_id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            let password: Option<String> = row.get("password");
            Ok(password.is_some() && !password.unwrap().is_empty())
        } else {
            Err(AppError::business("管理员不存在"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::NamedTempFile;

    async fn setup_test_db() -> Result<Database> {
        let temp_file = NamedTempFile::new()?;
        let db_url = format!("sqlite:{}", temp_file.path().to_str().unwrap());
        Database::new(&db_url).await
    }

    #[tokio::test]
    async fn test_admin_crud() -> Result<()> {
        let db = setup_test_db().await?;
        let pool = db.pool();

        // 创建管理员
        let mut tx = db.begin_transaction().await?;
        let admin = Admin::new(123456789);
        let unique_id = AdminRepository::create(&mut tx, &admin).await?;
        tx.commit().await?;

        assert!(unique_id > 0);

        // 查找管理员
        let found_admin = AdminRepository::find_by_telegram_id(pool, 123456789).await?;
        assert!(found_admin.is_some());
        let found_admin = found_admin.unwrap();
        assert_eq!(found_admin.id, 123456789);

        // 更新密码
        let mut tx = db.begin_transaction().await?;
        let updated = AdminRepository::update_password(&mut tx, unique_id, "1234").await?;
        tx.commit().await?;
        assert!(updated);

        // 验证密码
        let verified = AdminRepository::verify_password(pool, unique_id, "1234").await?;
        assert!(verified);

        // 生成邀请码
        let mut tx = db.begin_transaction().await?;
        let invite_code = AdminRepository::generate_invite_code(&mut tx, unique_id).await?;
        tx.commit().await?;
        assert!(!invite_code.is_empty());

        // 通过邀请码查找
        let found_by_code = AdminRepository::find_by_invite_code(pool, &invite_code).await?;
        assert!(found_by_code.is_some());

        db.close().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_password_validation() {
        assert!(Admin::validate_password("1234"));
        assert!(Admin::validate_password("123456789"));
        assert!(Admin::validate_password("1234567890"));
        
        assert!(!Admin::validate_password("123")); // 太短
        assert!(!Admin::validate_password("12345678901")); // 太长
        assert!(!Admin::validate_password("12a4")); // 包含字母
        assert!(!Admin::validate_password("")); // 空字符串
    }
}