// Smoke test : se connecte à la WebView de l'app (protocole DevTools) et vérifie que l'UI a démarré
// sans erreur. Code de sortie 1 en cas d'échec.
// Usage : node scripts/smoke-inspect.mjs <port> <texte attendu>
const [port = "9222", expected = "Liquid Compagnon"] = process.argv.slice(2);

const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
const page = targets.find((t) => t.type === "page");
if (!page) {
  console.error("ÉCHEC : aucune page WebView trouvée");
  process.exit(1);
}

const ws = new WebSocket(page.webSocketDebuggerUrl);
let nextId = 0;
const pending = new Map();
const errors = [];

const send = (method, params = {}) =>
  new Promise((resolve) => {
    const id = ++nextId;
    pending.set(id, resolve);
    ws.send(JSON.stringify({ id, method, params }));
  });

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.id && pending.has(msg.id)) {
    pending.get(msg.id)(msg.result ?? msg.error);
    pending.delete(msg.id);
  } else if (msg.method === "Runtime.exceptionThrown") {
    errors.push(msg.params.exceptionDetails.exception?.description ?? msg.params.exceptionDetails.text);
  } else if (msg.method === "Runtime.consoleAPICalled" && msg.params.type === "error") {
    errors.push(msg.params.args.map((a) => a.value ?? a.description).join(" "));
  } else if (msg.method === "Log.entryAdded" && msg.params.entry.level === "error") {
    errors.push(msg.params.entry.text);
  }
};

await new Promise((resolve) => (ws.onopen = resolve));
await send("Runtime.enable");
await send("Log.enable");
await send("Page.reload", { ignoreCache: true });
await new Promise((resolve) => setTimeout(resolve, 4000));

const evaluate = async (expression) =>
  (await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true })).result?.value;
const body = (await evaluate("document.body.innerText")) ?? "";

// Appels IPC réels, sans effet de bord : au démarrage la session est verrouillée et les
// commandes métier doivent être refusées.
const ipc = await evaluate(`(async () => {
  const invoke = window.__TAURI__.core.invoke;
  const status = await invoke("session_status");
  let guard;
  try { await invoke("get_dashboard", { input: { force_refresh: false } }); guard = "accepté"; }
  catch (error) { guard = error && error.code; }
  return { unlocked: status.unlocked, pinKnown: typeof status.pin_set === "boolean", guard };
})()`);
ws.close();

console.log(`IPC : ${JSON.stringify(ipc)}`);
if (!ipc || ipc.unlocked !== false || !ipc.pinKnown || ipc.guard !== "session_locked") {
  errors.push("contrôle IPC : session non verrouillée au démarrage ou commande métier non protégée");
}

console.log(`URL : ${page.url}`);
console.log(`Texte affiché :\n${body.trim() || "(vide)"}`);

if (errors.length > 0) {
  console.error(`ÉCHEC : ${errors.length} erreur(s) dans la WebView`);
  for (const error of errors) console.error(`  - ${error}`);
  process.exit(1);
}
if (!body.includes(expected)) {
  console.error(`ÉCHEC : texte attendu introuvable : « ${expected} »`);
  process.exit(1);
}
console.log("OK : l'UI a démarré sans erreur.");
