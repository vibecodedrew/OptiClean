import { invoke } from "@tauri-apps/api/core";
import { revealWhenMounted } from "./startup.js";

document.addEventListener("contextmenu", event => event.preventDefault());
document.addEventListener("keydown", event => {
  if ((event.ctrlKey || event.metaKey) && ["p", "s"].includes(event.key.toLowerCase())) event.preventDefault();
});

async function start() {
  const root = document.getElementById("root");
  revealWhenMounted(root, () => {
    if (window.__TAURI_INTERNALS__) return invoke("app_ready");
  });
  try {
    await import("./out/App.js");
  } catch (error) {
    root.textContent = `OptiClean could not load: ${String(error)}`;
  }
}
start();
