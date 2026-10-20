// Parcours utilisateur de bout en bout dans l'app de bureau (même UI et même backend que l'APK),
// piloté par le protocole DevTools de la WebView. Base isolée (LC_DATA_DIR), captures d'écran.
// Usage : node scripts/e2e-ui.mjs <port> <dossier des captures>
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const [port = "9222", shots = "e2e-captures"] = process.argv.slice(2);
mkdirSync(shots, { recursive: true });

const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
const page = targets.find((t) => t.type === "page");
const ws = new WebSocket(page.webSocketDebuggerUrl);
let nextId = 0;
const pending = new Map();
const errors = [];
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.id && pending.has(msg.id)) {
    pending.get(msg.id)(msg.result ?? msg.error);
    pending.delete(msg.id);
  } else if (msg.method === "Runtime.exceptionThrown") {
    errors.push(msg.params.exceptionDetails.exception?.description ?? msg.params.exceptionDetails.text);
  } else if (msg.method === "Log.entryAdded" && msg.params.entry.level === "error") {
    errors.push(msg.params.entry.text);
  }
};
const send = (method, params = {}) =>
  new Promise((resolve) => {
    const id = ++nextId;
    pending.set(id, resolve);
    ws.send(JSON.stringify({ id, method, params }));
  });
await new Promise((resolve) => (ws.onopen = resolve));
await send("Runtime.enable");
await send("Log.enable");

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const evaluate = async (expression) =>
  (await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true })).result?.value;
const bodyText = () => evaluate("document.body.innerText");

let step = 0;
async function shot(name) {
  const { data } = await send("Page.captureScreenshot", { format: "png" });
  const file = join(shots, `${String(++step).padStart(2, "0")}-${name}.png`);
  writeFileSync(file, Buffer.from(data, "base64"));
  console.log(`  capture : ${file}`);
}

async function fail(message) {
  console.error(`ÉCHEC : ${message}`);
  console.error(`Texte affiché :\n${(await bodyText())?.trim()}`);
  await shot("echec");
  process.exit(1);
}

async function waitForText(text, timeout = 8000) {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    if ((await bodyText())?.includes(text)) return;
    await sleep(150);
  }
  await fail(`texte attendu introuvable : « ${text} »`);
}

/** Clique le bouton ou lien dont le libellé visible ou `aria-label` vaut `label`. */
async function click(label) {
  const ok = await evaluate(`(() => {
    const label = ${JSON.stringify(label)};
    const el = [...document.querySelectorAll("button, a")].find(e =>
      !e.disabled && (e.getAttribute("aria-label") === label || e.textContent.trim() === label));
    if (!el) return false;
    el.click();
    return true;
  })()`);
  if (!ok) await fail(`élément cliquable introuvable : « ${label} »`);
  await sleep(200);
}

/** Remplit le champ dont le libellé vaut `label` (déclenche l'événement input). */
async function fill(label, value) {
  const ok = await evaluate(`(() => {
    const label = ${JSON.stringify(label)};
    const field = [...document.querySelectorAll("label")].find(l => l.querySelector("span")?.textContent.trim() === label);
    const input = field?.querySelector("input");
    if (!input) return false;
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set;
    setter.call(input, ${JSON.stringify(value)});
    input.dispatchEvent(new Event("input", { bubbles: true }));
    return true;
  })()`);
  if (!ok) await fail(`champ introuvable : « ${label} »`);
}

async function typePin(pin) {
  for (const digit of pin) await click(digit);
}

console.log("1. Création du PIN");
await waitForText("Créez votre code PIN");
await shot("creation-pin");
await typePin("1234");
await click("Continuer");
await waitForText("Confirmez votre code PIN");
await typePin("1234");
await click("Valider");

console.log("2. Tableau de bord vide");
await waitForText("Patrimoine total");
await waitForText("Ajoutez votre premier compte");
await shot("dashboard-vide");

console.log("3. Création d'un Livret A");
await click("Comptes");
await click("Nouveau");
await waitForText("Nouveau compte");
await fill("Versement mensuel programmé (facultatif)", "200");
await shot("creation-compte");
await click("Créer le compte");
await waitForText("Mettre à jour le solde");

console.log("4. Mise à jour du solde");
await click("Mettre à jour le solde");
await fill("Nouveau solde", "10640,25");
await click("Enregistrer");
await waitForText("10 640,25 €");
await shot("livret-a");

console.log("5. Tableau de bord alimenté");
await click("Accueil");
await waitForText("Livret A");
const text = await bodyText();
if (!text.includes("10 640,25 €")) await fail("le total du tableau de bord est incorrect");
await shot("dashboard");

console.log("6. Verrouillage, mauvais PIN, déverrouillage");
await evaluate("window.__TAURI__.core.invoke('lock')");
await evaluate("location.reload()");
await sleep(1500);
await waitForText("Saisissez votre code PIN");
await typePin("0000");
await click("Déverrouiller");
await waitForText("Code PIN incorrect");
await shot("pin-incorrect");
await typePin("1234");
await click("Déverrouiller");
await waitForText("Patrimoine total");

ws.close();
if (errors.length > 0) {
  console.error(`ÉCHEC : ${errors.length} erreur(s) dans la WebView`);
  for (const error of errors) console.error(`  - ${error}`);
  process.exit(1);
}
console.log("OK : parcours complet réussi.");
