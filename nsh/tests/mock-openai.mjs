// A tiny OpenAI-compatible server for translation tests.
// Run: node nsh/tests/mock-openai.mjs
import http from "node:http";

const port = Number(process.env.PORT || 8787);
const server = http.createServer((request, response) => {
  const chunks = [];
  request.on("data", (chunk) => chunks.push(chunk));
  request.on("end", () => {
    const url = new URL(request.url, "http://127.0.0.1:" + port);
    console.log(request.method + " " + url.pathname);
    if (request.method !== "POST" || !url.pathname.endsWith("/chat/completions")) {
      response.writeHead(404, { "Content-Type": "application/json" });
      response.end(JSON.stringify({ error: { message: "not found" } }));
      return;
    }
    if (url.pathname.startsWith("/v1/unauthorized")) {
      response.writeHead(401, { "Content-Type": "application/json" });
      response.end(JSON.stringify({ error: { message: "invalid api key" } }));
      return;
    }
    let body = {};
    try {
      body = JSON.parse(Buffer.concat(chunks).toString("utf8"));
    } catch {}
    const system = body?.messages?.find((message) => message.role === "system")?.content || "";
    const content = system.includes("English") ? "Hello from mock" : "你好，来自 mock";
    const answer = () => {
      response.writeHead(200, { "Content-Type": "application/json" });
      response.end(JSON.stringify({
        id: "mock-completion",
        choices: [{ index: 0, finish_reason: "stop", message: { role: "assistant", content } }],
      }));
    };
    if (url.pathname.startsWith("/v1/slow")) {
      setTimeout(answer, 3000);
    } else {
      answer();
    }
  });
});
server.listen(port, "127.0.0.1", () => console.log("mock OpenAI on http://127.0.0.1:" + port + "/v1"));