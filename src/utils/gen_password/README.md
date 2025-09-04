# M10 Door Password Generator

A Rust implementation of four different password generation algorithms based on KeeLoq encryption, designed for door access control systems. All timestamps use UTC+8 timezone (Beijing Time).

## Features

- **Temporary Password**: 4-second time window, valid for 10 minutes
- **Times Password**: Usable for specified times (1-31), valid for 20 hours
- **Limited Password**: Valid for specified duration (hours + 0/30 minutes)
- **Period Password**: Valid until specified date and time

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
chrono = { version = "0.4", features = ["serde"] }
```

Or clone and build from source:

```bash
git clone <repository-url>
cd M10_Door_Password
cargo build --release
```

## Quick Start

### As a Library

```rust
use password_algorithms::*;

fn main() {
    let admin_password = "123456";
    
    // Generate temporary password
    if let Ok((password, expire_time, message)) = generate_temp_password(admin_password) {
        println!("Password: {}", password);
        println!("Expires: {}", expire_time);
        
        // Verify the password
        if verify_temp_password(&password, admin_password) {
            println!("Password is valid!");
        }
    }
    
    // Use unified generator
    let generator = UnifiedPasswordGenerator::new();
    let result = generator.generate(admin_password, PasswordType::Times(5)).unwrap();
    println!("Times password: {}", result.password);
}
```

### As a CLI Application

```bash
# Run the interactive demo
cargo run

# Or run the binary directly
./target/release/M10_Door_Password
```

## Algorithm Details

### 1. Temporary Password ([`TempPasswordGenerator`](lib.rs:52))

- **Time Window**: 4 seconds
- **Validity**: 10 minutes from generation
- **Use Case**: Quick temporary access
- **Password Format**: 10+ digits starting with 5

```rust
use password_algorithms::TempPasswordGenerator;

let generator = TempPasswordGenerator::new();
let (password, expire_time, message) = generator.generate("admin123")?;
```

### 2. Times Password ([`TimesPasswordGenerator`](lib.rs:53))

- **Usage Limit**: 1-31 times
- **Validity**: 20 hours from generation
- **Use Case**: Multiple entries with count limit
- **Password Format**: 10+ digits starting with 5

```rust
use password_algorithms::TimesPasswordGenerator;

let generator = TimesPasswordGenerator::new();
let (password, expire_time, message) = generator.generate("admin123", 5)?; // 5 uses
```

### 3. Limited Password ([`LimitedPasswordGenerator`](lib.rs:54))

- **Duration**: 0-127 hours + 0/30 minutes
- **Time Window**: 30-minute windows
- **Use Case**: Access for specific duration
- **Password Format**: 10+ digits starting with 5

```rust
use password_algorithms::LimitedPasswordGenerator;

let generator = LimitedPasswordGenerator::new();
let (password, expire_time, message) = generator.generate("admin123", 2, 30)?; // 2h30m
```

### 4. Period Password ([`PeriodPasswordGenerator`](lib.rs:55))

- **End Time**: Specific date and hour
- **Precision**: Hour-level accuracy
- **Use Case**: Access until specific deadline
- **Password Format**: 10+ digits starting with 5

```rust
use password_algorithms::PeriodPasswordGenerator;

let generator = PeriodPasswordGenerator::new();
// Valid until 2024-12-25 18:00:00
let (password, expire_time, message) = generator.generate("admin123", 2024, 12, 25, 18)?;

// Or use string format
let (password, expire_time, message) = 
    generator.generate_from_string("admin123", "2024-12-25 18:00:00")?;
```

## Unified API

The [`UnifiedPasswordGenerator`](lib.rs:51) provides a single interface for all password types:

```rust
use password_algorithms::{UnifiedPasswordGenerator, PasswordType};

let generator = UnifiedPasswordGenerator::new();

// Generate different types of passwords
let temp_result = generator.generate("admin", PasswordType::Temporary)?;
let times_result = generator.generate("admin", PasswordType::Times(3))?;
let limited_result = generator.generate("admin", PasswordType::Limited(1, 30))?;
let period_result = generator.generate("admin", PasswordType::Period(2024, 12, 31, 23))?;

// Verify any password (auto-detect type)
if let Some(password_type) = generator.verify(&password, "admin") {
    println!("Valid password of type: {:?}", password_type);
    
    // Get remaining time
    if let Some(remaining) = generator.get_remaining_time(&password, "admin") {
        println!("Time remaining: {}", remaining);
    }
}
```

## API Reference

### Core Functions

#### Generation Functions
- [`generate_temp_password(admin_pwd)`](lib.rs:19) → `Result<(String, String, String), String>`
- [`generate_times_password(admin_pwd, times)`](lib.rs:20) → `Result<(String, String, String), String>`
- [`generate_limited_password(admin_pwd, hours, minutes)`](lib.rs:21) → `Result<(String, String, String), String>`
- [`generate_period_password(admin_pwd, year, month, day, hour)`](lib.rs:22) → `Result<(String, String, String), String>`

#### Verification Functions
- [`verify_temp_password(password, admin_pwd)`](lib.rs:19) → `bool`
- [`verify_times_password(password, admin_pwd)`](lib.rs:20) → `Option<u32>`
- [`verify_limited_password(password, admin_pwd)`](lib.rs:21) → `Option<(u32, u32)>`
- [`verify_period_password(password, admin_pwd)`](lib.rs:22) → `Option<String>`

#### Time Check Functions
- [`check_password_remaining_time(password, admin_pwd)`](lib.rs:20) → `Option<(String, u32)>`
- [`check_limited_password_remaining_time(password, admin_pwd)`](lib.rs:21) → `Option<(String, u32, u32)>`
- [`check_period_password_remaining_time(password, admin_pwd)`](lib.rs:22) → `Option<(String, String)>`

### Password Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PasswordType {
    Temporary,
    Times(u32),                    // number of uses
    Limited(u32, u32),             // hours, minutes
    Period(u32, u32, u32, u32),    // year, month, day, hour
}
```

