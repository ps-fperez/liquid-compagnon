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
  // Texte UTF-8 relu comme ANSI (« DÃ©tail ») : un fichier source a été mal réencodé.
  const garbled = (await bodyText())?.match(/\S*[ÃÂ][\u0080-¿©¨‰§ª´·]\S*/);
  if (garbled) await fail(`texte mal encodé à l'écran : « ${garbled[0]} »`);
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

/** Clique, dans le groupe intitulé `group` (« De », « Vers »…), le bouton libellé `label`. */
async function clickIn(group, label) {
  const ok = await evaluate(`(() => {
    const scope = document.querySelector('[role="dialog"]') ?? document;
    const title = [...scope.querySelectorAll("span")].find(s => s.textContent.trim() === ${JSON.stringify(group)});
    const el = title && [...title.parentElement.querySelectorAll("button")].find(b => b.textContent.trim() === ${JSON.stringify(label)});
    if (!el) return false;
    el.click();
    return true;
  })()`);
  if (!ok) await fail(`« ${label} » introuvable dans « ${group} »`);
  await sleep(200);
}

/** Coche la case de la feuille ouverte dont le libellé contient `text`. */
async function check(text) {
  const ok = await evaluate(`(() => {
    const label = [...document.querySelectorAll('[role="dialog"] label')].find(l => l.textContent.includes(${JSON.stringify(text)}));
    const box = label?.querySelector('input[type="checkbox"]');
    if (!box) return false;
    box.checked = true;
    box.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  })()`);
  if (!ok) await fail(`case introuvable : « ${text} »`);
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

/** Donne le focus au champ dont le libellé vaut `label`. */
async function focusField(label) {
  const ok = await evaluate(`(() => {
    const scope = document.querySelector('[role="dialog"]') ?? document;
    const field = [...scope.querySelectorAll("label")].find(l => l.querySelector("span")?.textContent.trim() === ${JSON.stringify(label)});
    const input = field?.querySelector("input");
    input?.focus();
    return Boolean(input);
  })()`);
  if (!ok) await fail(`champ introuvable : « ${label} »`);
}

/** Frappe réelle au clavier (événements clavier natifs) dans le champ qui a le focus. */
async function typeText(text) {
  for (const ch of text) {
    await send("Input.dispatchKeyEvent", { type: "keyDown", text: ch, key: ch });
    await send("Input.dispatchKeyEvent", { type: "keyUp", key: ch });
    await sleep(20);
  }
}

async function typePin(pin) {
  for (const digit of pin) await click(digit);
}

// Préférences d'appareil (bandeaux fermés, mode discret…) : la WebView les garde d'un lancement
// à l'autre ; le parcours repart de zéro, comme la base temporaire.
await evaluate("localStorage.clear()");
await evaluate("location.reload()");
await sleep(1500);

console.log("1. Création du PIN");
await waitForText("Créez votre code PIN");
await shot("creation-pin");
await typePin("1234");
await click("Continuer");
await waitForText("Confirmez votre code PIN");
await typePin("1234");
await click("Valider");

console.log("1 bis. Didacticiel de prise en main");
await waitForText("Bienvenue dans Liquid Compagnon");
await shot("didacticiel");
for (let i = 0; i < 4; i++) await click("Suivant");
await waitForText("Discret et protégé");
await click("Découvrir l'application");

console.log("2. Tableau de bord vide");
await waitForText("Patrimoine total");
await waitForText("Ajoutez votre premier compte");
await waitForText("Renseigner mon salaire"); // reste à dépenser sans revenu
await shot("dashboard-vide");

console.log("3. Création d'un Livret A");
await click("Comptes");
await click("Nouveau");
await waitForText("Nouveau compte");
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
await waitForText("10\u00a0640,25\u00a0€"); // le total défile jusqu'à sa valeur
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
await waitForText("Renseignez votre salaire net");
await click("Ajouter mon salaire"); // ajout directement depuis la carte vide
await fill("Salaire net mensuel (net à payer)", "3200");
await click("Enregistrer");
await waitForText("3 200,00 €");
await click("Saisie rapide");
await click("Dépense");
await waitForText("Nouvelle dépense");
// Saisie détaillée par une vraie frappe au clavier, libellé avant montant (ordre signalé en
// recette) : aperçu du montant reconnu, catégorie sélectionnée, puis « Enregistrer la dépense ».
await click("Date et libellé");
await focusField("Libellé (facultatif)");
await typeText("Courses");
await focusField("Montant");
await typeText("42,50");
await waitForText("= 42,50 €");
await click("Alimentation"); // sélectionne seulement, en saisie détaillée
await shot("depense-detaillee");
await click("Enregistrer la dépense");
await waitForText("Dépense de 42,50 € enregistrée"); // notification de succès
// Libellé habituel : reprend libellé et catégorie, et enregistre en un geste.
await click("Saisie rapide");
await click("Dépense");
await fill("Montant", "10");
await waitForText("Habituelles");
await click("Courses · Alimentation");
await waitForText("52,50 €");
await click("Ajouter une charge fixe");
await fill("Libellé", "Loyer");
await fill("Montant", "1300");
await click("Logement");
await click("Enregistrer");
await waitForText("Loyer");
await waitForText("1 847,50 €"); // 3 200 − 1 300 − 52,50 (seul mois de dépenses)
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
// Recherche d'actif : Yahoo Finance proposé par défaut, Twelve Data au choix (sans appel réseau).
await click("Ajouter une position"); // depuis la carte vide
await waitForText("Populaires");
await shot("pea-populaires");
await click("MSCI World (CW8)"); // suggestion choisie en un geste, sans réseau
await waitForText("CW8.PA");
await click("Changer");
if (process.env.LC_E2E_LIVE) {
  // Autocomplétion réelle (réseau) : résultats Yahoo Finance pendant la frappe, sans valider.
  await evaluate(`(() => {
    const input = document.querySelector('[aria-label="Rechercher un actif"]');
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set;
    setter.call(input, "LU1681043599"); // ISIN de CW8
    input.dispatchEvent(new Event("input", { bubbles: true }));
  })()`);
  await waitForText("CW8.PA", 10000);
  await shot("autocompletion-live");
  await click("Effacer la recherche");
}
await click("Twelve Data");
const checked = await evaluate(`document.querySelector('[role="dialog"] [role="radio"][aria-checked="true"]')?.textContent.trim()`);
if (checked !== "Twelve Data") await fail(`source sélectionnée inattendue : ${checked}`);
await shot("pea-recherche");
await click("Fermer");
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
// Mises à jour : un build local n'en propose jamais (aucune requête réseau).
await waitForText("Vérifier maintenant");
await waitForText("Aucune clé"); // état de la clé Twelve Data
await waitForText("build de développement");
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
await waitForText("Clé Twelve Data absente ou invalide");
await waitForText("prix de revient"); // valorisé sans cours ni appel réseau
await waitForText("12 140,25 €"); // 10 640,25 + 500 + 2 × 500
await shot("cle-absente");
// Avertissement refermable : il disparaît et reste fermé après un rechargement.
await click("Fermer l'avertissement");
if ((await bodyText()).includes("Clé Twelve Data absente ou invalide")) await fail("l'avertissement fermé reste affiché");
await click("Comptes");
await click("Accueil");
await sleep(500);
if ((await bodyText()).includes("Clé Twelve Data absente ou invalide")) await fail("l'avertissement fermé est réapparu");

console.log("12. Compte courant : solde de référence, dépense rattachée, découvert");
await click("Comptes");
await click("Nouveau");
await click("Compte courant");
await click("Créer le compte");
await waitForText("Aucun mouvement depuis");
await click("Mettre à jour le solde");
await fill("Nouveau solde", "1000");
await click("Enregistrer");
await waitForText("1 000,00 €");
await sleep(50); // la dépense est saisie strictement après la référence
await click("Saisie rapide");
await click("Dépense");
await waitForText("Payé depuis");
await fill("Montant", "20");
await click("Alimentation"); // compte courant présélectionné : toujours trois gestes
await waitForText("72,50 €"); // dépenses du mois : 42,50 + 10 + 20
const openChecking = () =>
  evaluate(`(() => {
    const link = [...document.querySelectorAll('a[href^="/comptes/"]')].find(a => a.textContent.includes("Compte courant"));
    link?.click();
    return Boolean(link);
  })()`);
await click("Comptes");
await waitForText("Compte courant");
if (!(await openChecking())) await fail("compte courant introuvable dans la liste");
await waitForText("Dernier solde saisi");
await waitForText("980,00 €"); // 1 000 − 20
await shot("compte-courant");
await click("Mettre à jour le solde");
await fill("Nouveau solde", "85,40");
await evaluate(`(() => {
  const box = document.querySelector('[role="dialog"] input[type="checkbox"]');
  box.checked = true;
  box.dispatchEvent(new Event("change", { bubbles: true }));
})()`);
await click("Enregistrer");
await waitForText("−85,40 €");
await waitForText("Aucun mouvement depuis"); // la nouvelle référence inclut la dépense
await click("Accueil");
await waitForText("12 054,85 €"); // 12 140,25 − 85,40
await shot("compte-courant-decouvert");

console.log("13. Mois type : autre livret, virements, compte hors patrimoine, abondement, prélèvement");
await click("Comptes");
await click("Nouveau");
await click("Autre livret");
await fill("Nom", "Livret jeune");
await click("Créer le compte");
await waitForText("Taux et plafond");
await waitForText("Plafond de versement");
await waitForText("Aucun mouvement depuis");
// Virement ponctuel depuis la saisie rapide : le compte courant est proposé au départ.
await click("Saisie rapide");
await click("Virement");
await waitForText("Nouveau virement");
await clickIn("Vers", "Livret A");
await click("Ponctuel");
await fill("Montant envoyé", "300");
await waitForText("10 940,25 €"); // aperçu : 10 640,25 + 300
await shot("virement-apercu");
await click("Enregistrer le virement");
await waitForText("Virement enregistré");
await waitForText("10 940,25 €");
// Virement mensuel vers un compte hors patrimoine créé depuis la feuille.
await click("Saisie rapide");
await click("Virement");
await clickIn("Vers", "+ Hors patrimoine");
await fill("Nom du compte hors patrimoine", "Compte joint");
await click("Ajouter");
await waitForText("Compte hors patrimoine ajouté");
await fill("Montant envoyé", "1560");
await click("Enregistrer le virement");
await waitForText("Virement programmé");
await waitForText("Compte joint"); // section « Hors patrimoine »
await waitForText("utilisé");
// Rapatriement ponctuel d'un livret vers le compte courant.
await click("Saisie rapide");
await click("Virement");
await clickIn("De", "Livret A");
await clickIn("Vers", "Compte courant");
await click("Ponctuel");
await fill("Montant envoyé", "100");
await click("Enregistrer le virement");
await waitForText("Virement enregistré");
await waitForText("10 840,25 €");
if (!(await openChecking())) await fail("compte courant introuvable dans la liste");
await waitForText("Virement reçu");
await waitForText("−285,40 €"); // −85,40 − 300 + 100 (le mensuel du jour est inclus dans la référence)
await waitForText("Chaque mois, le");
await shot("compte-courant-virements");
// Épargne salariale : versement annuel avec abondement net de prélèvements sociaux.
await click("Comptes");
await click("Nouveau");
await click("Épargne salariale");
await click("Créer le compte");
await waitForText("Nouveau virement");
await click("Nouveau virement");
await clickIn("De", "Compte courant");
await click("Annuel");
await fill("Montant envoyé", "500");
await check("Montant reçu différent");
await fill("Montant reçu", "1493,30");
await waitForText("Abondement net : +993,30 €");
await shot("virement-abondement");
await click("Enregistrer le virement");
await waitForText("Virement programmé");
await waitForText("Chaque année, le");
await waitForText("envoyé 500,00 € · reçu 1 493,30 €");
// Budget : virement vers l'extérieur et charge prélevée sur le compte courant.
await click("Budget");
await waitForText("Virements hors patrimoine");
await waitForText("1 560,00 €");
await click("Ajouter une charge fixe");
await fill("Libellé", "Box internet");
await fill("Montant", "30");
await click("Logement");
await waitForText("Jour du prélèvement");
await fill("Jour du prélèvement", "5");
await click("Enregistrer");
await waitForText("prélevée le 5");
await shot("budget-virements");
// Virements entre comptes suivis : le patrimoine total ne bouge pas.
await click("Accueil");
await waitForText("12 054,85 €");
console.log("14. Reste à dépenser et annulation d'une saisie");
// 3 200 + 100 rapatriés − 1 330 de charges − 800 d'épargne programmée (Livret A, épargne
// salariale) − 1 560 vers le compte joint = −390 ; moins 72,50 de dépenses.
await waitForText("Reste à dépenser ·");
await waitForText("Budget dépassé");
await waitForText("−462,50\u00a0€");
await shot("reste-a-depenser");
await click("Détail du reste à dépenser");
await waitForText("Enveloppe du mois");
await waitForText("−390,00\u00a0€");
await waitForText("Rapatriements d'épargne");
await waitForText("+100,00\u00a0€");
await waitForText("−800,00\u00a0€");
await waitForText("dépassent vos revenus du mois de 390,00\u00a0€");
await waitForText("Alimentation"); // où part l'argent
await shot("reste-a-depenser-detail");
await click("Fermer");
await click("Saisie rapide");
await click("Dépense");
await fill("Montant", "18,40");
await click("Alimentation");
await waitForText("Dépense de 18,40\u00a0€ enregistrée");
await shot("notification-annuler");
await click("Annuler");
await waitForText("Dépense annulée");
await sleep(500);
if ((await bodyText()).includes("18,40")) await fail("la dépense annulée est encore affichée");
await click("Accueil");
await waitForText("−462,50\u00a0€");
ws.close();
if (errors.length > 0) {
  console.error(`ÉCHEC : ${errors.length} erreur(s) dans la WebView`);
  for (const error of errors) console.error(`  - ${error}`);
  process.exit(1);
}
console.log("OK : parcours complet réussi.");
