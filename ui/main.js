const { invoke } = window.__TAURI__.core;
const $ = (id) => document.getElementById(id);
const fields = ["auto_restore", "restore_on_undock", "restore_desktop_icons", "launch_at_login", "settle_delay_secs", "check_for_updates", "place_late_windows", "restore_stacking"];
let settings = null;

function timeAgo(secs) {
  if (!secs) return "";
  const diff = Math.max(0, Date.now() / 1000 - secs);
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)} min ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} h ago`;
  return new Date(secs * 1000).toLocaleDateString();
}

function showMessage(text) {
  $("event").textContent = text;
}

async function loadSettings() {
  settings = await invoke("get_settings");
  for (const f of fields) {
    const el = $(f);
    if (el.type === "checkbox") el.checked = !!settings[f];
    else el.value = String(settings[f]);
  }
  $("login-approval").hidden = !settings.login_needs_approval;
}

async function saveSettings() {
  const next = { ...settings };
  for (const f of fields) {
    const el = $(f);
    next[f] = el.type === "checkbox" ? el.checked : Number(el.value);
  }
  try {
    settings = await invoke("update_settings", { view: next });
  } catch (e) {
    showMessage(`Couldn't save settings: ${e}`);
    await loadSettings();
  }
}

function layoutRow(layout) {
  const row = document.createElement("div");
  row.className = "row";
  const text = document.createElement("span");
  const title = document.createElement("span");
  title.className = "title";
  title.textContent = layout.label;
  if (layout.current) {
    const tag = document.createElement("span");
    tag.className = "tag";
    tag.textContent = "CURRENT";
    title.append(tag);
  }
  const hint = document.createElement("span");
  hint.className = "hint";
  const icons = layout.icon_count ? `, ${layout.icon_count} icons` : "";
  hint.textContent = `${layout.window_count} windows${icons} · updated ${timeAgo(layout.saved_at)}${layout.pinned ? " · locked" : ""}`;
  text.append(title, hint);

  const buttons = document.createElement("span");
  buttons.className = "buttons";
  const lock = document.createElement("button");
  lock.className = "small";
  lock.textContent = layout.pinned ? "Unlock" : "Lock";
  lock.title = layout.pinned
    ? "Let Mooring keep this layout up to date automatically"
    : "Keep this layout as it is; only “Save Current Layout” changes it";
  lock.onclick = async () => {
    await invoke("set_pinned", { key: layout.key, pinned: !layout.pinned });
    refresh();
  };
  const forget = document.createElement("button");
  forget.className = "small";
  forget.textContent = "Forget";
  forget.title = "Delete this saved layout";
  forget.onclick = async () => {
    if (!confirm(`Forget the saved layout for “${layout.label}”?`)) return;
    await invoke("forget_layout", { key: layout.key });
    refresh();
  };
  buttons.append(lock, forget);
  row.append(text, buttons);
  return row;
}

function renderUpdate(u) {
  $("update-title").textContent = `Mooring ${u.current_version}`;
  const action = $("update-action");
  let hint;
  if (u.installing) hint = `Installing ${u.available_version}… Mooring will restart.`;
  else if (u.checking) hint = "Checking for updates…";
  else if (u.available_version) hint = `Version ${u.available_version} is available.`;
  else if (u.error) hint = u.error;
  else if (u.checked_at) hint = `Up to date · checked ${timeAgo(u.checked_at)}`;
  else hint = "Not checked yet.";
  $("update-hint").textContent = hint;
  action.textContent = u.available_version ? "Install and Restart" : "Check Now";
  action.disabled = u.checking || u.installing;

  $("update-banner").hidden = !u.available_version;
  if (u.available_version) {
    $("update-banner-title").textContent = `Mooring ${u.available_version} is available.`;
    $("update-banner-notes").textContent = u.installing
      ? "Downloading and installing… Mooring will restart."
      : `You have ${u.current_version}.`;
    $("update-banner-install").disabled = u.installing;
  }
}

async function installUpdate() {
  renderUpdate({ ...(await invoke("get_update")), installing: true });
  try {
    await invoke("install_update");
  } catch (e) {
    showMessage(String(e));
  }
  renderUpdate(await invoke("get_update"));
}

async function refresh() {
  const status = await invoke("get_status");
  $("current").textContent = status.current_label || "Looking at your displays…";
  $("permission").hidden = status.has_permission;
  $("icons-row").hidden = !status.supports_icons;
  $("login-title").textContent = status.platform === "macos" ? "Open at login" : "Run at startup";
  if (status.platform === "macos") {
    $("late-hint").textContent =
      "For 30 minutes after a restore, put windows of apps you open, and windows on other Spaces when you switch to them, in their saved spot.";
    $("stacking-hint").textContent =
      "Also put back which window was in front of which. Each app comes forward briefly, so the screen may flicker.";
  }
  if (status.last_event) {
    showMessage(`${status.last_event} · ${timeAgo(status.last_event_at)}`);
  }

  renderUpdate(await invoke("get_update"));

  const list = $("layouts");
  list.replaceChildren();
  const layouts = status.layouts.sort((a, b) => (b.current - a.current) || (b.saved_at - a.saved_at));
  if (!layouts.length) {
    const empty = document.createElement("div");
    empty.className = "empty";
    empty.textContent = "Nothing saved yet. Mooring saves your layout every few seconds.";
    list.append(empty);
  }
  for (const layout of layouts) list.append(layoutRow(layout));
}

async function run(button, command, done) {
  button.disabled = true;
  try {
    showMessage(done(await invoke(command)));
  } catch (e) {
    showMessage(String(e));
  } finally {
    button.disabled = false;
    refresh();
  }
}

$("restore").onclick = () =>
  run($("restore"), "restore_now", (r) =>
    r.moved ? `Restored ${r.moved} windows` : "Windows are already in place");
$("save").onclick = () => run($("save"), "save_now", (n) => `Saved ${n} windows`);
$("grant").onclick = () => invoke("request_permission");
$("diagnostics").onclick = () =>
  run($("diagnostics"), "export_diagnostics", (path) =>
    `Saved ${path.split(/[\\/]/).pop()} to Downloads`);
$("open-login-items").onclick = () => invoke("open_login_items");
$("update-banner-install").onclick = installUpdate;
$("update-action").onclick = async () => {
  if ($("update-action").textContent.startsWith("Install")) return installUpdate();
  const pending = invoke("check_update");
  renderUpdate({ ...(await invoke("get_update")), checking: true });
  renderUpdate(await pending);
};
for (const f of fields) $(f).addEventListener("change", saveSettings);

// Pick up changes made from the tray menu when the window comes back.
window.addEventListener("focus", () => { loadSettings(); refresh(); });

loadSettings();
refresh();
setInterval(refresh, 3000);
