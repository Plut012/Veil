import { a9 as ssr_context, A as fallback, k as attr_class, j as attr, z as escape_html, l as bind_props, aa as store_get, y as ensure_array_like, ae as unsubscribe_stores } from "../../chunks/renderer.js";
import "clsx";
import { d as derived, w as writable } from "../../chunks/index.js";
function onDestroy(fn) {
  /** @type {SSRContext} */
  ssr_context.r.on_destroy(fn);
}
const connectionState = writable("disconnected");
const isConnected = derived(
  connectionState,
  ($state) => $state === "connected"
);
const contacts = writable([]);
function setContacts(list) {
  contacts.set(list);
}
function addContact(contact) {
  contacts.update((list) => {
    const exists = list.find((c) => c.contact_id === contact.contact_id);
    if (exists) {
      return list.map((c) => c.contact_id === contact.contact_id ? contact : c);
    }
    return [...list, contact];
  });
}
const selectedContactId = writable(null);
const showPairingModal = writable(false);
const showSettings = writable(false);
const pairingRole = writable(null);
const messages = writable(/* @__PURE__ */ new Map());
function addMessage(message) {
  messages.update((map) => {
    const key = message.contact_id;
    const existing = map.get(key) ?? [];
    const updated = new Map(map);
    updated.set(key, [...existing, message]);
    return updated;
  });
}
const currentMessages = derived(
  [messages, selectedContactId],
  ([$messages, $selectedId]) => {
    if (!$selectedId) return [];
    return $messages.get($selectedId) ?? [];
  }
);
const WS_URL = "ws://127.0.0.1:8900/ws";
const RECONNECT_DELAYS = [1e3, 2e3, 4e3, 8e3, 16e3, 3e4];
class VeilSocket {
  ws = null;
  handlers = /* @__PURE__ */ new Map();
  reconnectAttempt = 0;
  reconnectTimer = null;
  intentionalClose = false;
  qrImageStore = { set: (_v) => {
  } };
  // Store reference for QR image, set after construction
  setQrStore(store) {
    this.qrImageStore = store;
  }
  connect() {
    this.intentionalClose = false;
    this._connect();
  }
  _connect() {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) return;
    connectionState.set("connecting");
    try {
      this.ws = new WebSocket(WS_URL);
    } catch {
      this._scheduleReconnect();
      return;
    }
    this.ws.onopen = () => {
      this.reconnectAttempt = 0;
      connectionState.set("connected");
    };
    this.ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        this._dispatch(msg);
      } catch {
      }
    };
    this.ws.onclose = () => {
      connectionState.set("disconnected");
      if (!this.intentionalClose) {
        this._scheduleReconnect();
      }
    };
    this.ws.onerror = () => {
      connectionState.set("error");
    };
  }
  _scheduleReconnect() {
    if (this.intentionalClose) return;
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    const delay = RECONNECT_DELAYS[Math.min(this.reconnectAttempt, RECONNECT_DELAYS.length - 1)];
    this.reconnectAttempt++;
    connectionState.set("connecting");
    this.reconnectTimer = setTimeout(() => {
      this._connect();
    }, delay);
  }
  disconnect() {
    this.intentionalClose = true;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    connectionState.set("disconnected");
  }
  send(message) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }
  // Typed send helpers
  sendMessage(contactId, text) {
    this.send({ type: "send_message", contact_id: contactId, text });
  }
  requestContacts() {
    this.send({ type: "list_contacts" });
  }
  initiatePairing() {
    this.send({ type: "initiate_pairing" });
  }
  completePairing(qrData) {
    this.send({ type: "complete_pairing", qr_data: qrData });
  }
  updateEnvelope(template) {
    this.send({ type: "update_envelope", template });
  }
  setTheme(themeId) {
    this.send({ type: "set_theme", theme_id: themeId });
  }
  // Event registration
  on(type, handler) {
    const existing = this.handlers.get(type) ?? [];
    this.handlers.set(type, [...existing, handler]);
  }
  off(type, handler) {
    const existing = this.handlers.get(type) ?? [];
    this.handlers.set(
      type,
      existing.filter((h) => h !== handler)
    );
  }
  _dispatch(msg) {
    switch (msg.type) {
      case "connected":
        connectionState.set("connected");
        break;
      case "contacts":
        setContacts(msg.contacts ?? []);
        break;
      case "message":
        addMessage({
          contact_id: msg.contact_id,
          text: msg.text,
          direction: msg.direction,
          timestamp: msg.timestamp ?? (/* @__PURE__ */ new Date()).toISOString()
        });
        break;
      case "pairing_qr":
        break;
      case "pairing_complete":
        addContact(msg.contact);
        showPairingModal.set(false);
        break;
      case "error":
        console.error("[veil]", msg.message);
        break;
    }
    const handlers = this.handlers.get(msg.type) ?? [];
    for (const h of handlers) {
      h(msg);
    }
  }
}
const veilSocket = new VeilSocket();
function ContactItem($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let contact = $$props["contact"];
    let selected = fallback($$props["selected"], false);
    let onClick = fallback($$props["onClick"], () => {
    });
    $$renderer2.push(`<button${attr_class("contact-item svelte-5u7vir", void 0, { "selected": selected })}${attr("aria-current", selected ? "true" : void 0)}><span${attr_class("indicator svelte-5u7vir", void 0, { "active": selected })}></span> <span class="name svelte-5u7vir">${escape_html(contact.display_name)}</span></button>`);
    bind_props($$props, { contact, selected, onClick });
  });
}
function ContactList($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    $$renderer2.push(`<div class="contact-list svelte-r7m1hc">`);
    if (store_get($$store_subs ??= {}, "$contacts", contacts).length === 0) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="empty svelte-r7m1hc">No contacts yet.</p>`);
    } else {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<!--[-->`);
      const each_array = ensure_array_like(store_get($$store_subs ??= {}, "$contacts", contacts));
      for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
        let contact = each_array[$$index];
        ContactItem($$renderer2, {
          contact,
          selected: store_get($$store_subs ??= {}, "$selectedContactId", selectedContactId) === contact.contact_id,
          onClick: () => selectedContactId.set(contact.contact_id)
        });
      }
      $$renderer2.push(`<!--]-->`);
    }
    $$renderer2.push(`<!--]--></div>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
function Sidebar($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    const stateLabel = {
      connected: "live",
      connecting: "...",
      disconnected: "off",
      error: "err"
    };
    $$renderer2.push(`<aside class="sidebar svelte-129hoe0"><header class="sidebar-header svelte-129hoe0"><span class="wordmark svelte-129hoe0">Veil</span> <span class="conn-state svelte-129hoe0"${attr("data-state", store_get($$store_subs ??= {}, "$connectionState", connectionState))}>${escape_html(stateLabel[store_get($$store_subs ??= {}, "$connectionState", connectionState)] ?? "?")}</span></header> `);
    ContactList($$renderer2);
    $$renderer2.push(`<!----> <footer class="sidebar-footer svelte-129hoe0"><button class="pair-btn svelte-129hoe0">+ pair</button> <button class="settings-btn svelte-129hoe0" aria-label="Settings"><svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3"><circle cx="7" cy="7" r="2.5"></circle><path d="M7 1v1.5M7 11.5V13M1 7h1.5M11.5 7H13M2.93 2.93l1.06 1.06M10.01 10.01l1.06 1.06M2.93 11.07l1.06-1.06M10.01 3.99l1.06-1.06"></path></svg></button></footer></aside>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
function Message($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let message = $$props["message"];
    function formatTime(ts) {
      try {
        const d = new Date(ts);
        return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
      } catch {
        return "";
      }
    }
    $$renderer2.push(`<div${attr_class("message svelte-1uqoiy7", void 0, {
      "outbound": message.direction === "out",
      "inbound": message.direction === "in"
    })}><p class="text svelte-1uqoiy7">${escape_html(message.text)}</p> <span class="time svelte-1uqoiy7">${escape_html(formatTime(message.timestamp))}</span></div>`);
    bind_props($$props, { message });
  });
}
function MessageList($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    $$renderer2.push(`<div class="message-list svelte-qha2j"><!--[-->`);
    const each_array = ensure_array_like(store_get($$store_subs ??= {}, "$currentMessages", currentMessages));
    for (let i = 0, $$length = each_array.length; i < $$length; i++) {
      let msg = each_array[i];
      Message($$renderer2, { message: msg });
    }
    $$renderer2.push(`<!--]--></div>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
function Compose($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let text = "";
    $$renderer2.push(`<div class="compose svelte-nu6sgc"><textarea class="input svelte-nu6sgc" placeholder="write..." rows="1"${attr("disabled", !store_get($$store_subs ??= {}, "$isConnected", isConnected) || !store_get($$store_subs ??= {}, "$selectedContactId", selectedContactId), true)}>`);
    const $$body = escape_html(text);
    if ($$body) {
      $$renderer2.push(`${$$body}`);
    }
    $$renderer2.push(`</textarea> <button class="send-btn svelte-nu6sgc"${attr("disabled", !text.trim() || !store_get($$store_subs ??= {}, "$isConnected", isConnected) || !store_get($$store_subs ??= {}, "$selectedContactId", selectedContactId), true)} aria-label="Send"><svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4"><path d="M1 7h12M8 2l5 5-5 5" stroke-linecap="round" stroke-linejoin="round"></path></svg></button></div>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
function ChatView($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let selectedContact;
    selectedContact = store_get($$store_subs ??= {}, "$contacts", contacts).find((c) => c.contact_id === store_get($$store_subs ??= {}, "$selectedContactId", selectedContactId)) ?? null;
    $$renderer2.push(`<div class="chat-view svelte-may7r9">`);
    if (selectedContact) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<header class="chat-header svelte-may7r9"><span class="contact-name svelte-may7r9">${escape_html(selectedContact.display_name)}</span></header> `);
      MessageList($$renderer2);
      $$renderer2.push(`<!----> `);
      Compose($$renderer2);
      $$renderer2.push(`<!---->`);
    } else {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<div class="no-contact svelte-may7r9"><p>Select a contact to begin.</p></div>`);
    }
    $$renderer2.push(`<!--]--></div>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
function PairingModal($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let qrImage = null;
    let scanning = false;
    let scanError = null;
    let pairingStatus = "idle";
    function close() {
      stopScanner();
      qrImage = null;
      pairingStatus = "idle";
      scanError = null;
      pairingRole.set(null);
      showPairingModal.set(false);
    }
    async function stopScanner() {
    }
    function onQrMessage(data) {
      const msg = data;
      qrImage = `data:image/png;base64,${msg.qr_image}`;
      pairingStatus = "waiting";
    }
    function onPairingComplete() {
      pairingStatus = "done";
      setTimeout(close, 1200);
    }
    onDestroy(() => {
      veilSocket.off("pairing_qr", onQrMessage);
      veilSocket.off("pairing_complete", onPairingComplete);
      stopScanner();
    });
    $$renderer2.push(`<div class="backdrop svelte-1i7yvjj"><div class="modal svelte-1i7yvjj" role="dialog" aria-modal="true" aria-label="Pair a contact"><button class="close-btn svelte-1i7yvjj" aria-label="Close"><svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" class="svelte-1i7yvjj"><path d="M1 1l10 10M11 1L1 11" stroke-linecap="round" class="svelte-1i7yvjj"></path></svg></button> `);
    if (!store_get($$store_subs ??= {}, "$pairingRole", pairingRole)) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="role-select svelte-1i7yvjj"><p class="modal-title svelte-1i7yvjj">New contact</p> <div class="role-buttons svelte-1i7yvjj"><button class="role-btn svelte-1i7yvjj"><span class="role-icon svelte-1i7yvjj"><svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.3" class="svelte-1i7yvjj"><rect x="3" y="3" width="14" height="14" rx="1" class="svelte-1i7yvjj"></rect><rect x="6" y="6" width="3" height="3" class="svelte-1i7yvjj"></rect><rect x="11" y="6" width="3" height="3" class="svelte-1i7yvjj"></rect><rect x="6" y="11" width="3" height="3" class="svelte-1i7yvjj"></rect><rect x="11" y="11" width="2" height="2" class="svelte-1i7yvjj"></rect></svg></span> <span class="role-label svelte-1i7yvjj">Show QR</span></button> <button class="role-btn svelte-1i7yvjj"><span class="role-icon svelte-1i7yvjj"><svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.3" class="svelte-1i7yvjj"><path d="M2 10a8 8 0 1 0 16 0 8 8 0 0 0-16 0z" class="svelte-1i7yvjj"></path><circle cx="10" cy="10" r="3" class="svelte-1i7yvjj"></circle></svg></span> <span class="role-label svelte-1i7yvjj">Scan QR</span></button></div></div>`);
    } else if (store_get($$store_subs ??= {}, "$pairingRole", pairingRole) === "initiate") {
      $$renderer2.push("<!--[1-->");
      $$renderer2.push(`<div class="initiate-view svelte-1i7yvjj"><p class="modal-title svelte-1i7yvjj">Show this QR to your contact</p> `);
      if (qrImage) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<div class="qr-wrapper svelte-1i7yvjj"><img${attr("src", qrImage)} alt="Pairing QR code" class="qr-image svelte-1i7yvjj"/></div> <p class="qr-hint svelte-1i7yvjj">The key is ephemeral. Dismiss after scanning.</p>`);
      } else if (pairingStatus === "done") {
        $$renderer2.push("<!--[1-->");
        $$renderer2.push(`<p class="status-msg success svelte-1i7yvjj">Paired.</p>`);
      } else {
        $$renderer2.push("<!--[-1-->");
        $$renderer2.push(`<div class="qr-placeholder svelte-1i7yvjj"><span class="loading-dots svelte-1i7yvjj">...</span></div>`);
      }
      $$renderer2.push(`<!--]--></div>`);
    } else if (store_get($$store_subs ??= {}, "$pairingRole", pairingRole) === "join") {
      $$renderer2.push("<!--[2-->");
      $$renderer2.push(`<div class="join-view svelte-1i7yvjj">`);
      if (pairingStatus === "idle" || scanning) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<p class="modal-title svelte-1i7yvjj">Scan your contact's QR</p> <div class="scanner-container svelte-1i7yvjj"><div id="qr-scanner-container" class="svelte-1i7yvjj"></div> `);
        if (scanError) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<p class="scan-error svelte-1i7yvjj">${escape_html(scanError)}</p>`);
        } else {
          $$renderer2.push("<!--[-1-->");
        }
        $$renderer2.push(`<!--]--></div>`);
      } else if (pairingStatus === "waiting") {
        $$renderer2.push("<!--[1-->");
        $$renderer2.push(`<p class="modal-title svelte-1i7yvjj">Completing pairing...</p> <div class="qr-placeholder svelte-1i7yvjj"><span class="loading-dots svelte-1i7yvjj">...</span></div>`);
      } else if (pairingStatus === "done") {
        $$renderer2.push("<!--[2-->");
        $$renderer2.push(`<p class="status-msg success svelte-1i7yvjj">Paired.</p>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></div></div>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
