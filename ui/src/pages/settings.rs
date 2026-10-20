//! Réglages : sécurité, clé de cotation, sauvegarde, à propos.

use lc_shared_dto::{
    ApiKeyInput, ChangePinInput, ErrorDto, ImportInput, PingResponse, QuoteSourceDto, SettingsDto,
    ToggleInput,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::style::{BTN_DANGER, BTN_PRIMARY, BTN_SECONDARY, SECTION_TITLE};
use crate::components::{BottomSheet, Card, ErrorBanner, Field, Skeleton};
use crate::files;
use crate::format;
use crate::state::{session_locked, use_session};
use crate::util::{load, run_action};

#[component]
pub fn SettingsPage() -> impl IntoView {
    let settings = RwSignal::new(None::<Result<SettingsDto, ErrorDto>>);
    let version = RwSignal::new(None::<Result<PingResponse, ErrorDto>>);
    let reload = Callback::new(move |_: ()| load(settings, api::get_settings));
    reload.run(());
    load(version, api::ping);

    view! {
        <h1 class="px-1 text-2xl font-medium">"Réglages"</h1>
        {move || match settings.get() {
            None => view! { <Skeleton class="h-64" /> }.into_any(),
            Some(Err(error)) => view! { <ErrorBanner message=error.message /> }.into_any(),
            Some(Ok(current)) => view! {
                <SecuritySection settings=current.clone() on_saved=reload />
                <PricingSection has_key=current.has_twelve_data_key on_saved=reload />
                <BackupSection settings=current on_saved=reload />
            }
            .into_any(),
        }}
        <p class="px-1 text-center text-xs text-muted">
            "Liquid Compagnon "
            {move || version.get().and_then(Result::ok).map(|v| format!("v{}", v.backend_version))}
            " · données stockées uniquement sur ce téléphone"
        </p>
    }
}

#[component]
fn SecuritySection(settings: SettingsDto, on_saved: Callback<()>) -> impl IntoView {
    let session = use_session();
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let change_open = RwSignal::new(false);
    let biometric = RwSignal::new(settings.biometric_enabled);
    let available = settings.biometric_available;

    let toggle = move |event| {
        let enabled = event_target_checked(&event);
        biometric.set(enabled);
        run_action(
            busy,
            error,
            api::set_biometric_enabled(ToggleInput { enabled }),
            move |()| on_saved.run(()),
        );
    };
    let lock_now = move |_| {
        spawn_local(async move {
            let _ = api::lock().await;
            session_locked();
            session.refresh();
        });
    };

    view! {
        <section class="flex flex-col gap-2">
            <h2 class=SECTION_TITLE>"Sécurité"</h2>
            <Card class="flex flex-col gap-3">
                <label class="flex min-h-11 items-center justify-between gap-3">
                    <span class="flex flex-col">
                        <span>"Déverrouillage biométrique"</span>
                        {(!available).then(|| view! {
                            <span class="text-xs text-muted">"Aucune biométrie enregistrée sur cet appareil"</span>
                        })}
                    </span>
                    <input
                        type="checkbox"
                        role="switch"
                        class="size-6 accent-primary"
                        disabled=!available
                        prop:checked=move || biometric.get()
                        on:change=toggle
                    />
                </label>
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                <button class=BTN_SECONDARY on:click=move |_| change_open.set(true)>"Changer le code PIN"</button>
                <button class=BTN_SECONDARY on:click=lock_now>"Verrouiller maintenant"</button>
            </Card>
        </section>
        <ChangePinSheet open=change_open />
    }
}

#[component]
fn ChangePinSheet(open: RwSignal<bool>) -> impl IntoView {
    let current = RwSignal::new(String::new());
    let new_pin = RwSignal::new(String::new());
    let confirmation = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let done = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let submit = move |_| {
        let input = ChangePinInput {
            current: current.get_untracked(),
            new_pin: new_pin.get_untracked(),
            confirmation: confirmation.get_untracked(),
        };
        run_action(busy, error, api::change_pin(input), move |()| {
            for field in [current, new_pin, confirmation] {
                field.set(String::new());
            }
            done.set(true);
            open.set(false);
        });
    };
    view! {
        <BottomSheet open=open title="Changer le code PIN">
            <div class="flex flex-col gap-4">
                <Field label="Code actuel" value=current input_mode="numeric" input_type="password" />
                <Field label="Nouveau code (4 à 6 chiffres)" value=new_pin input_mode="numeric" input_type="password" />
                <Field label="Confirmez le nouveau code" value=confirmation input_mode="numeric" input_type="password" />
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit>"Enregistrer"</button>
            </div>
        </BottomSheet>
        <Show when=move || done.get()>
            <p class="px-1 text-sm text-gain">"Code PIN modifié"</p>
        </Show>
    }
}

#[component]
fn PricingSection(has_key: bool, on_saved: Callback<()>) -> impl IntoView {
    let key = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let info = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);

    let save = move |_| {
        let value = key.get_untracked();
        if value.trim().is_empty() {
            return error.set(Some("Saisissez la clé fournie par Twelve Data".into()));
        }
        info.set(None);
        run_action(
            busy,
            error,
            api::set_twelve_data_api_key(ApiKeyInput { key: Some(value) }),
            move |()| {
                key.set(String::new());
                on_saved.run(());
                // Vérifie la clé et affiche l'offre associée (aucune donnée personnelle transmise).
                spawn_local(async move {
                    match api::price_source_status(QuoteSourceDto::TwelveData).await {
                        Ok(status) => info.set(Some(match status {
                            Some(status) => format!("Clé vérifiée : {status}"),
                            None => "Clé enregistrée".into(),
                        })),
                        Err(failure) => error.set(Some(format!(
                            "Clé enregistrée, mais la vérification a échoué : {}",
                            failure.message
                        ))),
                    }
                });
            },
        );
    };
    let clear = move |_| {
        run_action(
            busy,
            error,
            api::set_twelve_data_api_key(ApiKeyInput { key: None }),
            move |()| {
                info.set(Some("Clé effacée".into()));
                on_saved.run(());
            },
        );
    };

    view! {
        <section class="flex flex-col gap-2">
            <h2 class=SECTION_TITLE>"Cours des ETF"</h2>
            <Card class="flex flex-col gap-3">
                <p class="text-sm text-muted">
                    "Les ETF et actions sont cotés par Yahoo Finance, sans clé. Twelve Data est une source facultative : son offre gratuite ne couvre pas les places européennes (Paris, Amsterdam…)."
                </p>
                <p class="text-sm text-muted">
                    {if has_key { "Une clé Twelve Data est enregistrée." } else { "Aucune clé Twelve Data." }}
                    " Elle reste sur ce téléphone et n'est jamais exportée."
                </p>
                <Field label="Clé API" value=key input_type="password" placeholder="Votre clé Twelve Data" />
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                {move || info.get().map(|text| view! { <p class="text-sm text-gain">{text}</p> })}
                <button class=BTN_PRIMARY disabled=move || busy.get() on:click=save>"Enregistrer la clé"</button>
                {has_key.then(|| view! { <button class=BTN_DANGER disabled=move || busy.get() on:click=clear>"Effacer la clé"</button> })}
            </Card>
        </section>
    }
}

