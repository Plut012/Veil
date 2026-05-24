

export const index = 0;
let component_cache;
export const component = async () => component_cache ??= (await import('../entries/pages/_layout.svelte.js')).default;
export const universal = {
  "prerender": true,
  "ssr": false
};
export const universal_id = "src/routes/+layout.ts";
export const imports = ["_app/immutable/nodes/0.CmELwP8Q.js","_app/immutable/chunks/CM2QGpeL.js","_app/immutable/chunks/CCUsgSTd.js","_app/immutable/chunks/CgLg63iB.js"];
export const stylesheets = ["_app/immutable/assets/0.C3Befu08.css"];
export const fonts = [];
