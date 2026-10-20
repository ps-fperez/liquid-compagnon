//! Création du code PIN (premier lancement) et déverrouillage (PIN ou biométrie).

use std::time::Duration;

use lc_shared_dto::{ErrorCode, PinInput, SessionStatusDto, SetupPinInput};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::style::BTN_PRIMARY;
use crate::icons::{Icon, IconName};
use crate::state::use_session;

const MIN_DIGITS: usize = 4;
const MAX_DIGITS: usize = 6;

/// Points indiquant le nombre de chiffres saisis (au moins 4 emplacements).
#[component]
fn PinDots(pin: RwSignal<String>, #[prop(into)] shake: Signal<bool>) -> impl IntoView {
    view! {
        <div
            class=move || format!("flex h-6 items-center justify-center gap-3 {}", if shake.get() { "animate-pulse" } else { "" })
            aria-live="polite"
            aria-label=move || format!("{} chiffres saisis", pin.with(String::len))
        >
            {move || {
                let entered = pin.with(String::len);
                (0..entered.clamp(MIN_DIGITS, MAX_DIGITS))
                    .map(|index| {
                        let filled = index < entered;
                        view! {
                            <span class=if filled {
                                "size-3.5 rounded-full bg-primary shadow-glow-primary"
                            } else {
                                "size-3.5 rounded-full border border-muted/60"
                            }></span>
                        }
                    })
                    .collect_view()
            }}
        </div>
    }
}

/// Pavé numérique. `on_full` est appelé quand 6 chiffres sont saisis.
#[component]
fn Keypad(
    pin: RwSignal<String>,
    #[prop(into)] disabled: Signal<bool>,
    on_full: Callback<()>,
    #[prop(optional)] on_biometric: Option<Callback<()>>,
) -> impl IntoView {
    let key = "flex h-16 items-center justify-center rounded-2xl text-2xl font-medium transition active:bg-surface-raised disabled:opacity-30";
    let digit = move |value: char| {
        view! {
            <button
                class=key
                aria-label=value.to_string()
                disabled=move || disabled.get()
                on:click=move |_| {
                    if pin.with(String::len) < MAX_DIGITS {
                        pin.update(|p| p.push(value));
                        if pin.with(String::len) == MAX_DIGITS {
                            on_full.run(());
                        }
                    }
                }
            >
                {value.to_string()}
            </button>
        }
    };
    view! {
        <div class="mx-auto grid w-full max-w-xs grid-cols-3 gap-2">
            {['1', '2', '3', '4', '5', '6', '7', '8', '9'].into_iter().map(digit).collect_view()}
            {match on_biometric {
                Some(callback) => view! {
                    <button class=format!("{key} text-primary") aria-label="Déverrouiller avec la biométrie" on:click=move |_| callback.run(())>
                        <Icon name=IconName::Fingerprint class="size-8" />
                    </button>
                }
                .into_any(),
                None => view! { <span></span> }.into_any(),
            }}
            {digit('0')}
            <button class=key aria-label="Effacer" disabled=move || disabled.get() on:click=move |_| pin.update(|p| { p.pop(); })>
                <Icon name=IconName::ChevronLeft class="size-7" />
            </button>
        </div>
    }
}

#[component]
fn PinLayout(title: Signal<String>, subtitle: Signal<String>, children: Children) -> impl IntoView {
    view! {
        <div class="mx-auto flex min-h-dvh max-w-md flex-col items-center justify-between gap-6 px-6 pt-[calc(3rem+env(safe-area-inset-top))] pb-[calc(2rem+env(safe-area-inset-bottom))]">
            <div class="flex flex-col items-center gap-3 text-center">
                <span class="flex size-14 items-center justify-center rounded-2xl bg-primary text-lg font-semibold text-bg shadow-glow-primary">
                    "LC"
                </span>
                <h1 class="text-xl font-medium">{move || title.get()}</h1>
                <p class="text-sm text-muted">{move || subtitle.get()}</p>
            </div>
            {children()}
        </div>
    }
}

