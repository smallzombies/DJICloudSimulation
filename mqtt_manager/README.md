# MQTT Manager

一个使用 Rust 开发的 MQTT 配置管理和连接工具，提供 Web 界面和 RESTful API。

## 功能特性

### 前端
1. **首次配置**: Web 页面可配置 MQTT 的登录地址、端口、用户名和密码（用户名和密码不可为空）
2. **持久化存储**: 后续登录从后端获取上一次的登录信息
3. **登录反馈**: 登录失败时提示失败信息，登录成功后显示 MQTT 配置信息
4. **状态显示**: 实时显示 MQTT 连接状态

### 后端
1. **配置保存**: 将前端填写的 MQTT 信息保存到 SQLite 数据库
2. **MQTT 连接**: 使用保存的配置进行 MQTT  broker 连接
3. **自动重连**: 启动时尝试使用上次保存的配置自动连接

## 项目结构

```
mqtt_manager/
├── Cargo.toml              # 项目依赖配置
├── src/
│   ├── main.rs             # 应用入口和路由配置
│   ├── config/
│   │   └── mod.rs          # 应用配置管理
│   ├── db/
│   │   └── mod.rs          # 数据库操作
│   ├── handlers/
│   │   └── mod.rs          # HTTP 请求处理器
│   ├── models/
│   │   └── mod.rs          # 数据模型定义
│   └── services/
│       ├── mod.rs          # 服务模块导出
│       └── mqtt.rs         # MQTT 连接服务
└── static/
    └── index.html          # 前端 Web 页面
```

## 技术栈

- **Web 框架**: Axum (Tokio 生态)
- **数据库**: SQLite (通过 SQLx)
- **MQTT 客户端**: rumqttc
- **前端**: 原生 HTML/CSS/JavaScript
- **异步运行时**: Tokio

## API 接口

### GET /api/config
获取保存的 MQTT 配置

**响应示例**:
```json
{
  "success": true,
  "message": "Configuration retrieved successfully",
  "data": {
    "id": 1,
    "host": "broker.mqtt.com",
    "port": 1883,
    "username": "user",
    "created_at": "2024-01-01 00:00:00",
    "updated_at": "2024-01-01 00:00:00"
  }
}
```

### POST /api/config
保存 MQTT 配置并尝试连接

**请求体**:
```json
{
  "host": "broker.mqtt.com",
  "port": 1883,
  "username": "user",
  "password": "password"
}
```

### GET /api/status
获取当前 MQTT 连接状态

**响应示例**:
```json
{
  "connected": true,
  "config": {
    "id": 1,
    "host": "broker.mqtt.com",
    "port": 1883,
    "username": "user",
    "created_at": "2024-01-01 00:00:00",
    "updated_at": "2024-01-01 00:00:00"
  },
  "error": null
}
```

## 运行方法

### 环境要求
- Rust 1.70+ 
- Cargo

### 编译和运行

```bash
cd mqtt_manager
cargo build --release
cargo run
```

### 环境变量（可选）

```bash
export SERVER_HOST=0.0.0.0    # 监听地址，默认 0.0.0.0
export SERVER_PORT=3000       # 监听端口，默认 3000
export DATABASE_PATH=./mqtt_manager.db  # 数据库路径
```

### 访问应用

启动后，在浏览器中访问：`http://localhost:3000`

## 扩展开发

项目采用模块化设计，便于后续添加新功能：

1. **添加新的 API**: 在 `src/handlers/mod.rs` 中添加处理函数，在 `main.rs` 中注册路由
2. **添加新服务**: 在 `src/services/` 目录下创建新模块
3. **扩展数据模型**: 在 `src/models/mod.rs` 中添加新模型
4. **数据库迁移**: 在 `src/db/mod.rs` 中管理表结构

## 注意事项

1. 密码以明文存储在数据库中，生产环境应加密存储
2. 默认允许所有 CORS 请求，生产环境应限制来源
3. MQTT 连接失败时会持续重试，间隔 5 秒

## License

MIT