#[component]
fn BackupSection(settings: SettingsDto, on_saved: Callback<()>) -> impl IntoView {
    let export_open = RwSignal::new(false);
    let import_content = RwSignal::new(None::<String>);
    let error = RwSignal::new(None::<String>);
    let info = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);

    let export = move |_| {
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let result = match api::export_backup().await {
                Ok(backup) => files::save_text(&backup.file_name, &backup.content).await,
                Err(failure) => Err(failure.message),
            };
            match result {
                Ok(true) => {
                    info.set(Some("Sauvegarde enregistrée".into()));
                    export_open.set(false);
                    on_saved.run(());
                }
                Ok(false) => {}
                Err(message) => error.set(Some(message)),
            }
            busy.set(false);
        });
    };
    // Le fichier choisi ouvre directement la confirmation (pas d'effets croisés entre signaux :
    // écrire un signal notifie ses observateurs même si la valeur ne change pas).
    let confirm_open = RwSignal::new(false);
    let choose_import = move |_| {
        error.set(None);
        spawn_local(async move {
            match files::open_text().await {
                Ok(Some(content)) => {
                    import_content.set(Some(content));
                    confirm_open.set(true);
                }
                Ok(None) => {}
                Err(message) => error.set(Some(message)),
            }
        });
    };
    let import = move |_| {
        let Some(content) = import_content.get_untracked() else {
            return;
        };
        run_action(
            busy,
            error,
            api::import_backup(ImportInput { content }),
            move |summary| {
                info.set(Some(format!(
                    "Import terminé : {} compte(s), {} dépense(s), {} instantané(s)",
                    summary.accounts, summary.expenses, summary.snapshots
                )));
                confirm_open.set(false);
                on_saved.run(());
            },
        );
    };

    view! {
        <section class="flex flex-col gap-2">
            <h2 class=SECTION_TITLE>"Sauvegarde"</h2>
            <Card class="flex flex-col gap-3">
                <p class="text-sm text-muted">
                    "Dernier export : "
                    {settings.last_export_at.map_or_else(|| "jamais".to_string(), |at| format::date(at.date_naive()))}
                </p>
                {settings.export_reminder_due.then(|| view! {
                    <p class="rounded-xl bg-primary/10 px-3 py-2 text-sm text-primary-soft">
                        "Vos données n'existent que sur ce téléphone : exportez-les régulièrement."
                    </p>
                })}
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                {move || info.get().map(|text| view! { <p class="text-sm text-gain">{text}</p> })}
                <button class=BTN_PRIMARY on:click=move |_| export_open.set(true)>"Exporter mes données"</button>
                <button class=BTN_SECONDARY on:click=choose_import>"Importer une sauvegarde"</button>
            </Card>
        </section>

        <BottomSheet open=export_open title="Exporter mes données">
            <div class="flex flex-col gap-4">
                <p class="text-sm text-muted">
                    "Le fichier contient tous vos comptes, revenus et dépenses, en clair. Conservez-le dans un endroit sûr. Votre code PIN et votre clé Twelve Data n'en font pas partie."
                </p>
                <button class=BTN_PRIMARY disabled=move || busy.get() on:click=export>"Choisir l'emplacement"</button>
            </div>
        </BottomSheet>
        <BottomSheet open=confirm_open title="Remplacer toutes les données ?">
            <div class="flex flex-col gap-4">
                <p class="text-sm text-muted">
                    "Toutes les données actuelles seront remplacées par celles du fichier. Le fichier est vérifié avant : s'il est invalide, rien n'est modifié."
                </p>
                <button class=BTN_DANGER disabled=move || busy.get() on:click=import>"Remplacer mes données"</button>
            </div>
        </BottomSheet>
    }
}
