import com.sun.net.httpserver.HttpExchange;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentSkipListMap;

/**
 * 排行榜服务 — 提交分数、查询前 N 名
 *
 * 用 ConcurrentSkipListMap 按分数降序存储，O(log n) 插入/查询
 */
public class LeaderboardService {

    // playerId -> score
    private static final Map<String, Long> scores = new ConcurrentHashMap<>();
    // score(desc) -> playerId（TreeMap 降序）
    private static final ConcurrentSkipListMap<Long, String> ranking = new ConcurrentSkipListMap<>();

    static void handle(HttpExchange exchange) throws IOException {
        String path = exchange.getRequestURI().getPath();
        if (path.endsWith("/top")) {
            handleTop(exchange);
        } else if (path.endsWith("/submit")) {
            handleSubmit(exchange);
        } else {
            WastelandServer.sendJson(exchange, 404, "{\"error\":\"unknown endpoint\"}");
        }
    }

    private static void handleTop(HttpExchange exchange) throws IOException {
        String nStr = WastelandServer.getQueryParam(exchange, "n");
        int n = (nStr != null) ? Integer.parseInt(nStr) : 10;
        n = Math.max(1, Math.min(n, 100));

        List<Map.Entry<Long, String>> top = new ArrayList<>();
        for (Map.Entry<Long, String> e : ranking.descendingMap().entrySet()) {
            top.add(e);
            if (top.size() >= n) break;
        }

        StringBuilder sb = new StringBuilder("[");
        for (int i = 0; i < top.size(); i++) {
            if (i > 0) sb.append(",");
            Map.Entry<Long, String> e = top.get(i);
            sb.append(String.format(
                "{\"rank\":%d,\"playerId\":\"%s\",\"score\":%d}",
                i + 1, e.getValue(), e.getKey()
            ));
        }
        sb.append("]");
        String response = String.format("{\"top\":%s,\"total\":%d}", sb, scores.size());
        WastelandServer.sendJson(exchange, 200, response);
    }

    private static void handleSubmit(HttpExchange exchange) throws IOException {
        String playerId = WastelandServer.getQueryParam(exchange, "playerId");
        String scoreStr = WastelandServer.getQueryParam(exchange, "score");
        if (playerId == null || scoreStr == null) {
            WastelandServer.sendJson(exchange, 400, "{\"error\":\"missing playerId or score\"}");
            return;
        }
        long newScore = Long.parseLong(scoreStr);
        Long oldScore = scores.put(playerId, newScore);
        if (oldScore != null) {
            ranking.remove(oldScore, playerId);
        }
        ranking.put(newScore, playerId);

        int rank = ranking.descendingMap().headMap(newScore, true).size();
        String response = String.format(
            "{\"accepted\":true,\"playerId\":\"%s\",\"score\":%d,\"rank\":%d,\"requestId\":%d}",
            playerId, newScore, rank, WastelandServer.nextRequestId()
        );
        WastelandServer.sendJson(exchange, 200, response);
    }
}
