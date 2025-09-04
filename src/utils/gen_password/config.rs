//! 配置模块
//! 包含应用程序的常量和配置项

/// 应用程序配置
pub struct AppConfig;

impl AppConfig {
    /// 最小管理员密码长度
    pub const MIN_ADMIN_PWD_LENGTH: usize = 4;
    
    /// 密码前缀（5e9）
    pub const PASSWORD_PREFIX: u64 = 5000000000;
    
    /// 临时密码配置
    pub const TEMP_PWD_TIME_WINDOW_MS: i64 = 4000;  // 4秒时间窗口
    pub const TEMP_PWD_EXPIRE_MS: i64 = 600000;     // 10分钟有效期
    pub const TEMP_PWD_DEFAULT_TOLERANCE: u32 = 1;   // 默认容忍窗口
    
    /// 次数密码配置
    pub const TIMES_PWD_MIN_TIMES: u32 = 1;
    pub const TIMES_PWD_MAX_TIMES: u32 = 31;
    pub const TIMES_PWD_EXPIRE_HOURS: i64 = 20;     // 20小时有效期
    pub const TIMES_PWD_EXPIRE_MS: i64 = 72000000;  // 20小时 = 72000000毫秒
    pub const TIMES_PWD_MAGIC_CONSTANT: u32 = 1073741824; // 0x40000000
    pub const TIMES_PWD_WINDOW_MASK: u32 = 0xFFFFFFE0;
    pub const TIMES_PWD_DEFAULT_TOLERANCE: u32 = 2;
    
    /// 限时密码配置
    pub const LIMITED_PWD_MAX_HOURS: u32 = 127;
    pub const LIMITED_PWD_TIME_WINDOW_MS: i64 = 1800000; // 30分钟时间窗口
    pub const LIMITED_PWD_MAGIC_CONSTANT: u32 = 2147483648; // 0x80000000
    pub const LIMITED_PWD_DEFAULT_TOLERANCE: u32 = 2;
    
    /// 周期密码配置
    pub const PERIOD_PWD_MAGIC_CONSTANT: u32 = 3221225472; // 0xC0000000
    pub const PERIOD_PWD_UTC8_OFFSET: i64 = 28800; // 8小时 = 28800秒
    pub const PERIOD_PWD_SECONDS_PER_DAY: i64 = 86400;
    pub const PERIOD_PWD_SECONDS_PER_HOUR: i64 = 3600;
    pub const PERIOD_PWD_MAX_HOURS: u32 = 32768;
    pub const PERIOD_PWD_DEFAULT_TOLERANCE: u32 = 1;
}

/// 菜单选项枚举
#[derive(Debug, Clone, PartialEq)]
pub enum MenuOption {
    TempPassword,
    TimesPassword,
    LimitedPassword,
    PeriodPassword,
    VerifyPassword,
    Exit,
    Invalid,
}

impl From<&str> for MenuOption {
    fn from(choice: &str) -> Self {
        match choice.trim() {
            "1" => MenuOption::TempPassword,
            "2" => MenuOption::TimesPassword,
            "3" => MenuOption::LimitedPassword,
            "4" => MenuOption::PeriodPassword,
            "5" => MenuOption::VerifyPassword,
            "6" => MenuOption::Exit,
            _ => MenuOption::Invalid,
        }
    }
}

/// 验证结果枚举
#[derive(Debug, Clone)]
pub enum VerificationResult {
    Success { password_type: String, additional_info: Option<String> },
    Failure { reason: String },
}

/// 时间单位枚举
#[derive(Debug, Clone, PartialEq)]
pub enum TimeUnit {
    Milliseconds,
    Seconds,
    Minutes,
    Hours,
    Days,
}

/// 格式化时间帮助器
pub struct TimeFormatter;

