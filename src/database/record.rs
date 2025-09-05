//! 访客记录表操作模块

use crate::error::{AppError, Result};
use crate::types::{AuthStatus, AuthType, Record};
use chrono::{DateTime, Utc};
use serde_json;
use sqlx::{Row, Sqlite, Transaction};

/// 访客记录数据库操作
pub struct RecordRepository;

impl RecordRepository {
    /// 创建新访客记录
    pub async fn create(tx: &mut Transaction<'_, Sqlite>, record: &Record) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO record (status, vis_id, type, times, start_time, ended_time, password, inviter, update_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(record.status.as_str())
        .bind(record.vis_id)
        .bind(record.auth_type.as_str())
        .bind(record.times)
        .bind(record.start_time)
        .bind(record.ended_time)
        .bind(&record.password)
        .bind(record.inviter)
        .bind(record.update_at)
        .execute(&mut **tx)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// 通过unique_id查找记录
    pub async fn find_by_id(
        pool: &sqlx::Pool<Sqlite>,
        unique_id: i64,
    ) -> Result<Option<Record>> {
        let row = sqlx::query(
            r#"
            SELECT unique_id, status, vis_id, type, times, start_time, ended_time, password, inviter, update_at
            FROM record
            WHERE unique_id = ?
            "#,
        )
        .bind(unique_id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Self::row_to_record(row)?))
        } else {
            Ok(None)
        }
    }

    /// 查找访客的待处理请求
    pub async fn find_pending_by_visitor(
        pool: &sqlx::Pool<Sqlite>,
        vis_id: i64,
    ) -> Result<Option<Record>> {
        let row = sqlx::query(
            r#"
            SELECT unique_id, status, vis_id, type, times, start_time, ended_time, password, inviter, update_at
            FROM record
            WHERE vis_id = ? AND status = 'pending'
            ORDER BY update_at DESC
            LIMIT 1
            "#,
        )
        .bind(vis_id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Self::row_to_record(row)?))
        } else {
            Ok(None)
        }
    }

    /// 查找访客的所有活跃授权
    pub async fn find_active_by_visitor(
        pool: &sqlx::Pool<Sqlite>,
        vis_id: i64,
    ) -> Result<Vec<Record>> {
        // 获取所有状态为'auth'的记录，在应用层判断是否过期
        let rows = sqlx::query(
            r#"
            SELECT unique_id, status, vis_id, type, times, start_time, ended_time, password, inviter, update_at
            FROM record
            WHERE vis_id = ? AND status = 'auth'
            ORDER BY update_at DESC
            "#,
        )
        .bind(vis_id)
        .fetch_all(pool)
        .await?;

        let mut expired_ids = Vec::new();
        let mut active_records = Vec::new();

        // 在应用层检查每个记录是否真正活跃
        for row in rows {
            let record = Self::row_to_record(row)?;
            if record.is_active() {
                active_records.push(record);
            } else {
                // 记录已过期，标记为需要清理
                expired_ids.push(record.unique_id);
            }
        }

        // 批量清理过期的记录
        if !expired_ids.is_empty() {
            let expired_count = expired_ids.len();
            let mut tx = pool.begin().await?;
            for expired_id in expired_ids {
                sqlx::query(
                    r#"
                    UPDATE record
                    SET status = 'revoked', update_at = ?
                    WHERE unique_id = ?
                    "#,
                )
                .bind(Utc::now())
                .bind(expired_id)
                .execute(&mut *tx)
                .await?;
            }
            tx.commit().await?;
            log::info!("为用户 {} 清理了 {} 个过期授权", vis_id, expired_count);
        }

        Ok(active_records)
    }

    /// 查找管理员的所有记录
    pub async fn find_by_inviter(
        pool: &sqlx::Pool<Sqlite>,
        inviter_id: i64,
    ) -> Result<Vec<Record>> {
        let rows = sqlx::query(
            r#"
            SELECT unique_id, status, vis_id, type, times, start_time, ended_time, password, inviter, update_at
            FROM record
            WHERE inviter = ?
            ORDER BY update_at DESC
            "#,
        )
        .bind(inviter_id)
        .fetch_all(pool)
        .await?;

        let mut records = Vec::new();
        for row in rows {
            records.push(Self::row_to_record(row)?);
        }

        Ok(records)
    }

    /// 获取所有活跃的授权记录
    pub async fn find_all_active(pool: &sqlx::Pool<Sqlite>) -> Result<Vec<Record>> {
        let rows = sqlx::query(
            r#"
            SELECT unique_id, status, vis_id, type, times, start_time, ended_time, password, inviter, update_at
            FROM record
            WHERE status = 'auth' AND (ended_time IS NULL OR ended_time > CURRENT_TIMESTAMP)
            ORDER BY update_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        let mut records = Vec::new();
        for row in rows {
            records.push(Self::row_to_record(row)?);
        }

        Ok(records)
    }

    /// 更新记录状态
    pub async fn update_status(
        tx: &mut Transaction<'_, Sqlite>,
        unique_id: i64,
        status: AuthStatus,
    ) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE record
            SET status = ?, update_at = CURRENT_TIMESTAMP
            WHERE unique_id = ?
            "#,
        )
        .bind(status.as_str())
        .bind(unique_id)
        .execute(&mut **tx)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 批准授权（更新记录详情）
    pub async fn approve_authorization(
        tx: &mut Transaction<'_, Sqlite>,
        unique_id: i64,
        auth_type: AuthType,
        start_time: Option<DateTime<Utc>>,
        ended_time: Option<DateTime<Utc>>,
        times: Option<i32>,
    ) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE record
            SET status = 'auth', type = ?, start_time = ?, ended_time = ?, times = ?, update_at = CURRENT_TIMESTAMP
            WHERE unique_id = ?
            "#,
        )
        .bind(auth_type.as_str())
        .bind(start_time)
        .bind(ended_time)
        .bind(times)
        .bind(unique_id)
        .execute(&mut **tx)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 添加密码到记录
    pub async fn add_password(
        tx: &mut Transaction<'_, Sqlite>,
        unique_id: i64,
        new_password: &str,
    ) -> Result<bool> {
        // 先获取现有密码
        let row = sqlx::query(
            r#"
            SELECT password FROM record WHERE unique_id = ?
            "#,
        )
        .bind(unique_id)
        .fetch_optional(&mut **tx)
        .await?;

        let mut passwords: Vec<String> = if let Some(row) = row {
            let password_json: Option<String> = row.get("password");
            if let Some(json) = password_json {
                serde_json::from_str(&json).unwrap_or_else(|_| Vec::new())
            } else {
                Vec::new()
            }
        } else {
            return Ok(false); // 记录不存在
        };

        passwords.push(new_password.to_string());
        let updated_passwords = serde_json::to_string(&passwords)?;

        let result = sqlx::query(
            r#"
            UPDATE record
            SET password = ?, update_at = CURRENT_TIMESTAMP
            WHERE unique_id = ?
            "#,
        )
        .bind(updated_passwords)
        .bind(unique_id)
        .execute(&mut **tx)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 撤销所有用户的授权
    pub async fn revoke_all_by_visitor(
        tx: &mut Transaction<'_, Sqlite>,
        vis_id: i64,
    ) -> Result<usize> {
        let result = sqlx::query(
            r#"
            UPDATE record
            SET status = 'revoked', update_at = CURRENT_TIMESTAMP
            WHERE vis_id = ? AND status = 'auth'
            "#,
        )
        .bind(vis_id)
        .execute(&mut **tx)
        .await?;

        Ok(result.rows_affected() as usize)
    }

    /// 撤销指定记录
    pub async fn revoke_by_id(
        tx: &mut Transaction<'_, Sqlite>,
        unique_id: i64,
    ) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE record
            SET status = 'revoked', update_at = CURRENT_TIMESTAMP
            WHERE unique_id = ? AND status IN ('pending', 'auth')
            "#,
        )
        .bind(unique_id)
        .execute(&mut **tx)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 清理用户的过期授权
    pub async fn cleanup_expired_authorizations(
        tx: &mut Transaction<'_, Sqlite>,
        vis_id: i64,
    ) -> Result<usize> {
        let result = sqlx::query(
            r#"
            UPDATE record
            SET status = 'revoked', update_at = CURRENT_TIMESTAMP
            WHERE vis_id = ? AND status = 'auth' AND ended_time IS NOT NULL AND ended_time <= CURRENT_TIMESTAMP
            "#,
        )
        .bind(vis_id)
        .execute(&mut **tx)
        .await?;

        Ok(result.rows_affected() as usize)
    }

    /// 检查用户是否有活跃授权
    pub async fn has_active_authorization(
        pool: &sqlx::Pool<Sqlite>,
        vis_id: i64,
    ) -> Result<bool> {
        // 获取所有状态为'auth'的记录，在应用层判断是否过期
        let rows = sqlx::query(
            r#"
            SELECT unique_id, status, vis_id, type, times, start_time, ended_time, password, inviter, update_at
            FROM record
            WHERE vis_id = ? AND status = 'auth'
            "#,
        )
        .bind(vis_id)
        .fetch_all(pool)
        .await?;

        let mut expired_ids = Vec::new();
        let mut has_active = false;

        // 在应用层检查每个记录是否真正活跃
        for row in rows {
            let record = Self::row_to_record(row)?;
            if record.is_active() {
                has_active = true;
            } else {
                // 记录已过期，标记为需要清理
                expired_ids.push(record.unique_id);
            }
        }

        // 批量清理过期的记录
        if !expired_ids.is_empty() {
            let expired_count = expired_ids.len();
            let mut tx = pool.begin().await?;
            for expired_id in expired_ids {
                sqlx::query(
                    r#"
                    UPDATE record
                    SET status = 'revoked', update_at = ?
                    WHERE unique_id = ?
                    "#,
                )
                .bind(Utc::now())
                .bind(expired_id)
                .execute(&mut *tx)
                .await?;
            }
            tx.commit().await?;
            log::info!("为用户 {} 清理了 {} 个过期授权", vis_id, expired_count);
        }

        Ok(has_active)
    }

    /// 检查用户是否有待处理请求
    pub async fn has_pending_request(
        pool: &sqlx::Pool<Sqlite>,
        vis_id: i64,
    ) -> Result<bool> {
        let result = sqlx::query(
            r#"
            SELECT 1 FROM record
            WHERE vis_id = ? AND status = 'pending'
            LIMIT 1
            "#,
        )
        .bind(vis_id)
        .fetch_optional(pool)
        .await?;

        Ok(result.is_some())
    }

    /// 删除记录（慎用）
    pub async fn delete(tx: &mut Transaction<'_, Sqlite>, unique_id: i64) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM record WHERE unique_id = ?
            "#,
        )
        .bind(unique_id)
        .execute(&mut **tx)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 获取记录统计信息
    pub async fn get_statistics(
        pool: &sqlx::Pool<Sqlite>,
    ) -> Result<(i64, i64, i64, i64)> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending,
                SUM(CASE WHEN status = 'auth' THEN 1 ELSE 0 END) as authorized,
                SUM(CASE WHEN status = 'revoked' THEN 1 ELSE 0 END) as revoked
            FROM record
            "#,
        )
        .fetch_one(pool)
        .await?;

        Ok((
            row.get("total"),
            row.get("pending"),
            row.get("authorized"),
            row.get("revoked"),
        ))
    }

    /// 将数据库行转换为Record结构
    fn row_to_record(row: sqlx::sqlite::SqliteRow) -> Result<Record> {
        let status_str: String = row.get("status");
        let status = AuthStatus::from_str(&status_str)
            .ok_or_else(|| AppError::business("无效的状态值"))?;

        let type_str: String = row.get("type");
        let auth_type = AuthType::from_str(&type_str)
            .ok_or_else(|| AppError::business("无效的授权类型"))?;

        Ok(Record {
            unique_id: row.get("unique_id"),
            status,
            vis_id: row.get("vis_id"),
            auth_type,
            times: row.get("times"),
            start_time: row.get("start_time"),
            ended_time: row.get("ended_time"),
            password: row.get("password"),
            inviter: row.get("inviter"),
            update_at: row.get("update_at"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{admin::AdminRepository, Database};
    use crate::types::Admin;
    use tempfile::NamedTempFile;

    async fn setup_test_db() -> Result<(Database, i64)> {
        let temp_file = NamedTempFile::new()?;
        let db_url = format!("sqlite:{}", temp_file.path().to_str().unwrap());
        let db = Database::new(&db_url).await?;

        // 创建一个测试管理员
        let mut tx = db.begin_transaction().await?;
        let admin = Admin::new(123456789);
        let admin_id = AdminRepository::create(&mut tx, &admin).await?;
        tx.commit().await?;

        Ok((db, admin_id))
    }

    #[tokio::test]
    async fn test_record_crud() -> Result<()> {
        let (db, admin_id) = setup_test_db().await?;
        let pool = db.pool();

        // 创建访客记录
        let mut tx = db.begin_transaction().await?;
        let record = Record::new(987654321, admin_id);
        let record_id = RecordRepository::create(&mut tx, &record).await?;
        tx.commit().await?;

        assert!(record_id > 0);

        // 查找记录
        let found_record = RecordRepository::find_by_id(pool, record_id).await?;
        assert!(found_record.is_some());
        let found_record = found_record.unwrap();
        assert_eq!(found_record.vis_id, 987654321);
        assert_eq!(found_record.status, AuthStatus::Pending);

        // 批准授权
        let mut tx = db.begin_transaction().await?;
        let approved = RecordRepository::approve_authorization(
            &mut tx,
            record_id,
            AuthType::Temp,
            Some(Utc::now()),
            Some(Utc::now() + chrono::Duration::minutes(10)),
            None,
        ).await?;
        tx.commit().await?;
        assert!(approved);

        // 添加密码
        let mut tx = db.begin_transaction().await?;
        let password_added = RecordRepository::add_password(&mut tx, record_id, "5001234567").await?;
        tx.commit().await?;
        assert!(password_added);

        // 检查活跃授权
        let active_records = RecordRepository::find_active_by_visitor(pool, 987654321).await?;
        assert!(!active_records.is_empty());

        // 撤销授权
        let mut tx = db.begin_transaction().await?;
        let revoked = RecordRepository::revoke_by_id(&mut tx, record_id).await?;
        tx.commit().await?;
        assert!(revoked);

        db.close().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_visitor_constraints() -> Result<()> {
        let (db, admin_id) = setup_test_db().await?;
        let pool = db.pool();
        let visitor_id = 555666777;

        // 检查是否有待处理请求
        let has_pending = RecordRepository::has_pending_request(pool, visitor_id).await?;
        assert!(!has_pending);

        // 创建待处理请求
        let mut tx = db.begin_transaction().await?;
        let record = Record::new(visitor_id, admin_id);
        let _record_id = RecordRepository::create(&mut tx, &record).await?;
        tx.commit().await?;

        // 现在应该有待处理请求
        let has_pending = RecordRepository::has_pending_request(pool, visitor_id).await?;
        assert!(has_pending);

        // 检查是否有活跃授权
        let has_active = RecordRepository::has_active_authorization(pool, visitor_id).await?;
        assert!(!has_active);

        db.close().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_statistics() -> Result<()> {
        let (db, admin_id) = setup_test_db().await?;
        let pool = db.pool();

        // 创建一些测试记录
        let mut tx = db.begin_transaction().await?;
        
        // 创建待处理记录
        let pending_record = Record::new(111111111, admin_id);
        RecordRepository::create(&mut tx, &pending_record).await?;
        
        // 创建已授权记录
        let mut auth_record = Record::new(222222222, admin_id);
        auth_record.status = AuthStatus::Auth;
        RecordRepository::create(&mut tx, &auth_record).await?;
        
        tx.commit().await?;

        // 获取统计信息
        let (total, pending, authorized, revoked) = RecordRepository::get_statistics(pool).await?;
        assert_eq!(total, 2);
        assert_eq!(pending, 1);
        assert_eq!(authorized, 1);
        assert_eq!(revoked, 0);

        db.close().await;
        Ok(())
    }
}