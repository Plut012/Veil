import { a7 as slot, l as bind_props } from "../../chunks/renderer.js";
function _layout($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    function switchTheme(themeId) {
    }
    $$renderer2.push(`<!--[-->`);
    slot($$renderer2, $$props, "default", {});
    $$renderer2.push(`<!--]-->`);
    bind_props($$props, { switchTheme });
  });
}
export {
  _layout as default
};
