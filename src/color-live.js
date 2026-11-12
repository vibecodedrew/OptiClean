// At most one driver call in flight and one latest value waiting. Timers exist
// only while editing; continuous dragging gets updates without an idle loop.
export function createColorQueue(apply, {
  delay = 75,
  schedule = setTimeout,
  unschedule = clearTimeout,
} = {}) {
  let timer = null;
  let pending = null;
  let running = null;
  let ready = false;
  let revision = 0;

  const receipt = (request, status, error = "") => ({
    status,
    current: request.revision === revision,
    saturation: request.value[0],
    shadow: request.value[1],
    error,
  });
  const stopTimer = () => {
    if (timer !== null) unschedule(timer);
    timer = null;
  };
  const flush = () => {
    stopTimer();
    ready = true;
    if (running || !pending) return;
    const request = pending;
    pending = null;
    ready = false;
    running = Promise.resolve()
      .then(() => apply(request.value))
      .then(
        () => request.resolve(receipt(request, "applied")),
        error => request.resolve(receipt(request, "failed", String(error))),
      )
      .finally(() => {
        running = null;
        if (ready && pending) flush();
      });
  };

  return {
    enqueue(value, immediate = false) {
      revision++;
      if (pending) pending.resolve(receipt(pending, "superseded"));
      const result = new Promise(resolve => {
        pending = { value: Array.from(value), revision, resolve };
      });
      if (immediate) {
        flush();
      } else if (timer === null && !ready) {
        // Do not keep pushing the deadline back during a continuous drag.
        timer = schedule(flush, delay);
      }
      return result;
    },
    async cancel() {
      revision++;
      stopTimer();
      ready = false;
      if (pending) pending.resolve(receipt(pending, "cancelled"));
      pending = null;
      // Recovery waits for an existing native call; no stale slider write may
      // arrive after restored settings.
      if (running) await running;
    },
  };
}
