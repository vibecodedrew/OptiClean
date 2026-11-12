// A hidden WebView can suspend animation frames. Reveal after React's DOM commit,
// with a matching native/web background, rather than waiting for a hidden paint.
export function revealWhenMounted(root, show, Observer = globalThis.MutationObserver) {
  let shown = false;
  const reveal = () => {
    if (shown || !root.hasChildNodes()) return;
    shown = true;
    observer.disconnect();
    Promise.resolve().then(show).catch(() => {});
  };
  const observer = new Observer(reveal);
  observer.observe(root, { childList: true });
  reveal();
}
