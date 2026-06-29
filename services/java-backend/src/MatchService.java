import com.sun.net.httpserver.HttpExchange;

import java.io.IOException;
import java.util.concurrent.ConcurrentLinkedQueue;

/**
 * 匹配服务 — 玩家加入队列，凑够 4 人自动匹配成一队
 */
public class MatchService {

    private static final ConcurrentLinkedQueue<String> queue = new ConcurrentLinkedQueue<>();
    private static final int TEAM_SIZE = 4;

    static void handle(HttpExchange exchange) throws IOException {
        String path = exchange.getRequestURI().getPath();
        if (path.endsWith("/join")) {
            handleJoin(exchange);
        } else if (path.endsWith("/status")) {
            handleStatus(exchange);
        } else {
            WastelandServer.sendJson(exchange, 404, "{\"error\":\"unknown endpoint\"}");
        }
    }

    private static void handleJoin(HttpExchange exchange) throws IOException {
        String playerId = WastelandServer.getQueryParam(exchange, "playerId");
        if (playerId == null || playerId.isEmpty()) {
            WastelandServer.sendJson(exchange, 400, "{\"error\":\"missing playerId\"}");
            return;
        }
        queue.add(playerId);
        int position = queue.size();

        String response;
        if (queue.size() >= TEAM_SIZE) {
            StringBuilder team = new StringBuilder("[");
            for (int i = 0; i < TEAM_SIZE; i++) {
                if (i > 0) team.append(",");
                team.append("\"").append(queue.poll()).append("\"");
            }
            team.append("]");
            response = String.format(
                "{\"matched\":true,\"team\":%s,\"requestId\":%d}",
                team, WastelandServer.nextRequestId()
            );
        } else {
            response = String.format(
                "{\"matched\":false,\"position\":%d,\"need\":%d,\"requestId\":%d}",
                position, TEAM_SIZE - position, WastelandServer.nextRequestId()
            );
        }
        WastelandServer.sendJson(exchange, 200, response);
    }

    private static void handleStatus(HttpExchange exchange) throws IOException {
        String response = String.format(
            "{\"queueSize\":%d,\"teamSize\":%d,\"waiting\":%s}",
            queue.size(), TEAM_SIZE, queue.isEmpty() ? "[]" : "[\"" + String.join("\",\"", queue) + "\"]"
        );
        WastelandServer.sendJson(exchange, 200, response);
    }
}
