let worker;
let sequence = 0;
const pending = new Map();

function load() {
  if (worker) return worker;
  const instance = new Worker(new URL("./compiler.js", import.meta.url), { type: "module" });
  instance.onmessage = ({ data }) => {
    const task = pending.get(data.id);
    if (!task) return;
    pending.delete(data.id);
    if (data.error) task.reject(new Error(data.error));
    else task.resolve(data.svg);
  };
  const failed = event => {
    event.preventDefault?.();
    if (worker !== instance) return;
    instance.terminate();
    worker = undefined;
    for (const task of pending.values()) task.reject(new Error(event.message || "Typst worker failed"));
    pending.clear();
  };
  instance.onerror = failed;
  instance.onmessageerror = failed;
  return worker = instance;
}

export function compile(source) {
  return new Promise((resolve, reject) => {
    const instance = load();
    const id = ++sequence;
    pending.set(id, { resolve, reject });
    instance.postMessage({ id, source });
  });
}
