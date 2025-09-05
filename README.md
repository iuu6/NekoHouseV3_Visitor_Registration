# NekoHouse V3 访客登记系统

一个基于Rust和Telegram Bot的现代化访客登记系统，支持多层权限管理和多种密码授权类型。

## 🌟 功能特性

### 🔐 三级权限管理
- **超级管理员**：拥有最高权限，可以管理所有系统功能
- **管理员**：可以审批访客请求，生成邀请码，管理授权
- **访客**：可以申请授权，获取访问密码

### 🎫 多种授权类型
- **临时密码**：10分钟有效期的单次密码
- **次数密码**：2小时内可使用指定次数
- **时效密码**：指定时长有效（最长127小时）
- **指定时间密码**：在特定时间前有效
- **长期临时密码**：在有效期内可重复获取（5分钟间隔）

### 🛡️ 安全特性
- 基于KeeLoq算法的密码生成
- 时间偏移混淆，防止重放攻击
- SQLite数据库存储，数据持久化
- 完整的日志记录和错误处理

## 🚀 快速开始

### 环境要求
- Rust 1.70+
- SQLite 3.0+
- Telegram Bot Token

### 安装步骤

1. **克隆项目**
```bash
git clone https://github.com/iuu6/NekoHouseV3_Visitor_Registration.git
cd NekoHouseV3_Visitor_Registration
```

2. **配置环境**
```bash
# 复制配置文件模板
cp config.json.example config.json

# 编辑配置文件
vim config.json
```

3. **配置说明**
```json
{
  "database": {
    "path": "./data/nekohouse.db"
  },
  "telegram": {
    "bot_token": "YOUR_BOT_TOKEN_HERE"
  },
  "super_admin_ids": [
    1234567890
  ],
  "time_offset": 3600
}
```

