export default {
  fetch: (req) => {
    console.log(req);
    return new Response({
      body: new ArrayBuffer("Hello from JS"),
    });
  },
};
