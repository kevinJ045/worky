# Worky: Workers-Like Serverless Runtime in Rust (deno_core)

This document outlines the design, architecture, and roadmap for building a Workers-like serverless runtime in Rust using `deno_core`. It includes all major components, goals, and a detailed TODO list.

---

## Overview

A serverless runtime allows developers to write small JS/TS functions that respond to events (HTTP requests, timers, messages) in an isolated environment. This runtime will be **self-hosted, portable, embeddable**, and provide **Web-like APIs** similar to Cloudflare Workers but fully open-source.

### Key Features

* JS/TS function execution inside Rust runtime (deno_core)
* Async support
* Host ops (`fetch`, `KV`, logging, etc.)
* Hot-reload and local dev
* Isolate pooling and resource limits
* ES module / TypeScript support
* Optional WASM plugin support

---

## Architecture

```
HTTP(S) ──> Router ─┬─> Worker Isolate 1 (JS/TS)
                   │    - module loader
                   │    - async execution
                   │    - host ops (fetch, KV, crypto)
                   ├─> Worker Isolate 2 (JS/TS or WASM)
                   └─> Worker Isolate N
```

### Components

1. **Host server (Rust + axum/hyper)**

   * Accept HTTP requests
   * Route requests to appropriate module
   * Metrics, logging

2. **Isolate manager**

   * Pools `deno_core::JsRuntime` isolates
   * Manages per-request execution
   * Enforces memory/time limits

3. **JS Engine (deno_core)**

   * Embeds V8 engine
   * Handles async JS execution

4. **Module loader**

   * Loads ES modules
   * Supports TypeScript via transpile (esbuild/swc)
   * Handles caching and hot-reload

5. **Host Ops**

   * `fetch(url)`: async HTTP
   * `console.log(...)`: logging
   * `KV.get/set`: simple key-value store (sled/SQLite)
   * Optional WebSocket, timers, crypto

6. **Storage / KV**

   * sled/SQLite for persistent storage
   * Durable Objects / per-resource actor pattern

7. **Scheduler / Queue**

   * Timer events
   * Background jobs / cron

8. **CLI / Dev workflow**

   * `dev`: local server with hot-reload
   * `build`: transpile/bundle TS
   * `publish`: package module

9. **Deployment**

   * Self-hosted binary / container
   * Optional edge deployment
   * Registry for module sharing

---

## Developer API (JS/TS)

```ts
export default {
  fetch: function(req: Request){
    const res = await fetch("https://api.example.com/data");
    const json = await res.json();
    await KV.put("last", JSON.stringify({ ts: Date.now() }));
    return new Response(
      JSON.stringify({ ok: true, data: json }),
      {
        headers: {
          "Content-Type": "application/json"
        }
      }
    );
  }
}
```

### Exposed Host APIs

* `fetch(url: string) => Promise<Response>`
* `KV.get(key: string)`
* `KV.put(key: string, value: any)`
* `console.log(...args)`
* `crypto` (optional)
* `WebSocket` (optional)
* Timers: `setTimeout`, `setInterval`

---

## Security & Isolation

* Memory and execution time limits
* Network egress restrictions
* Filesystem isolation / VFS ops
* Secrets management (never inject into global scope)
* Per-tenant isolation
* Module integrity verification (signatures)

---

## Roadmap & TODO List

### Milestone 0: Research & Prototype

* [x] Decide JS engine: `deno_core` (V8) ✅
* [x] Prototype embedding small runtime, executing `console.log('hi')`
* [ ] Benchmark startup and memory usage

### Milestone 1: Basic HTTP Handler

* [x] Setup axum/hyper HTTP server
* [x] Load JS/TS files
* [x] Execute JS module default export
* [x] Return JSON response
* [x] Add simple logging

### Milestone 2: Host Ops

* [x] Implement `fetch` host op
* [x] Implement `console.log` op
* [ ] Implement basic KV op with sled
* [ ] Make host ops async compatible

### Milestone 3: TypeScript Support

* [ ] Integrate esbuild or swc for transpiling TS → JS
* [ ] Support source maps for better dev experience
* [ ] Hot-reload TS modules

### Milestone 4: Async & ES Module Execution

* [ ] Properly load ES modules using `deno_core` module loader
* [ ] Support async default exports
* [ ] Handle errors and exceptions properly

### Milestone 5: Isolate Pooling & Resource Limits

* [ ] Create isolate pool
* [ ] Enforce per-isolate memory limit
* [ ] Enforce per-request timeout
* [ ] Support concurrent requests

### Milestone 6: CLI & Dev Workflow

* [ ] `dev` command: run local server with hot-reload
* [ ] `build` command: bundle TS modules
* [ ] `publish` command: package module for deployment

### Milestone 7: Persistence & Durable Objects

* [ ] Implement per-resource KV namespaces
* [ ] Optional actor pattern for durable objects
* [ ] Evaluate SQLite/sled/RocksDB backend

### Milestone 8: Observability & Metrics

* [ ] Add tracing for request/response
* [ ] Metrics: latency, active isolates, KV ops
* [ ] Optional Prometheus integration

### Milestone 9: WASM & Plugin Support (Optional)

* [ ] Support WASM modules as plugins
* [ ] Integrate with isolate pool
* [ ] Provide host ops to WASM modules

### Milestone 10: Deployment & Registry

* [ ] Containerize runtime
* [ ] Define bundle manifest.json for modules
* [ ] Registry for module sharing and versioning
* [ ] CLI commands for uploading/downloading modules

---

## Notes

* Start small: Milestone 1–2 is enough for a working local dev runtime.
* Use QuickJS or V8 in `deno_core` depending on performance needs.
* Hot reload is critical for TS/JS development iteration.
* Host ops are the bridge between Rust and JS; async ops allow real-world usage (HTTP, KV, timers).
* Security must be considered from day 1: isolation, memory limits, secrets, egress policy.

---

## References

* [deno_core docs](https://docs.rs/deno_core)
* [Cloudflare Workers API](https://developers.cloudflare.com/workers/)
* [Bun runtime](https://bun.sh/)
* [esbuild](https://esbuild.github.io/)
* [swc](https://swc.rs/)