4. **获取Bot Token**
   - 联系 [@BotFather](https://t.me/BotFather)
   - 创建新Bot：`/newbot`
   - 获取Token并填入配置文件

5. **获取用户ID**
   - 发送消息给 [@userinfobot](https://t.me/userinfobot)
   - 或启动Bot后发送 `/start`

6. **构建和运行**
```bash
# 开发模式
cargo run

# 生产模式
cargo build --release
./target/release/nekohouse_bot
```

## 📖 使用指南

### 超级管理员命令

```bash
/start                    # 查看欢迎消息和权限信息
/addadmin <用户ID>        # 添加新管理员
/editpasswd <密码>        # 修改管理密码（4-10位数字）
/geninvite               # 生成/更新邀请码
/revoke <目标>           # 撤销授权
/getpassword            # 获取临时密码
```

### 管理员命令

```bash
/start                    # 查看欢迎消息和权限信息  
/editpasswd <密码>        # 修改管理密码（4-10位数字）
/geninvite               # 生成/更新邀请码
/revoke <目标>           # 撤销授权
/getpassword            # 获取临时密码
```

### 访客命令

```bash
/start                    # 查看欢迎消息
/req <邀请码>            # 申请访客授权
/getpassword            # 获取访问密码
```

### 撤销授权格式

```bash
# 撤销用户的所有授权
/revoke 123456789
/revoke user 123456789

# 撤销指定记录
/revoke record 1
/revoke r1
```

### 管理员审批流程

1. 访客发送 `/req <邀请码>` 申请授权
2. 管理员收到审批通知，点击"批准"或"拒绝"
3. 选择授权类型：
   - **临时密码**：直接批准，10分钟有效
   - **次数密码**：选择使用次数（1-31次），2小时有效
   - **时效密码**：选择时长（1-127小时）
   - **指定时间**：发送格式 `期间 <记录ID> YYYY-MM-DD HH`
   - **长期临时**：发送格式 `长期 <记录ID> YYYY-MM-DD HH:MM`

## 🏗️ 项目结构

```
src/
├── main.rs                 # 程序入口
├── lib.rs                  # 库文件
├── config.rs               # 配置管理
├── error.rs                # 错误处理
├── types.rs                # 类型定义
├── auth/                   # 认证授权模块
│   ├── mod.rs
│   ├── password_service.rs # 密码服务
│   └── user_service.rs     # 用户服务
├── database/               # 数据库模块
│   ├── mod.rs
│   ├── admin.rs           # 管理员表操作
│   └── record.rs          # 记录表操作
├── bot/                   # Bot框架
│   ├── mod.rs
│   └── bot.rs            # Bot主体
├── handlers/              # 消息处理器
│   ├── mod.rs
│   ├── start.rs          # /start命令
│   ├── admin.rs          # 管理员命令
│   ├── visitor.rs        # 访客命令
│   ├── callback.rs       # 回调处理
│   ├── text.rs           # 文本消息
│   └── member.rs         # 成员更新
└── utils/                # 工具模块
    ├── mod.rs
    └── gen_password/     # 密码生成算法
        ├── lib.rs
        ├── config.rs
        ├── keeloq_crypto.rs
        ├── temp_password.rs
        ├── times_password.rs
        ├── limited_password.rs
        └── period_password.rs
```

## 🗄️ 数据库结构

### admin表
```sql
CREATE TABLE admin (
    unique_id INTEGER PRIMARY KEY AUTOINCREMENT,  -- 数据库唯一ID
    id INTEGER NOT NULL UNIQUE,                   -- Telegram用户ID
    password TEXT,                                -- 管理密码（4-10位数字）
    invite_code TEXT,                            -- 邀请码（UUID）
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### record表
```sql
CREATE TABLE record (
    unique_id INTEGER PRIMARY KEY AUTOINCREMENT,  -- 数据库唯一ID
    status TEXT NOT NULL DEFAULT 'pending',       -- 状态（pending/auth/revoked）
    vis_id INTEGER NOT NULL,                      -- 访客Telegram ID
    type TEXT NOT NULL DEFAULT 'temp',            -- 授权类型
    times INTEGER,                                -- 使用次数
    start_time DATETIME,                          -- 开始时间
    ended_time DATETIME,                          -- 结束时间
    password TEXT,                                -- 密码列表（JSON）
    inviter INTEGER NOT NULL,                     -- 邀请者ID
    update_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (inviter) REFERENCES admin (unique_id)
);
```

## 🔧 开发指南

### 环境设置
```bash
# 安装依赖
cargo build

# 运行测试
cargo test

# 代码格式化
cargo fmt

# 代码检查
cargo clippy
```

### 日志配置
```bash
# 设置日志级别
export RUST_LOG=debug

# 或在运行时指定
RUST_LOG=info cargo run
```

### 配置文件路径
```bash
# 自定义配置文件路径
export CONFIG_PATH=/path/to/your/config.json
```

## 🛠️ 技术栈

- **语言**：Rust
- **异步运行时**：Tokio
- **Telegram Bot**：Teloxide
- **数据库**：SQLite + SQLx
- **序列化**：Serde
- **日志**：log + env_logger
- **加密算法**：KeeLoq (自实现)

## 🚨 安全注意事项

1. **保护Bot Token**：不要将Token提交到版本控制系统
2. **定期备份数据库**：SQLite文件包含所有用户数据
3. **时间偏移配置**：合理设置时间偏移以增加安全性
4. **权限控制**：谨慎分配超级管理员权限
5. **日志监控**：定期检查系统日志

## 🤝 贡献指南

1. Fork项目
2. 创建功能分支：`git checkout -b feature/amazing-feature`
3. 提交更改：`git commit -m 'Add amazing feature'`
4. 推送分支：`git push origin feature/amazing-feature`
5. 提交Pull Request

## 🐛 问题反馈

如果您发现任何问题或有功能建议，请提交 [Issue](https://github.com/iuu6/NekoHouseV3_Visitor_Registration/issues)

## 📞 支持

- 技术支持：请提交Issue
- 功能请求：请提交Feature Request
- 安全问题：请私信联系维护者

---

<div align="center">

**NekoHouse V3 访客登记系统** - 让访客管理更简单、更安全

Made with ❤️ by NekoHouse Team

</div>
