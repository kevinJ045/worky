export default {
  fetch: (req) => {
    console.log(req);
    console.log(Deno.core.encode("Hello from JS"));
    console.log(new Response(Deno.core.encode("Hello from JS")));
    return new Response(Deno.core.encode("Hello from JS"));
  },
};
