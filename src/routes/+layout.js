// Client-rendered SPA: the Rust server ships the built bundle as static files
// and every read/write goes over JSON-RPC, so there is nothing to render ahead
// of time.
export const prerender = false;
export const ssr = false;
