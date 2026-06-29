import com.sun.net.httpserver.HttpServer;
import com.sun.net.httpserver.HttpExchange;

import java.io.IOException;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * Wasteland 多人游戏后端服务器
 *
 * 纯 JDK 实现（无需 Maven/Gradle），用 com.sun.net.httpserver.HttpServer。
 * 提供匹配/房间/排行榜三个核心服务。
 *
 * 运行:
 *   cd services/java-backend
 *   javac -d out src\*.java
 *   java -cp out WastelandServer
 *
 * 测试:
 *   curl http://localhost:8080/health
 *   curl http://localhost:8080/match/join?playerId=alice
 *   curl http://localhost:8080/room/create?hostId=bob
 *   curl http://localhost:8080/leaderboard/top?n=10
 */
public class WastelandServer {

    private static final int PORT = 8080;
    private static final AtomicInteger requestId = new AtomicInteger(0);

    public static void main(String[] args) throws IOException {
        HttpServer server = HttpServer.create(new InetSocketAddress(PORT), 0);

        server.createContext("/health", WastelandServer::handleHealth);
        server.createContext("/match/", MatchService::handle);
        server.createContext("/room/", RoomService::handle);
        server.createContext("/leaderboard/", LeaderboardService::handle);

        server.setExecutor(Executors.newFixedThreadPool(8));
        server.start();

        System.out.println("=== Wasteland 后端服务启动 ===");
        System.out.println("端口: " + PORT);
        System.out.println("线程池: 8 线程");
        System.out.println("端点:");
        System.out.println("  GET /health              - 健康检查");
        System.out.println("  GET /match/join?playerId - 加入匹配队列");
        System.out.println("  GET /match/status        - 匹配队列状态");
        System.out.println("  GET /room/create?hostId  - 创建房间");
        System.out.println("  GET /room/list           - 房间列表");
        System.out.println("  GET /leaderboard/top?n   - 前 N 名");
        System.out.println("  GET /leaderboard/submit  - 提交分数");
        System.out.println("等待 Rust 游戏客户端连接...");
    }

    static void handleHealth(HttpExchange exchange) throws IOException {
        String response = "{\"status\":\"ok\",\"service\":\"wasteland-backend\",\"version\":\"0.1.0\"}";
        sendJson(exchange, 200, response);
    }

    static void sendJson(HttpExchange exchange, int status, String body) throws IOException {
        byte[] bytes = body.getBytes(StandardCharsets.UTF_8);
        exchange.getResponseHeaders().set("Content-Type", "application/json; charset=utf-8");
        exchange.getResponseHeaders().set("Access-Control-Allow-Origin", "*");
        exchange.sendResponseHeaders(status, bytes.length);
        try (OutputStream os = exchange.getResponseBody()) {
            os.write(bytes);
        }
    }

    static String getQueryParam(HttpExchange exchange, String key) {
        String query = exchange.getRequestURI().getQuery();
        if (query == null) return null;
        for (String pair : query.split("&")) {
            String[] kv = pair.split("=", 2);
            if (kv[0].equals(key)) return kv.length > 1 ? kv[1] : "";
        }
        return null;
    }

    static int nextRequestId() {
        return requestId.incrementAndGet();
    }
}
