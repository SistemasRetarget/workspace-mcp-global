# ---- Build stage: compilar el servidor Rust ----
FROM rust:1.85-alpine AS builder

RUN apk add --no-cache musl-dev pkgconfig

WORKDIR /build
COPY servers/quality-gate/Cargo.toml servers/quality-gate/Cargo.lock ./
COPY servers/quality-gate/src ./src

RUN cargo build --release

# ---- Runtime: Node.js wrapper HTTP → stdio MCP ----
FROM node:18-alpine AS runner

WORKDIR /app

# Copiar binario Rust compilado
COPY --from=builder /build/target/release/quality-gate-server ./servers/quality-gate/target/release/quality-gate-server
RUN chmod +x ./servers/quality-gate/target/release/quality-gate-server

# Copiar workspace MCP (contratos, lecciones, tools, config)
COPY contracts ./contracts
COPY lessons ./lessons
COPY tools ./tools
COPY reports ./reports
COPY evidence ./evidence
COPY mcp-config.json ./
COPY mcp-subagents-config.json ./
COPY INTEGRATION_GUIDE.md ./
COPY MCP_EVOLUTION_PLAN.md ./
COPY SUBAGENT_ARCHITECTURE.md ./

# HTTP wrapper que expone el MCP como REST API para Cloud Run
COPY http-wrapper.mjs ./

ENV NODE_ENV=production
ENV PORT=8080
ENV WORKSPACE_MCP_HOME=/app
ENV RETARGET_WORKSPACE=/app/contracts

EXPOSE 8080

CMD ["node", "http-wrapper.mjs"]
