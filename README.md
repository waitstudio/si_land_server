# si_land_server

矽澜（si_land）后端服务，基于 Rust + axum 构建。围绕「抖音主播开播订阅通知」场景，提供从用户认证、主播订阅、开播轮询到消息推送的完整服务端能力。

配套项目：[si_land_client](https://github.com/waitstudio/si_land_client)（Flutter 移动端） / [si_land_admin](https://github.com/waitstudio/si_land_admin)（管理后台）。

## 功能特性

### 用户与认证
- 手机号验证码登录，JWT 鉴权
- 验证码基于 Redis 存储，支持有效期控制、重发冷却、每小时发送上限与验证失败次数限制

### 主播订阅
- 热门主播列表：按人气降序展示，最多返回前 100 名
- 订阅 / 取消订阅主播（用户仅可订阅热门主播池中的主播）
- 想看意愿提交：暂未支持的主播可提交「想看」，按需扩充主播池
- 主播信息自动解析：通过抖音号解析 sec_uid、昵称、头像与开播状态

### 开播检测与轮询调度
- 开播状态检测：抽象 `LiveChecker` trait，当前实现基于抖音 enter 接口判断开播状态
- 单主播主动检测 + 订阅批量轮询两种模式
- 分布式轮询调度：Redis ZSet 任务队列 + inflight 在途队列，取出-执行-放回模式，进程崩溃后自动回收在途任务重新调度
- 轮询间隔按主播热度分级，热门主播检测更频繁

### 通知投递
- 在线用户：WebSocket 实时推送开播通知与未读数
- 离线用户：通过 Bark 走系统级推送
- Outbox 模式解耦：通知先落库再异步投递，投递失败自动重试，避免推送阻塞业务与消息丢失
- Kafka 消费通知事件，支持指数退避重试

### 管理后台接口
- 管理员登录（内置账号由环境变量配置，无数据库表）
- 手动添加主播：输入抖音号，自动完成解析 → 查重 → 入库 → 创建轮询任务
- 管理员与普通用户接口按 `/api/v1/admin/*` 与 `/api/v1/app/*` 前缀隔离，双层鉴权

### 基础设施
- 启动时自动执行幂等数据库迁移（CREATE TABLE IF NOT EXISTS），无需额外迁移工具
- 统一响应体 `{code, msg, data}` 与业务错误码
- 日志固定北京时间输出（毫秒精度）
- ULID 作为统一 ID 生成算法

## 技术栈

Rust · axum · tokio · sqlx · PostgreSQL · Redis · Kafka（rdkafka）

## 运行流程

环境要求：Rust（edition 2024）、Docker（含 docker compose 插件）。

```bash
# 1. 准备配置（按需修改数据库口令、JWT_SECRET、管理员账号等）
cp .env.example .env

# 2. 启动基础设施：PostgreSQL + Redis + Kafka
docker compose up -d

# 3. 启动服务（首次启动自动执行幂等数据库迁移，默认监听 0.0.0.0:8080）
cargo run

# 4. 验证
curl http://localhost:8080/health
```

说明：
- 管理员账号由 `.env` 中 `ADMIN_USERNAME` / `ADMIN_PASSWORD` 配置，供 [si_land_admin](https://github.com/waitstudio/si_land_admin) 登录使用。
- 开发联调可将 `MOCK_FIXED_CODE` 设为固定验证码（如 `1234`），跳过真实短信通道。
- 停止基础设施：`docker compose down`；清理数据卷：`docker compose down -v`。

## 免责声明

1. 本项目**仅供个人学习与技术研究用途**，严禁用于任何商业用途或违法违规用途。
2. 本项目涉及对第三方平台（抖音）数据的访问与解析，相关实现仅用于技术研究演示。使用者应遵守抖音平台的相关服务条款与 robots 协议，不得对平台服务造成干扰或滥用其接口。
3. 本项目不提供任何明示或默示的保证，不对数据的准确性、完整性与时效性作任何承诺。使用者因使用本项目产生的任何直接或间接损失，作者不承担任何责任。
4. 开播状态、主播信息等数据版权归原平台及主播所有。若相关权利人认为本项目侵犯了其合法权益，请通过 Issue 联系，核实后将及时处理。
5. 使用本项目产生的任何行为与后果，均由使用者本人承担。请在使用前了解并遵守您所在地区及目标平台适用的法律法规。
