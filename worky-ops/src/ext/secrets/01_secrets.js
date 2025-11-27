const core = globalThis.Deno.core;

class Secrets {
    static get(key) {
        return core.ops.op_secret_get(key);
    }
}

globalThis.Secrets = Secrets;
