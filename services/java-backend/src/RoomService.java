import com.sun.net.httpserver.HttpExchange;

import java.io.IOException;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * 房间服务 — 创建/列出/加入游戏房间
 */
public class RoomService {

    private static final Map<String, Room> rooms = new ConcurrentHashMap<>();
    private static final AtomicInteger roomIdGen = new AtomicInteger(1000);

    static class Room {
        String id;
        String hostId;
        int capacity;
        long createdAt;

        Room(String id, String hostId) {
            this.id = id;
            this.hostId = hostId;
            this.capacity = 8;
            this.createdAt = System.currentTimeMillis();
        }

        String toJson() {
            return String.format(
                "{\"id\":\"%s\",\"hostId\":\"%s\",\"capacity\":%d,\"createdAt\":%d}",
                id, hostId, capacity, createdAt
            );
        }
    }

    static void handle(HttpExchange exchange) throws IOException {
        String path = exchange.getRequestURI().getPath();
        if (path.endsWith("/create")) {
            handleCreate(exchange);
        } else if (path.endsWith("/list")) {
            handleList(exchange);
        } else {
            WastelandServer.sendJson(exchange, 404, "{\"error\":\"unknown endpoint\"}");
        }
    }

    private static void handleCreate(HttpExchange exchange) throws IOException {
        String hostId = WastelandServer.getQueryParam(exchange, "hostId");
        if (hostId == null || hostId.isEmpty()) {
            WastelandServer.sendJson(exchange, 400, "{\"error\":\"missing hostId\"}");
            return;
        }
        String roomId = "room-" + roomIdGen.incrementAndGet();
        Room room = new Room(roomId, hostId);
        rooms.put(roomId, room);

        String response = String.format(
            "{\"created\":true,%s,\"requestId\":%d}",
            room.toJson(), WastelandServer.nextRequestId()
        );
        WastelandServer.sendJson(exchange, 200, response);
    }

    private static void handleList(HttpExchange exchange) throws IOException {
        StringBuilder sb = new StringBuilder("[");
        boolean first = true;
        for (Room room : rooms.values()) {
            if (!first) sb.append(",");
            sb.append(room.toJson());
            first = false;
        }
        sb.append("]");
        String response = String.format(
            "{\"count\":%d,\"rooms\":%s}",
            rooms.size(), sb
        );
        WastelandServer.sendJson(exchange, 200, response);
    }
}