function EnvelopeConfig($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let template = "{ciphertext}";
    $$renderer2.push(`<div class="envelope-config svelte-1kzlodm"><label class="field-label svelte-1kzlodm" for="envelope-template">Envelope template</label> <textarea id="envelope-template" class="template-input svelte-1kzlodm" rows="2" spellcheck="false">`);
    const $$body = escape_html(template);
    if ($$body) {
      $$renderer2.push(`${$$body}`);
    }
    $$renderer2.push(`</textarea> <p class="hint svelte-1kzlodm">Use <code class="svelte-1kzlodm">{ciphertext}</code> as the placeholder. Example: <code class="svelte-1kzlodm">~~ {ciphertext} ~~</code></p> <button class="save-btn svelte-1kzlodm">${escape_html("apply")}</button></div>`);
  });
}
function SettingsPanel($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    const themes = [{ id: "art-nouveau", label: "Art Nouveau" }];
    let selectedTheme = "art-nouveau";
    $$renderer2.push(`<div class="settings-panel svelte-d580bl" role="complementary" aria-label="Settings"><header class="settings-header svelte-d580bl"><span class="settings-title svelte-d580bl">Settings</span> <button class="close-btn svelte-d580bl" aria-label="Close settings"><svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M1 1l10 10M11 1L1 11" stroke-linecap="round"></path></svg></button></header> <div class="settings-body svelte-d580bl"><section class="settings-section svelte-d580bl"><h3 class="section-title svelte-d580bl">Envelope</h3> `);
    EnvelopeConfig($$renderer2);
    $$renderer2.push(`<!----></section> <section class="settings-section svelte-d580bl"><h3 class="section-title svelte-d580bl">Theme</h3> <div class="theme-list svelte-d580bl"><!--[-->`);
    const each_array = ensure_array_like(themes);
    for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
      let theme = each_array[$$index];
      $$renderer2.push(`<button${attr_class("theme-option svelte-d580bl", void 0, { "active": selectedTheme === theme.id })}><span class="theme-swatch svelte-d580bl"${attr("data-theme", theme.id)}></span> <span class="theme-label svelte-d580bl">${escape_html(theme.label)}</span></button>`);
    }
    $$renderer2.push(`<!--]--></div></section></div></div>`);
  });
}
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    onDestroy(() => {
      veilSocket.disconnect();
    });
    $$renderer2.push(`<div class="app-shell svelte-1uha8ag">`);
    Sidebar($$renderer2);
    $$renderer2.push(`<!----> <main class="main-area svelte-1uha8ag">`);
    ChatView($$renderer2);
    $$renderer2.push(`<!----></main> `);
    if (store_get($$store_subs ??= {}, "$showSettings", showSettings)) {
      $$renderer2.push("<!--[0-->");
      SettingsPanel($$renderer2);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> `);
    if (store_get($$store_subs ??= {}, "$showPairingModal", showPairingModal)) {
      $$renderer2.push("<!--[0-->");
      PairingModal($$renderer2);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></div>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
export {
  _page as default
};
