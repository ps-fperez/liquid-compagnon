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
/** Évalue une expression dans la page ; échoue au bout de 10 s plutôt que de bloquer le test. */
const evaluate = async (expression) => {
  const timeout = new Promise((_, reject) => setTimeout(() => reject(new Error(`délai dépassé : ${expression.slice(0, 80)}`)), 10_000));
  const result = await Promise.race([send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true }), timeout]);
  return result.result?.value;
};
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
    const scope = document.querySelector('[role="dialog"]') ?? document;
    const el = [...scope.querySelectorAll("button, a")].find(e =>
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
    const scope = document.querySelector('[role="dialog"]') ?? document;
    const field = [...scope.querySelectorAll("label")].find(l => l.querySelector("span")?.textContent.trim() === label);
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
await waitForText("10 640,25 €");
await shot("livret-a");

console.log("5. Tableau de bord alimenté");
await click("Accueil");
await waitForText("Livret A");
const text = await bodyText();
if (!text.includes("10 640,25 €")) await fail("le total du tableau de bord est incorrect");
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

console.log("7. Budget : salaire, dépense en saisie rapide, charge fixe");
await click("Budget");
await waitForText("Salaire net mensuel");
await click("Nouveau salaire");
await fill("Salaire net mensuel (net à payer)", "3200");
await click("Enregistrer");
await waitForText("3 200,00 €");
await click("Saisie rapide");
await click("Dépense");
await waitForText("Nouvelle dépense");
await fill("Montant", "42,50");
await click("Alimentation"); // 3e interaction : la catégorie enregistre
await waitForText("42,50 €");
await click("Ajouter une charge fixe");
await fill("Libellé", "Loyer");
await fill("Montant", "1300");
await click("Logement");
await click("Enregistrer");
await waitForText("Loyer");
await waitForText("1 857,50 €"); // 3 200 − 1 300 − 42,50 (seul mois de dépenses)
await shot("budget");

console.log("8. Projection");
await click("Projection");
await waitForText("Capacité d'épargne mensuelle");
await waitForText("Neutre");
await shot("projection");

console.log("9. PEA : espèces");
await click("Comptes");
await click("Nouveau");
await click("PEA");
await click("Créer le compte");
await waitForText("Ajouter une position");
await click("Modifier les espèces");
await fill("Solde espèces", "500");
await click("Enregistrer");
await waitForText("500,00 €");
await shot("pea");
await click("Accueil");
await waitForText("11 140,25 €");

console.log("10. Réglages et sauvegarde");
await click("Réglages");
await waitForText("Exporter mes données");
const plugins = await evaluate(
  "typeof window.__TAURI__.dialog?.save + '/' + typeof window.__TAURI__.fs?.writeTextFile + '/' + typeof window.__TAURI__.fs?.readTextFile",
);
if (plugins !== "function/function/function") await fail(`plugins dialog/fs non exposés : ${plugins}`);
const backup = await evaluate("window.__TAURI__.core.invoke('export_backup').then(b => b.content)");
if (!backup?.includes('"schema_version"') || backup.includes("argon2")) await fail("export invalide ou contenant le PIN");
await shot("reglages");

console.log("11. Robustesse : import invalide, clé Twelve Data absente");
const invoke = (command, input) =>
  evaluate(`window.__TAURI__.core.invoke(${JSON.stringify(command)}, ${JSON.stringify(input === undefined ? {} : { input })})`);
const rejected = await evaluate(
  `window.__TAURI__.core.invoke('import_backup', { input: { content: '{"schema_version": 1, "oups": true}' } }).then(() => 'accepté', e => e.code)`,
);
if (rejected !== "backup") await fail(`un fichier invalide devait être refusé (résultat : ${rejected})`);
const remaining = await invoke("list_accounts", { include_archived: true });
if (remaining?.accounts?.length !== 2) await fail("l'import refusé a modifié les données");
const pea = remaining.accounts.find((a) => a.account_type === "pea");
await invoke("upsert_position", {
  account_id: pea.id,
  asset: { source: "twelve_data", symbol: "CW8:Euronext", display_name: "Amundi MSCI World", currency: "EUR" },
  quantity: "2",
  unit_cost: "500",
});
await click("Accueil");
await waitForText("Ajoutez une clé Twelve Data");
await waitForText("prix de revient"); // valorisé sans cours ni appel réseau
await waitForText("12 140,25 €"); // 10 640,25 + 500 + 2 × 500
await shot("cle-absente");

ws.close();
if (errors.length > 0) {
  console.error(`ÉCHEC : ${errors.length} erreur(s) dans la WebView`);
  for (const error of errors) console.error(`  - ${error}`);
  process.exit(1);
}
console.log("OK : parcours complet réussi.");