/// Premier lancement : saisie puis confirmation du code PIN.
#[component]
pub fn SetupPin() -> impl IntoView {
    let session = use_session();
    let pin = RwSignal::new(String::new());
    let first = RwSignal::new(None::<String>);
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);

    let submit = Callback::new(move |_: ()| {
        let entered = pin.get_untracked();
        if entered.len() < MIN_DIGITS {
            error.set(Some("Le code PIN doit contenir 4 à 6 chiffres".into()));
            return;
        }
        error.set(None);
        match first.get_untracked() {
            None => {
                first.set(Some(entered));
                pin.set(String::new());
            }
            Some(initial) => {
                busy.set(true);
                spawn_local(async move {
                    match api::setup_pin(SetupPinInput {
                        pin: initial,
                        confirmation: entered,
                    })
                    .await
                    {
                        Ok(()) => session.refresh(),
                        Err(failure) => {
                            error.set(Some(failure.message));
                            first.set(None);
                            pin.set(String::new());
                        }
                    }
                    busy.set(false);
                });
            }
        }
    });

    let title = Signal::derive(move || {
        if first.with(Option::is_none) {
            "Créez votre code PIN"
        } else {
            "Confirmez votre code PIN"
        }
        .to_string()
    });
    let subtitle = Signal::derive(move || {
        if first.with(Option::is_none) {
            "4 à 6 chiffres. Il protège vos données, qui restent sur ce téléphone.".to_string()
        } else {
            "Saisissez le même code une seconde fois.".to_string()
        }
    });

    view! {
        <PinLayout title=title subtitle=subtitle>
            <div class="flex w-full flex-col items-center gap-4">
                <PinDots pin=pin shake=Signal::derive(move || error.with(Option::is_some)) />
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
            </div>
            <div class="flex w-full flex-col gap-4">
                <Keypad pin=pin disabled=busy on_full=submit />
                <button
                    class=BTN_PRIMARY
                    disabled=move || busy.get() || pin.with(String::len) < MIN_DIGITS
                    on:click=move |_| submit.run(())
                >
                    {move || if first.with(Option::is_none) { "Continuer" } else { "Valider" }}
                </button>
            </div>
        </PinLayout>
    }
}

/// Écran de verrouillage : PIN, biométrie si activée, compte à rebours après 5 échecs.
#[component]
pub fn LockScreen(status: SessionStatusDto) -> impl IntoView {
    let session = use_session();
    let pin = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let countdown = RwSignal::new(status.lockout_remaining_seconds);

    if let Ok(handle) = set_interval_with_handle(
        move || {
            if countdown.get_untracked().is_some() {
                countdown.update(|c| *c = c.and_then(|s| s.checked_sub(1)).filter(|s| *s > 0));
            }
        },
        Duration::from_secs(1),
    ) {
        on_cleanup(move || handle.clear());
    }

    let submit = Callback::new(move |_: ()| {
        let entered = pin.get_untracked();
        if entered.len() < MIN_DIGITS || busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            match api::unlock(PinInput { pin: entered }).await {
                Ok(()) => session.refresh(),
                Err(failure) => {
                    pin.set(String::new());
                    if matches!(failure.code, ErrorCode::WrongPin | ErrorCode::LockedOut) {
                        countdown.set(failure.lockout_seconds);
                    }
                    error.set(Some(failure.message));
                }
            }
            busy.set(false);
        });
    });

    let biometric = (status.biometric_enabled && status.biometric_available).then(|| {
        let prompt = Callback::new(move |_: ()| {
            spawn_local(async move {
                match api::unlock_with_biometric().await {
                    Ok(()) => session.refresh(),
                    Err(failure) => error.set(Some(failure.message)),
                }
            });
        });
        // Invite proposée d'emblée, le PIN reste disponible.
        prompt.run(());
        prompt
    });

    let locked_out = Signal::derive(move || countdown.get().is_some());
    let disabled = Signal::derive(move || busy.get() || locked_out.get());

    view! {
        <PinLayout
            title=Signal::derive(|| "Saisissez votre code PIN".to_string())
            subtitle=Signal::derive(|| "Liquid Compagnon est verrouillé.".to_string())
        >
            <div class="flex w-full flex-col items-center gap-4">
                <PinDots pin=pin shake=Signal::derive(move || error.with(Option::is_some)) />
                <p class="min-h-5 text-center text-sm text-loss" role="alert">
                    {move || match countdown.get() {
                        Some(seconds) => format!("Trop de tentatives. Réessayez dans {seconds} s"),
                        None => error.get().unwrap_or_default(),
                    }}
                </p>
            </div>
            <div class="flex w-full flex-col gap-4">
                {match biometric {
                    Some(callback) => view! { <Keypad pin=pin disabled=disabled on_full=submit on_biometric=callback /> }.into_any(),
                    None => view! { <Keypad pin=pin disabled=disabled on_full=submit /> }.into_any(),
                }}
                <button
                    class=BTN_PRIMARY
                    disabled=move || disabled.get() || pin.with(String::len) < MIN_DIGITS
                    on:click=move |_| submit.run(())
                >
                    <Icon name=IconName::Lock class="size-5" />
                    "Déverrouiller"
                </button>
            </div>
        </PinLayout>
    }
}
