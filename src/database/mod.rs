//! 数据库模块

pub mod admin;
pub mod record;

// 重新导出数据库操作
pub use admin::AdminRepository;
pub use record::RecordRepository;

use crate::error::Result;
use sqlx::{sqlite::SqlitePool, Pool, Sqlite};
use std::path::Path;

/// 数据库连接池类型
pub type DbPool = Pool<Sqlite>;

/// 数据库管理器
#[derive(Clone)]
pub struct Database {
    pool: DbPool,
}

impl Database {
    /// 初始化数据库连接
    pub async fn new(database_url: &str) -> Result<Self> {
        // 确保数据库文件所在目录存在
        if let Some(path) = database_url.strip_prefix("sqlite:") {
            if let Some(parent) = Path::new(path).parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        // 创建连接池
        let pool = SqlitePool::connect(database_url).await?;

        let database = Self { pool };

        // 初始化数据库表结构
        database.init_tables().await?;

        Ok(database)
    }

    /// 获取数据库连接池
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// 初始化数据库表结构
    async fn init_tables(&self) -> Result<()> {
        // 创建admin表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS admin (
                unique_id INTEGER PRIMARY KEY AUTOINCREMENT,
                id INTEGER NOT NULL UNIQUE,
                password TEXT,
                invite_code TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // 创建record表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS record (
                unique_id INTEGER PRIMARY KEY AUTOINCREMENT,
                status TEXT NOT NULL DEFAULT 'pending',
                vis_id INTEGER NOT NULL,
                type TEXT NOT NULL DEFAULT 'temp',
                times INTEGER,
                start_time DATETIME,
                ended_time DATETIME,
                password TEXT,
                inviter INTEGER NOT NULL,
                update_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (inviter) REFERENCES admin (unique_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // 创建索引以提高查询性能
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_admin_id ON admin (id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_record_vis_id ON record (vis_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_record_status ON record (status)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_record_inviter ON record (inviter)")
            .execute(&self.pool)
            .await?;

        log::info!("数据库表结构初始化完成");
        Ok(())
    }

    /// 检查数据库连接
    pub async fn ping(&self) -> Result<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    /// 关闭数据库连接
    pub async fn close(self) {
        self.pool.close().await;
    }

    /// 开始事务
    pub async fn begin_transaction(&self) -> Result<sqlx::Transaction<'_, Sqlite>> {
        Ok(self.pool.begin().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_database_init() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let db_url = format!("sqlite:{}", temp_file.path().to_str().unwrap());
        
        let database = Database::new(&db_url).await?;
        
        // 测试连接
        database.ping().await?;
        
        // 验证表是否创建
        let admin_table_exists = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='admin'"
        )
        .fetch_optional(database.pool())
        .await?;
        
        assert!(admin_table_exists.is_some());

        let record_table_exists = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='record'"
        )
        .fetch_optional(database.pool())
        .await?;
        
        assert!(record_table_exists.is_some());

        database.close().await;
        Ok(())
    }
}