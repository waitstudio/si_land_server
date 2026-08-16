# si_land_server

硅基星球（si_land）项目后端服务，基于 Rust + [axum](https://github.com/tokio-rs/axum) 构建。

> 当前为 **Mock 阶段**：接口契约与分层架构已就绪，外部依赖（短信通道、验证码存储）用内存实现，未接真实 DB / 短信通道。

## 技术栈

- 语言：Rust（edition 2024）
- Web 框架：axum 0.8
- 异步运行时：tokio
- 中间件：tower / tower-http（CORS、Trace）
- 日志：tracing + tracing-subscriber
- 配置：dotenv（`.env`）
- 基础设施：PostgreSQL 16 + Redis 7（由 docker-compose 管理）

## 架构概览

采用 **分层 + trait 抽象** 架构，关注点分离，便于扩展与替换：

```
HTTP 请求
   │
   ▼
routes.rs            路由聚合（含 /health）
   │
   ▼
api/v1/<模块>        表现层：handler（HTTP 适配）+ dto（入参出参）+ service（业务编排）
   │  handler 只做参数解析与响应组装，不写业务逻辑
   │  service 调用 domain 与 infra trait 编排业务
   ▼
domain/              领域层：纯业务模型（User、SmsCode），不依赖 HTTP/IO
   │
   ▼
services/            基础设施层：trait 定义 + mock 实现
   │  SmsProvider  —— 短信通道（MockSmsProvider 仅日志）
   │  CodeStore    —— 验证码与限流存储（Redis）
   ▼
state.rs             AppState：依赖注入，持有 trait 对象（Arc<dyn ...>）
config.rs            配置集中管理
error.rs             统一错误 + IntoResponse
response.rs          统一响应体 + 业务码
```

### 扩展指引

- **新增功能模块**：在 `src/api/v1/` 下新建目录（`mod.rs` + `handler.rs` + `dto.rs` + `service.rs`），在 `api/v1/mod.rs` 注册路由即可。
- **替换短信通道为阿里云/腾讯云**：实现 `SmsProvider` trait，在 `state::build_state` 替换注入即可，业务代码不动。
- **替换验证码存储为 Redis**：实现 `CodeStore` trait，在 `state::build_state` 替换注入即可。
- **接入数据库**：在 `services/` 下新增 `user_repository` 等 trait + 实现，service 层调用。

## 目录结构

```
si_land_server/
├── src/
│   ├── main.rs              # 入口：加载配置、构建 state、启动服务
│   ├── lib.rs               # 库入口（便于集成测试）
│   ├── config.rs            # AppConfig：从环境变量读取
│   ├── error.rs             # AppError + IntoResponse
│   ├── response.rs          # ApiResponse<T> + BizCode
│   ├── state.rs             # AppState + build_state（依赖注入）
│   ├── routes.rs            # 路由聚合
│   ├── middleware.rs        # 请求日志等中间件
│   ├── api/
│   │   ├── mod.rs           # /api 嵌套
│   │   └── v1/
│   │       ├── mod.rs       # v1 路由聚合
│   │       ├── sms/         # 短信验证码模块
│   │       │   ├── mod.rs
│   │       │   ├── handler.rs
│   │       │   ├── dto.rs
│   │       │   └── service.rs
│   │       └── auth/        # 认证模块
│   │           ├── mod.rs
│   │           ├── handler.rs
│   │           ├── dto.rs
│   │           └── service.rs
│   ├── domain/              # 领域模型
│   │   ├── user.rs
│   │   └── sms.rs
│   ├── services/            # 基础设施 trait + mock 实现
│   │   ├── sms_provider.rs
│   │   └── code_store.rs
│   └── utils/
│       └── phone.rs         # 手机号校验
├── docker-compose.yml       # PostgreSQL + Redis
├── .env.example             # 环境变量示例
└── Cargo.toml
```

## 快速开始

### 1. 启动依赖（PostgreSQL + Redis）

```bash
cd si_land_server
cp .env.example .env       # 按需修改密码 / 端口
docker compose up -d       # 启动 postgres 与 redis
docker compose ps          # 查看状态
```

### 2. 运行服务

```bash
cargo run                 # 默认监听 0.0.0.0:8080
```

看到 `🚀 si_land_server listening on http://0.0.0.0:8080` 即启动成功。

### 3. 验证接口

```bash
# 健康检查
curl http://localhost:8080/health

# 发送验证码（mock：默认固定验证码 1234，可在日志看到）
curl -X POST http://localhost:8080/api/v1/sms/send \
  -H 'Content-Type: application/json' \
  -d '{"phone":"13800138000"}'

# 验证码登录
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"phone":"13800138000","code":"1234"}'
```

## API 接口

所有响应统一格式：

```json
{
  "code": 0,
  "msg": "success",
  "data": { ... }
}
```

`code = 0` 表示成功，其余为业务错误码。

### 健康检查

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/health` | 服务存活探针，返回版本号 |

### 短信验证码

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| POST | `/api/v1/sms/send` | 向指定手机号发送验证码 |

请求体：
```json
{ "phone": "13800138000" }
```

响应（mock）：
```json
{
  "code": 0,
  "msg": "success",
  "data": { "phone": "13800138000", "expire_in": 300 }
}
```

> Mock 行为：不真正发送短信。配置 `MOCK_FIXED_CODE=1234` 时验证码固定为 `1234`（日志可见）；留空则随机 6 位。

### 验证码登录

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| POST | `/api/v1/auth/login` | 校验验证码并登录，返回 Token |

请求体：
```json
{ "phone": "13800138000", "code": "1234" }
```

响应（mock）：
```json
{
  "code": 0,
  "msg": "success",
  "data": {
    "token": "mock-token-u_8000",
    "token_type": "Bearer",
    "expires_at": 1786000000,
    "user": {
      "user_id": "u_8000",
      "phone": "13800138000",
      "nickname": "硅基星球用户",
      "avatar": ""
    }
  }
}
```

### 业务错误码

| code | 含义 |
| --- | --- |
| 0 | 成功 |
| 1001 | 参数不合法（手机号格式错误等） |
| 1002 | 验证码错误 |
| 1003 | 验证码已过期 |
| 1004 | 手机号未注册 |
| 1005 | 未授权 |
| 5000 | 服务内部错误 |

## 配置项

通过 `.env` 配置，详见 `.env.example`：

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `SERVER_HOST` | `0.0.0.0` | 监听地址 |
| `SERVER_PORT` | `8080` | 监听端口 |
| `RUST_LOG` | `si_land_server=debug,tower_http=debug` | 日志级别 |
| `DATABASE_URL` | `postgres://siland:siland123@localhost:5432/siland` | PostgreSQL 连接串 |
| `REDIS_URL` | `redis://:siland123@localhost:6379/0` | Redis 连接串 |
| `JWT_SECRET` | `please-change-me-in-production` | JWT 签名密钥 |
| `JWT_EXPIRES_HOURS` | `168` | Token 有效期（小时） |
| `SMS_CODE_EXPIRE_IN` | `300` | 验证码有效期（秒） |
| `SMS_CODE_RESEND_COOLDOWN` | `60` | 重发冷却（秒） |
| `MOCK_FIXED_CODE` | `1234` | 固定验证码（留空则随机 6 位） |

## 常用命令

```bash
docker compose up -d          # 启动 postgres + redis
docker compose down           # 停止
docker compose down -v        # 停止并删除数据卷
cargo build --release         # 构建生产产物
cargo run                     # 开发运行
cargo fmt                     # 格式化
cargo clippy                  # 静态检查
cargo test                    # 测试（含集成测试，基于 lib.rs）
```

## 后续路线（业务实现阶段）

- [ ] 接入 sqlx + PostgreSQL，建表与迁移
- [x] 使用 `RedisCodeStore` 持久化验证码并执行手机号限流
- [ ] 实现真实 `SmsProvider`（阿里云 / 腾讯云）
- [ ] JWT 签发与鉴权中间件
- [ ] 用户体系、注册流程
- [ ] 限流中间件（基于 Redis）
