const core = globalThis.Deno.core;

class KV {
    static async get(key) {
        return await core.ops.op_kv_get(key);
    }

    static async put(key, value) {
        return await core.ops.op_kv_put(key, value);
    }

    static async delete(key) {
        return await core.ops.op_kv_delete(key);
    }
}

globalThis.KV = KV;