impl TimeFormatter {
    /// 格式化毫秒为可读字符串
    pub fn format_duration_ms(ms: i64) -> String {
        let days = ms / (1000 * 60 * 60 * 24);
        let hours = (ms % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60);
        let minutes = (ms % (1000 * 60 * 60)) / (1000 * 60);
        let seconds = (ms % (1000 * 60)) / 1000;
        
        if days > 0 {
            format!("{}天{}小时{}分钟", days, hours, minutes)
        } else if hours > 0 {
            format!("{}小时{}分钟", hours, minutes)
        } else if minutes > 0 {
            format!("{}分钟{}秒", minutes, seconds)
        } else {
            format!("{}秒", seconds)
        }
    }
    
    /// 格式化小时和分钟
    pub fn format_hours_minutes(hours: u32, minutes: u32) -> String {
        match (hours, minutes) {
            (0, m) => format!("{}分钟", m),
            (h, 0) if h == 1 => "1小时".to_string(),
            (h, 0) => format!("{}小时", h),
            (1, m) => format!("1小时{}分钟", m),
            (h, m) => format!("{}小时{}分钟", h, m),
        }
    }
}

/// 验证器配置
pub struct ValidatorConfig;

impl ValidatorConfig {
    /// 获取临时密码验证容忍度
    pub fn temp_password_tolerance() -> u32 {
        AppConfig::TEMP_PWD_DEFAULT_TOLERANCE
    }
    
    /// 获取次数密码验证容忍度
    pub fn times_password_tolerance() -> u32 {
        AppConfig::TIMES_PWD_DEFAULT_TOLERANCE
    }
    
    /// 获取限时密码验证容忍度
    pub fn limited_password_tolerance() -> u32 {
        AppConfig::LIMITED_PWD_DEFAULT_TOLERANCE
    }
    
    /// 获取周期密码验证容忍度
    pub fn period_password_tolerance() -> u32 {
        AppConfig::PERIOD_PWD_DEFAULT_TOLERANCE
    }
}

/// 输入验证帮助器
pub struct InputValidator;

impl InputValidator {
    /// 验证管理员密码长度
    pub fn validate_admin_password(password: &str) -> Result<(), String> {
        if password.len() < AppConfig::MIN_ADMIN_PWD_LENGTH {
            Err(format!("管理员密码至少需要{}位", AppConfig::MIN_ADMIN_PWD_LENGTH))
        } else {
            Ok(())
        }
    }
    
    /// 验证使用次数
    pub fn validate_use_times(times: u32) -> Result<(), String> {
        if times < AppConfig::TIMES_PWD_MIN_TIMES || times > AppConfig::TIMES_PWD_MAX_TIMES {
            Err(format!("使用次数必须在{}-{}之间", AppConfig::TIMES_PWD_MIN_TIMES, AppConfig::TIMES_PWD_MAX_TIMES))
        } else {
            Ok(())
        }
    }
    
    /// 验证小时数
    pub fn validate_hours(hours: u32) -> Result<(), String> {
        if hours > AppConfig::LIMITED_PWD_MAX_HOURS {
            Err(format!("小时数不能超过{}", AppConfig::LIMITED_PWD_MAX_HOURS))
        } else {
            Ok(())
        }
    }
    
    /// 验证分钟数（只允许0或30）
    pub fn validate_minutes(minutes: u32) -> Result<(), String> {
        if minutes != 0 && minutes != 30 {
            Err("分钟数只能是0或30".to_string())
        } else {
            Ok(())
        }
    }
    
    /// 验证月份
    pub fn validate_month(month: u32) -> Result<(), String> {
        if month < 1 || month > 12 {
            Err("月份必须在1-12之间".to_string())
        } else {
            Ok(())
        }
    }
    
    /// 验证日期
    pub fn validate_day(day: u32) -> Result<(), String> {
        if day < 1 || day > 31 {
            Err("日期必须在1-31之间".to_string())
        } else {
            Ok(())
        }
    }
    
    /// 验证小时（0-23）
    pub fn validate_hour(hour: u32) -> Result<(), String> {
        if hour > 23 {
            Err("小时必须在0-23之间".to_string())
        } else {
            Ok(())
        }
    }
}