### Password Result

```rust
#[derive(Debug, Clone)]
pub struct PasswordResult {
    pub password: String,       // Generated password
    pub expire_time: String,    // Formatted expiry time
    pub message: String,        // Description message
    pub password_type: PasswordType,  // Type of password
}
```

## Error Handling

All generation functions return `Result<(String, String, String), String>` where:
- **Success**: `(password, expire_time, message)`
- **Error**: Descriptive error message

Common errors:
- `"管理员密码至少需要4位"` - Admin password too short
- `"使用次数必须在1-31之间"` - Invalid usage count
- `"小时数不能超过127"` - Hours out of range
- `"分钟数只能是0或30"` - Invalid minutes (must be 0 or 30)
- `"结束时间必须晚于当前时间"` - End time in the past

## Security Features

### KeeLoq Encryption
- Uses authentic KeeLoq algorithm implementation
- 528-round encryption process
- Configurable S-Box lookup table
- Key derivation from admin password

### Time Synchronization
- All operations use UTC+8 (Beijing Time)
- Configurable tolerance windows for verification
- Automatic time window alignment

### Password Format
- All passwords start with prefix `5`
- Minimum 10 digits long
- Based on encrypted timestamp + parameters

## Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test temp_password
cargo test times_password
cargo test limited_password
cargo test period_password

# Run with output
cargo test -- --nocapture
```

## Examples

### Basic Usage

```rust
use password_algorithms::*;

// Generate a 2-hour temporary access password
let (password, expire_time, _) = generate_limited_password("mykey", 2, 0)?;
println!("Password: {} (expires: {})", password, expire_time);

// Verify and get remaining time
if let Some((hours, minutes)) = verify_limited_password(&password, "mykey") {
    println!("Valid for {}h{}m", hours, minutes);
    
    if let Some((remaining, _, _)) = check_limited_password_remaining_time(&password, "mykey") {
        println!("Time left: {}", remaining);
    }
}
```

### Advanced Usage with Unified Generator

```rust
use password_algorithms::{UnifiedPasswordGenerator, PasswordType};

let generator = UnifiedPasswordGenerator::new();
let admin_key = "secure123";

// Generate different password types
let passwords = vec![
    generator.generate(admin_key, PasswordType::Temporary)?,
    generator.generate(admin_key, PasswordType::Times(10))?,
    generator.generate(admin_key, PasswordType::Limited(4, 30))?,
    generator.generate(admin_key, PasswordType::Period(2024, 12, 31, 23))?,
];

// Verify all passwords
for result in &passwords {
    if let Some(pwd_type) = generator.verify(&result.password, admin_key) {
        println!("✅ {} password valid: {}", 
                 format!("{:?}", pwd_type), 
                 result.password);
        
        if let Some(remaining) = generator.get_remaining_time(&result.password, admin_key) {
            println!("   Time remaining: {}", remaining);
        }
    }
}
```

### Time Window Information

```rust
// Get current time window information
let (window, start, end) = TempPasswordGenerator::get_current_window_info();
println!("Temp password window: {} ({} - {})", window, start, end);

let (current, aligned, start, expire) = TimesPasswordGenerator::get_current_window_info();
println!("Times password window: current={}, aligned={}", current, aligned);
println!("Window: {} to {}", start, expire);

let (window, start, end) = LimitedPasswordGenerator::get_current_window_info();
println!("Limited password 30min window: {} ({} - {})", window, start, end);
```

## Architecture

```
src/
├── lib.rs                    # Main library exports and unified generator
├── main.rs                   # CLI application with interactive demo
├── keeloq_crypto.rs         # KeeLoq encryption implementation
├── temp_password.rs         # Temporary password algorithm
├── times_password.rs        # Times-limited password algorithm  
├── limited_password.rs      # Duration-limited password algorithm
└── period_password.rs       # Period-based password algorithm
```

## Dependencies

- [`chrono`](https://docs.rs/chrono/) - Date and time handling with timezone support

## License

This project is licensed under the terms specified in the project's license file.

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -am 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Changelog

### Version 0.1.0
- Initial implementation of four password algorithms
- KeeLoq encryption support
- UTC+8 timezone handling
- Comprehensive test suite
- Interactive CLI application
- Unified generator API
