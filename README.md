# QQ Bot

This is a Rust implementation of a QQ Bot using the official open API.

## Project Structure

The project has been refactored for better modularity and maintainability:

```
src/
├── lib.rs          # Library root, exports modules
├── main.rs         # Application entry point
├── config.rs       # Configuration management (Env vars)
├── models/         # Data models
│   ├── auth.rs     # Authentication models
│   ├── event.rs    # Event models (QQBotEvent, OpCode)
│   └── message.rs  # Message models (GroupMessage, PostMessageBody)
├── services/       # Core business logic
│   ├── client.rs   # QQ API Client
│   └── server.rs   # WebHook / WebSocket Server
└── utils/          # Utilities
    └── validation.rs # WebHook signature validation
```

## Configuration

The application uses `dotenv` to load configuration from a `.env` file or environment variables.

Supported variables:
- `QQ_APP_ID`: App ID (default: 102640909)
- `QQ_CLIENT_SECRET`: Client Secret
- `QQ_BASE_URL`: API Base URL (default: https://api.sgroup.qq.com)
- `QQ_AUTH_URL`: Auth URL (default: https://bots.qq.com/app/getAppAccessToken)

## Build and Run

1. Build:
   ```bash
   cargo build
   ```

2. Run:
   ```bash
   cargo run
   ```

3. Test:
   ```bash
   cargo test
   ```
