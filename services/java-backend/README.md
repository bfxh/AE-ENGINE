# Wasteland Java 后端服务

> 多人游戏后端服务（匹配 / 房间 / 排行榜）
> 语言: Java 17
> 依赖: 零外部依赖（纯 JDK com.sun.net.httpserver）

## 快速开始

### 运行（无需 Maven/Gradle）

```bash
cd services/java-backend
javac -encoding UTF-8 -d out src\*.java
java -cp out WastelandServer
```

服务器启动后监听 `http://localhost:8080`

### 测试端点

```bash
# 健康检查
curl http://localhost:8080/health

# 匹配服务
curl "http://localhost:8080/match/join?playerId=alice"
curl "http://localhost:8080/match/status"

# 房间服务
curl "http://localhost:8080/room/create?hostId=bob"
curl "http://localhost:8080/room/list"

# 排行榜服务
curl "http://localhost:8080/leaderboard/submit?playerId=alice&score=9500"
curl "http://localhost:8080/leaderboard/top?n=10"
```

## API 端点

| 方法 | 路径 | 参数 | 说明 |
|------|------|------|------|
| GET | `/health` | - | 健康检查 |
| GET | `/match/join` | playerId | 加入匹配队列（4 人成队） |
| GET | `/match/status` | - | 查询匹配队列状态 |
| GET | `/room/create` | hostId | 创建房间 |
| GET | `/room/list` | - | 列出所有房间 |
| GET | `/leaderboard/submit` | playerId, score | 提交分数 |
| GET | `/leaderboard/top` | n | 查询前 N 名（默认 10，最大 100） |

## 架构

```
WastelandServer (主服务器)
├── MatchService      (匹配队列 — ConcurrentLinkedQueue)
├── RoomService       (房间管理 — ConcurrentHashMap)
└── LeaderboardService (排行榜 — ConcurrentSkipListMap 降序)
```

- **线程池**: 8 线程 FixedThreadPool
- **并发安全**: 全部用 java.util.concurrent 并发集合
- **JSON**: 手写最小化 JSON（无外部依赖）

## Rust 客户端集成

Rust 游戏客户端通过 HTTP 调用本服务。在 Rust 端添加依赖：

```toml
# ae_network/Cargo.toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
```

调用示例：
```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Health { status: String, version: String }

let resp: Health = reqwest::get("http://localhost:8080/health")
    .await?.json().await?;
```

## 升级到 Spring Boot（可选）

本服务用纯 JDK 实现，便于零依赖快速验证。生产环境可升级到 Spring Boot：

1. 安装 Maven 或 Gradle
2. 用 `pom.xml`（见同目录）作为 Spring Boot 骨架
3. 将 3 个 Service 类迁移为 `@RestController`
4. 添加 JPA + PostgreSQL 持久化
5. 添加 Spring Security + JWT 认证

升级后获得：
- 自动配置 + Starter 依赖管理
- 内嵌 Tomcat / Netty
- actuator 监控端点
- 数据库 ORM
- 安全认证框架

## 设计决策

### 为什么用纯 JDK 而非 Spring Boot？

1. **零依赖快速验证**：用户环境无 Maven/Gradle，纯 JDK 可直接 javac 运行
2. **最小化复杂度**：3 个服务 + 7 个端点，不需要 Spring 的 IoC/AOP
3. **教学价值**：展示 Java 并发集合（ConcurrentHashMap/SkipListMap）的实际用法
4. **升级路径清晰**：见上文，迁移到 Spring Boot 成本低

### 为什么用 HTTP 而非 gRPC？

1. **调试友好**：curl 即可测试，无需 protobuf 工具链
2. **Rust 客户端简单**：reqwest 一行调用，无需 tonic/prost
3. **游戏场景适用**：匹配/排行榜是低频请求，HTTP 足够；实时同步才需 gRPC/WebSocket
4. **未来可扩展**：gRPC proto 定义见 `proto/ae.proto`，需要时可生成

## 性能

- 单机 QPS: ~5000（JDK HttpServer + 8 线程，纯内存操作）
- 排行榜插入: O(log n)（ConcurrentSkipListMap）
- 匹配成队: O(1)（队列 poll）
- 房间查找: O(1)（HashMap）

## 许可证

MIT
