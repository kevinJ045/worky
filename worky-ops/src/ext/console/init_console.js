import * as console from "ext:deno_console/01_console.js";
globalThis.console = new console.Console((output, code) =>
  Deno.core.ops[code ? "op_log_stdout" : "op_log_stderr"](
    Deno.core.encode(output),
  ),
);
