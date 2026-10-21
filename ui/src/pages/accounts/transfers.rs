//! Virements : feuille de saisie (aperçu, plafond, montant reçu, fréquence), fil des mouvements
//! d'un compte et virements programmés.

use chrono::Datelike;
use lc_shared_dto::{
    AccountDetailsDto, AccountDto, AccountListDto, Decimal, EndpointDto, ExternalAccountDto,
    IdInput, IncludeArchivedInput, MovementDto, MovementKindDto, NaiveDate, NameInput, RenameInput,
    ScheduleDto, TransferDto, TransferInput,
};
use leptos::prelude::*;

use crate::api;
use crate::components::Busy;
use crate::components::amount_hint;
use crate::components::style::{BTN_DANGER, BTN_GHOST, BTN_PRIMARY, LABEL, SECTION_TITLE};
use crate::components::toast;
use crate::components::{Amount, BottomSheet, Card, EmptyState, Field};
use crate::format;
use crate::icons::{Icon, IconName};
use crate::util::{Loaded, load, run_action, today};

const MONTHS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

fn chip_class(active: bool) -> String {
    format!(
        "min-h-11 rounded-xl border px-4 text-sm transition {}",
        if active {
            "border-primary bg-primary/15 text-ink"
        } else {
            "border-line text-muted"
        }
    )
}

/// Fréquence lisible : « Chaque mois, le 5 », « Chaque année, le 15 janvier », « Le 21/10/2026 ».
pub fn schedule_label(schedule: &ScheduleDto) -> String {
    match schedule {
        ScheduleDto::Once { date } => format!("Le {}", format::date(*date)),
        ScheduleDto::Monthly { day, until, .. } => {
            let end = until
                .as_deref()
                .map(|m| format!(", jusqu'en {}", format::month_label(m)))
                .unwrap_or_default();
            format!("Chaque mois, le {day}{end}")
        }
        ScheduleDto::Yearly {
            month,
            day,
            until_year,
            ..
        } => {
            let name = MONTHS.get(*month as usize - 1).copied().unwrap_or_default();
            let end = until_year
                .map(|year| format!(", jusqu'en {year}"))
                .unwrap_or_default();
            format!("Chaque année, le {day} {name}{end}")
        }
    }
}

/// Jour `day` du mois, ramené au dernier jour d'un mois plus court.
fn clamped(year: i32, month: u32, day: u8) -> Option<NaiveDate> {
    (1..=u32::from(day))
        .rev()
        .find_map(|d| NaiveDate::from_ymd_opt(year, month, d))
}

/// Montant disponible sur un compte, celui qu'un virement fait bouger.
fn movable_balance(account: &AccountDto) -> Decimal {
    match &account.details {
        AccountDetailsDto::Savings { balance, .. }
        | AccountDetailsDto::Checking { balance, .. } => *balance,
        AccountDetailsDto::Securities { cash, .. } => *cash,
        AccountDetailsDto::EmployeeSavings { .. } => account.value,
    }
}

fn ceiling_of(account: &AccountDto) -> Option<Decimal> {
    match &account.details {
        AccountDetailsDto::Savings {
            deposit_ceiling, ..
        } => *deposit_ceiling,
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Frequency {
    Once,
    Monthly,
    Yearly,
}

impl Frequency {
    const ALL: [Frequency; 3] = [Frequency::Once, Frequency::Monthly, Frequency::Yearly];

    fn label(self) -> &'static str {
        match self {
            Frequency::Once => "Ponctuel",
            Frequency::Monthly => "Mensuel",
            Frequency::Yearly => "Annuel",
        }
    }
}

/// Feuille de virement : création (côtés éventuellement pré-remplis) ou modification.
#[component]
pub fn TransferSheet(
    open: RwSignal<bool>,
    #[prop(default = None)] existing: Option<TransferDto>,
    #[prop(default = None)] from: Option<EndpointDto>,
    #[prop(default = None)] to: Option<EndpointDto>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let title = if existing.is_some() {
        "Virement"
    } else {
        "Nouveau virement"
    };
    let preset = StoredValue::new((existing, from, to));
    view! {
        <BottomSheet open=open title=title>
            {move || {
                let (existing, from, to) = preset.get_value();
                view! { <TransferForm open=open existing=existing from=from to=to on_saved=on_saved /> }
            }}
        </BottomSheet>
    }
}

#[component]
fn TransferForm(
    open: RwSignal<bool>,
    existing: Option<TransferDto>,
    from: Option<EndpointDto>,
    to: Option<EndpointDto>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let accounts: Loaded<AccountListDto> = RwSignal::new(None);
    let externals: Loaded<Vec<ExternalAccountDto>> = RwSignal::new(None);
    load(accounts, || {
        api::list_accounts(IncludeArchivedInput {
            include_archived: false,
        })
    });
    let reload_externals = Callback::new(move |_: ()| load(externals, api::list_external_accounts));
    reload_externals.run(());

    let editing = existing.is_some();
    let source = RwSignal::new(existing.as_ref().map(|t| t.from.clone()).or(from));
    let target = RwSignal::new(existing.as_ref().map(|t| t.to.clone()).or(to));
    let sent = RwSignal::new(
        existing
            .as_ref()
            .map(|t| format::quantity(t.sent))
            .unwrap_or_default(),
    );
    let different = RwSignal::new(existing.as_ref().is_some_and(|t| t.received != t.sent));
    let received = RwSignal::new(
        existing
            .as_ref()
            .filter(|t| t.received != t.sent)
            .map(|t| format::quantity(t.received))
            .unwrap_or_default(),
    );
    let label = RwSignal::new(
        existing
            .as_ref()
            .and_then(|t| t.label.clone())
            .unwrap_or_default(),
    );
    let (frequency, first, until, original_day) = match existing.as_ref().map(|t| &t.schedule) {
        Some(ScheduleDto::Once { date }) => (Frequency::Once, Some(*date), String::new(), None),
        Some(ScheduleDto::Monthly { day, from, until }) => (
            Frequency::Monthly,
            from.split_once('-')
                .and_then(|(y, m)| clamped(y.parse().ok()?, m.parse().ok()?, *day)),
            until.clone().unwrap_or_default(),
            Some(*day),
        ),
        Some(ScheduleDto::Yearly {
            month,
            day,
            from_year,
            until_year,
        }) => (
            Frequency::Yearly,
            clamped(*from_year, *month, *day),
            until_year.map(|y| y.to_string()).unwrap_or_default(),
            Some(*day),
        ),
        None => (Frequency::Monthly, None, String::new(), None),
    };
    let frequency = RwSignal::new(frequency);
    let date = RwSignal::new(first.unwrap_or_else(today).to_string());
    let until = RwSignal::new(until);
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let transfer_id = StoredValue::new(existing.map(|t| t.id));

    // Sans côté pré-rempli, le virement part du premier compte courant.
    Effect::new(move |_| {
        let Some(Ok(list)) = accounts.get() else {
            return;
        };
        if source.get_untracked().is_none()
            && target.get_untracked().is_none()
            && let Some(checking) = list
                .accounts
                .iter()
                .find(|a| matches!(a.details, AccountDetailsDto::Checking { .. }))
        {
            source.set(Some(EndpointDto::Account(checking.id.clone())));
        }
    });

    let tracked = Signal::derive(move || {
        accounts
            .get()
            .and_then(Result::ok)
            .map(|list| list.accounts)
            .unwrap_or_default()
    });
    let external_list =
        Signal::derive(move || externals.get().and_then(Result::ok).unwrap_or_default());

    let sent_amount = move || format::parse_amount(&sent.get()).ok();
    let received_amount = move || {
        if different.get() {
            format::parse_amount(&received.get()).ok()
        } else {
            sent_amount()
        }
    };
    let received_hint = Signal::derive(move || {
        let raw = received.get();
        Some(match (sent_amount(), format::parse_amount(&raw).ok()) {
            (Some(out), Some(arrived)) if arrived > out => {
                format!("Abondement net : +{}", format::amount(arrived - out))
            }
            (Some(out), Some(arrived)) if arrived < out => {
                format!("Frais ou prélèvements : −{}", format::amount(out - arrived))
            }
            _ => amount_hint(&raw),
        })
    });

    let submit = move |_| {
        let Some(from) = source.get_untracked() else {
            return error.set(Some("Choisissez le compte de départ".into()));
        };
        let Some(to) = target.get_untracked() else {
            return error.set(Some("Choisissez le compte d'arrivée".into()));
        };
        let sent = match format::parse_amount(&sent.get_untracked()) {
            Ok(amount) => amount,
            Err(message) => return error.set(Some(message)),
        };
        let received = if different.get_untracked() {
            match format::parse_amount(&received.get_untracked()) {
                Ok(amount) => Some(amount),
                Err(message) => return error.set(Some(format!("Montant reçu : {message}"))),
            }
        } else {
            None
        };
        let schedule = match build_schedule(
            frequency.get_untracked(),
            &date.get_untracked(),
            &until.get_untracked(),
            original_day,
        ) {
            Ok(schedule) => schedule,
            Err(message) => return error.set(Some(message)),
        };
        let recurring = !matches!(schedule, ScheduleDto::Once { .. });
        let input = TransferInput {
            id: transfer_id.get_value(),
            from,
            to,
            sent,
            received,
            label: Some(label.get_untracked()).filter(|l| !l.trim().is_empty()),
            schedule,
        };
        run_action(busy, error, api::save_transfer(input), move |_: String| {
            toast::success(if recurring {
                "Virement programmé"
            } else {
                "Virement enregistré"
            });
            open.set(false);
            on_saved.run(());
        });
    };
    let remove = move |_| {
        let Some(id) = transfer_id.get_value() else {
            return;
        };
        run_action(
            busy,
            error,
            api::delete_transfer(IdInput { id }),
            move |()| {
                toast::success("Virement supprimé");
                open.set(false);
                on_saved.run(());
            },
        );
    };

    view! {
        <div class="flex flex-col gap-4">
            <EndpointChoice label="De" selected=source other=target accounts=tracked externals=external_list on_external_created=reload_externals />
            <EndpointChoice label="Vers" selected=target other=source accounts=tracked externals=external_list on_external_created=reload_externals />
            <Field label="Montant envoyé" value=sent input_mode="decimal" suffix="€" hint=Signal::derive(move || Some(amount_hint(&sent.get()))) />
            <label class="flex min-h-11 items-center justify-between gap-3">
                <span class="flex flex-col">
                    <span class="text-sm">"Montant reçu différent"</span>
                    <span class="text-xs text-muted">"Abondement de l'employeur, frais ou prélèvements sociaux"</span>
                </span>
                <input
                    type="checkbox"
                    class="size-5 shrink-0 accent-primary"
                    prop:checked=move || different.get()
                    on:change=move |event| different.set(event_target_checked(&event))
                />
            </label>
            <Show when=move || different.get()>
                <div class="animate-fade-in">
                    <Field label="Montant reçu" value=received input_mode="decimal" suffix="€" hint=received_hint />
                </div>
            </Show>

            <div class="flex flex-col gap-1.5">
                <span class=LABEL>"Fréquence"</span>
                <div class="flex flex-wrap gap-2" role="radiogroup" aria-label="Fréquence">
                    {Frequency::ALL.into_iter().map(|f| view! {
                        <button
                            role="radio"
                            aria-checked=move || (frequency.get() == f).to_string()
                            class=move || chip_class(frequency.get() == f)
                            on:click=move |_| frequency.set(f)
                        >
                            {f.label()}
                        </button>
                    }).collect_view()}
                </div>
            </div>
            {move || match frequency.get() {
                Frequency::Once => view! { <Field label="Date" value=date input_type="date" /> }.into_any(),
                Frequency::Monthly => view! {
                    <div class="grid grid-cols-2 gap-2">
                        <Field label="Premier virement" value=date input_type="date" />
                        <Field label="Jusqu'à (facultatif)" value=until input_type="month" />
                    </div>
                }.into_any(),
                Frequency::Yearly => view! {
                    <div class="grid grid-cols-2 gap-2">
                        <Field label="Premier virement" value=date input_type="date" />
                        <Field label="Jusqu'en (facultatif)" value=until input_mode="numeric" placeholder="2035" />
                    </div>
                }.into_any(),
            }}
            <Field label="Libellé (facultatif)" value=label placeholder="Épargne du mois" />

            {move || preview(&tracked.get(), source.get(), target.get(), sent_amount(), received_amount(), frequency.get())}

            <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
            <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit><Busy busy=busy />"Enregistrer le virement"</button>
            {editing.then(|| view! { <button class=BTN_DANGER disabled=move || busy.get() on:click=remove><Busy busy=busy />"Supprimer ce virement"</button> })}
        </div>
    }
}

fn build_schedule(
    frequency: Frequency,
    date: &str,
    until: &str,
    original_day: Option<u8>,
) -> Result<ScheduleDto, String> {
    let date = NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d")
        .map_err(|_| "Choisissez la date du virement".to_string())?;
    // Un jour 31 ramené au 30 à l'affichage reste le 31 s'il n'a pas été modifié.
    let day = original_day
        .filter(|d| clamped(date.year(), date.month(), *d) == Some(date))
        .unwrap_or(date.day() as u8);
    let until = until.trim();
    Ok(match frequency {
        Frequency::Once => ScheduleDto::Once { date },
        Frequency::Monthly => ScheduleDto::Monthly {
            day,
            from: format::iso_month(date),
            until: (!until.is_empty()).then(|| until.to_string()),
        },
        Frequency::Yearly => ScheduleDto::Yearly {
            month: date.month(),
            day,
            from_year: date.year(),
            until_year: if until.is_empty() {
                None
            } else {
                Some(
                    until
                        .parse()
                        .map_err(|_| "Année de fin invalide (ex. 2035)".to_string())?,
                )
            },
        },
    })
}

/// Soldes avant et après le premier virement, et alerte de plafond.
fn preview(
    accounts: &[AccountDto],
    from: Option<EndpointDto>,
    to: Option<EndpointDto>,
    sent: Option<Decimal>,
    received: Option<Decimal>,
    frequency: Frequency,
) -> Option<impl IntoView + use<>> {
    let (sent, received) = (sent?, received?);
    let find = |endpoint: &Option<EndpointDto>| match endpoint {
        Some(EndpointDto::Account(id)) => accounts.iter().find(|a| &a.id == id).cloned(),
        _ => None,
    };
    let (source, target) = (find(&from), find(&to));
    if source.is_none() && target.is_none() {
        return None;
    }
    let row = |account: &AccountDto, delta: Decimal| {
        let before = movable_balance(account);
        view! {
            <div class="flex items-center justify-between gap-3 text-sm">
                <span class="flex min-w-0 items-center gap-2">
                    <span class="size-2 shrink-0 rounded-full" style=format!("background-color: {}", account.color)></span>
                    <span class="truncate">{account.name.clone()}</span>
                </span>
                <span class="flex items-center gap-1.5 tabular-nums">
                    <span class="text-muted">{format::amount(before)}</span>
                    <Icon name=IconName::ChevronRight class="size-4 text-muted" />
                    <span class="font-medium">{format::amount(before + delta)}</span>
                </span>
            </div>
        }
    };
    let ceiling = target.as_ref().and_then(|account| {
        let limit = ceiling_of(account)?;
        let balance = movable_balance(account);
        let room = (limit - balance).max(Decimal::ZERO);
        if received > room {
            return Some(view! {
                <p class="rounded-xl bg-secondary/10 px-3 py-2 text-sm text-secondary-light">
                    "Au-delà du plafond de versement (" {format::amount(limit)} ") : ce compte ne peut plus recevoir que " {format::amount(room)} "."
                </p>
            }.into_any());
        }
        let months = (room / received).floor();
        (frequency == Frequency::Monthly && months < Decimal::from(36)).then(|| view! {
            <p class="text-xs text-muted">
                "À ce rythme, le plafond de " {format::amount(limit)} " sera atteint dans " {months.to_string()} " mois environ."
            </p>
        }.into_any())
    });
    let rhythm = match frequency {
        Frequency::Once => "Aperçu",
        Frequency::Monthly => "Aperçu du premier virement, puis chaque mois",
        Frequency::Yearly => "Aperçu du premier virement, puis chaque année",
    };
    Some(view! {
        <Card class="flex animate-fade-in flex-col gap-2">
            <span class="text-xs text-muted">{rhythm}</span>
            {source.as_ref().map(|account| row(account, -sent))}
            {target.as_ref().map(|account| row(account, received))}
            {ceiling}
        </Card>
    })
}

/// Côté d'un virement : comptes suivis, comptes hors patrimoine et création en ligne d'un compte
/// hors patrimoine.
#[component]
fn EndpointChoice(
    label: &'static str,
    selected: RwSignal<Option<EndpointDto>>,
    other: RwSignal<Option<EndpointDto>>,
    accounts: Signal<Vec<AccountDto>>,
    externals: Signal<Vec<ExternalAccountDto>>,
    on_external_created: Callback<()>,
) -> impl IntoView {
    let adding = RwSignal::new(false);
    let name = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let choose = move |endpoint: EndpointDto| {
        // Pas de virement d'un compte vers lui-même, ni entre deux comptes hors patrimoine.
        let clash = other.with_untracked(|o| {
            o.as_ref().is_some_and(|o| {
                o == &endpoint
                    || (matches!(o, EndpointDto::External(_))
                        && matches!(endpoint, EndpointDto::External(_)))
            })
        });
        if clash {
            other.set(None);
        }
        selected.set(Some(endpoint));
    };
    let create = move |_| {
        let input = NameInput {
            name: name.get_untracked(),
        };
        run_action(
            busy,
            error,
            api::create_external_account(input),
            move |id: String| {
                toast::success("Compte hors patrimoine ajouté");
                name.set(String::new());
                adding.set(false);
                choose(EndpointDto::External(id));
                on_external_created.run(());
            },
        );
    };
    let chip = move |endpoint: EndpointDto, text: String, color: Option<String>, external: bool| {
        let check = endpoint.clone();
        let active = Memo::new(move |_| selected.with(|s| s.as_ref() == Some(&check)));
        view! {
            <button
                class=move || format!(
                    "{} {}",
                    chip_class(active.get()),
                    if external { "border-dashed" } else { "" },
                )
                aria-pressed=move || active.get().to_string()
                on:click=move |_| choose(endpoint.clone())
            >
                <span class="flex items-center gap-2">
                    {color.map(|c| view! { <span class="size-2 rounded-full" style=format!("background-color: {c}")></span> })}
                    {text}
                </span>
            </button>
        }
    };
    view! {
        <div class="flex flex-col gap-1.5">
            <span class=LABEL>{label}</span>
            <div class="flex flex-wrap gap-2">
                {move || accounts.get().into_iter().map(|a| chip(EndpointDto::Account(a.id), a.name, Some(a.color), false)).collect_view()}
                {move || externals.get().into_iter().map(|e| chip(EndpointDto::External(e.id), e.name, None, true)).collect_view()}
                <button class="min-h-11 rounded-xl border border-dashed border-primary/40 px-4 text-sm text-primary-soft" on:click=move |_| adding.update(|a| *a = !*a)>
                    "+ Hors patrimoine"
                </button>
            </div>
            <Show when=move || adding.get()>
                <div class="flex animate-fade-in flex-col gap-2 rounded-xl border border-line p-3">
                    <Field label="Nom du compte hors patrimoine" value=name placeholder="Compte joint" hint="Il n'entre pas dans votre patrimoine : seuls les virements comptent." />
                    <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                    <button class=BTN_GHOST disabled=move || busy.get() on:click=create><Busy busy=busy />"Ajouter"</button>
                </div>
            </Show>
        </div>
    }
}

fn movement_look(kind: MovementKindDto) -> (IconName, &'static str, &'static str) {
    match kind {
        MovementKindDto::Expense => (IconName::Receipt, "text-primary-soft", "Dépense"),
        MovementKindDto::Income => (IconName::Coins, "text-gain", "Revenu ponctuel"),
        MovementKindDto::Charge => (IconName::Calendar, "text-muted", "Charge fixe"),
        MovementKindDto::TransferIn => (IconName::ArrowIn, "text-gain", "Virement reçu"),
        MovementKindDto::TransferOut => {
            (IconName::ArrowOut, "text-secondary-light", "Virement émis")
        }
    }
}

/// Fil des mouvements comptés depuis le dernier solde saisi, du plus récent au plus ancien.
#[component]
pub fn MovementFeed(
    movements: Vec<MovementDto>,
    #[prop(into)] empty_body: String,
    #[prop(into)] empty_action: String,
    on_empty_action: Callback<()>,
) -> impl IntoView {
    view! {
        <section class="flex flex-col gap-2">
            <h2 class=SECTION_TITLE>"Depuis le dernier solde saisi"</h2>
            {if movements.is_empty() {
                view! {
                    <EmptyState title="Aucun mouvement depuis" body=empty_body action=empty_action on_action=on_empty_action />
                }.into_any()
            } else {
                view! {
                    <Card class="flex flex-col p-2">
                        {movements.into_iter().map(|movement| {
                            let (icon, tone, fallback) = movement_look(movement.kind);
                            let direction = match movement.kind {
                                MovementKindDto::TransferIn => "de ",
                                _ => "vers ",
                            };
                            let counterpart = movement.counterpart.map(|name| format!(" · {direction}{name}"));
                            view! {
                                <div class="flex min-h-14 items-center gap-3 px-2">
                                    <span class=format!("flex size-9 shrink-0 items-center justify-center rounded-full bg-surface-raised {tone}")>
                                        <Icon name=icon class="size-5" />
                                    </span>
                                    <span class="flex min-w-0 flex-1 flex-col">
                                        <span class="truncate text-sm">{movement.label.unwrap_or_else(|| fallback.to_string())}</span>
                                        <span class="truncate text-xs text-muted">{format::date(movement.date)} {counterpart}</span>
                                    </span>
                                    <Amount value=movement.amount signed=true colored=true />
                                </div>
                            }
                        }).collect_view()}
                    </Card>
                }.into_any()
            }}
        </section>
    }
}

/// Virements à venir d'un compte (récurrents ou ponctuels datés après aujourd'hui), modifiables,
/// et bouton « Nouveau virement » pré-rempli avec ce compte.
#[component]
pub fn ScheduledTransfers(
    account_id: String,
    /// Le compte est d'abord alimenté (livret, placement) : il est proposé à l'arrivée.
    incoming: bool,
    new_open: RwSignal<bool>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let endpoint = EndpointDto::Account(account_id);
    let transfers: Loaded<Vec<TransferDto>> = RwSignal::new(None);
    let for_load = endpoint.clone();
    load(transfers, move || api::list_transfers(Some(for_load)));
    let editing = RwSignal::new(None::<TransferDto>);
    let (from, to) = if incoming {
        (None, Some(endpoint.clone()))
    } else {
        (Some(endpoint.clone()), None)
    };
    let endpoint = StoredValue::new(endpoint);
    let today = today();

    view! {
        <section class="flex flex-col gap-2">
            <div class="flex items-center justify-between">
                <h2 class=SECTION_TITLE>"Virements programmés"</h2>
                <button class=BTN_GHOST on:click=move |_| new_open.set(true)>
                    <Icon name=IconName::Transfer class="size-5" />
                    "Nouveau virement"
                </button>
            </div>
            {move || {
                let upcoming: Vec<TransferDto> = transfers
                    .get()
                    .and_then(Result::ok)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|t| !matches!(t.schedule, ScheduleDto::Once { date } if date <= today))
                    .collect();
                if upcoming.is_empty() {
                    return view! {
                        <p class="px-1 text-sm text-muted">"Aucun virement à venir. Programmez votre épargne du mois une fois pour toutes."</p>
                    }.into_any();
                }
                view! {
                    <Card class="flex flex-col p-2">
                        {upcoming.into_iter().map(|transfer| {
                            let incoming = endpoint.with_value(|e| &transfer.to == e);
                            let amount = if incoming { transfer.received } else { -transfer.sent };
                            let counterpart = if incoming {
                                format!("de {}", transfer.from_name)
                            } else {
                                format!("vers {}", transfer.to_name)
                            };
                            let bonus = (transfer.received != transfer.sent).then(|| {
                                format!("envoyé {} · reçu {}", format::amount(transfer.sent), format::amount(transfer.received))
                            });
                            let open_it = transfer.clone();
                            view! {
                                <button
                                    class="flex min-h-14 items-center gap-3 rounded-xl px-2 py-2 text-left active:bg-surface-raised"
                                    on:click=move |_| editing.set(Some(open_it.clone()))
                                >
                                    <span class="flex size-9 shrink-0 items-center justify-center rounded-full bg-surface-raised text-primary-soft">
                                        <Icon name=IconName::Transfer class="size-5" />
                                    </span>
                                    <span class="flex min-w-0 flex-1 flex-col">
                                        <span class="truncate text-sm">{transfer.label.clone().unwrap_or_else(|| if incoming { format!("De {}", transfer.from_name) } else { format!("Vers {}", transfer.to_name) })}</span>
                                        <span class="truncate text-xs text-muted">
                                            {schedule_label(&transfer.schedule)}
                                            {transfer.label.is_some().then(|| format!(" · {counterpart}"))}
                                        </span>
                                        {bonus.map(|b| view! { <span class="truncate text-xs text-muted">{b}</span> })}
                                    </span>
                                    <Amount value=amount signed=true colored=true />
                                </button>
                            }
                        }).collect_view()}
                    </Card>
                }.into_any()
            }}
        </section>
        <TransferSheet open=new_open from=from to=to on_saved=on_saved />
        {move || editing.get().map(|transfer| {
            let open = RwSignal::new(true);
            Effect::new(move |_| if !open.get() { editing.set(None) });
            view! { <TransferSheet open=open existing=Some(transfer) on_saved=on_saved /> }
        })}
    }
}

/// Comptes hors patrimoine (compte joint, banque d'un crédit…) : renommer, supprimer s'ils ne
/// servent à aucun virement.
#[component]
pub fn ExternalAccounts(
    /// Incrémenté à chaque virement enregistré : un compte créé ou utilisé depuis la feuille de
    /// virement apparaît aussitôt.
    refresh: RwSignal<u32>,
) -> impl IntoView {
    let externals: Loaded<Vec<ExternalAccountDto>> = RwSignal::new(None);
    let reload = Callback::new(move |_: ()| load(externals, api::list_external_accounts));
    Effect::new(move |_| {
        refresh.track();
        reload.run(());
    });
    let editing = RwSignal::new(None::<ExternalAccountDto>);
    let adding = RwSignal::new(false);
    view! {
        <section class="flex flex-col gap-2">
            <div class="flex items-center justify-between">
                <h2 class=SECTION_TITLE>"Hors patrimoine"</h2>
                <button class=BTN_GHOST on:click=move |_| adding.set(true)>"Ajouter"</button>
            </div>
            {move || {
                let list = externals.get().and_then(Result::ok).unwrap_or_default();
                if list.is_empty() {
                    return view! {
                        <EmptyState
                            title="Aucun compte hors patrimoine"
                            body="Compte joint, banque d'un crédit : ils n'entrent pas dans votre patrimoine, mais vos virements vers eux comptent dans le budget."
                            action="Ajouter un compte hors patrimoine"
                            on_action=Callback::new(move |_| adding.set(true))
                        />
                    }.into_any();
                }
                view! {
                    <Card class="flex flex-col p-1">
                        {list.into_iter().map(|external| {
                            let open_it = external.clone();
                            view! {
                                <button
                                    class="flex min-h-14 items-center justify-between gap-3 rounded-xl px-3 text-left active:bg-surface-raised"
                                    on:click=move |_| editing.set(Some(open_it.clone()))
                                >
                                    <span class="truncate">{external.name}</span>
                                    <span class="text-xs text-muted">{if external.used { "utilisé" } else { "aucun virement" }}</span>
                                </button>
                            }
                        }).collect_view()}
                    </Card>
                }.into_any()
            }}
        </section>
        <BottomSheet open=adding title="Compte hors patrimoine">
            <ExternalForm existing=None open=adding on_saved=reload />
        </BottomSheet>
        {move || editing.get().map(|external| {
            let open = RwSignal::new(true);
            Effect::new(move |_| if !open.get() { editing.set(None) });
            let external = StoredValue::new(external);
            view! {
                <BottomSheet open=open title="Compte hors patrimoine">
                    <ExternalForm existing=Some(external.get_value()) open=open on_saved=reload />
                </BottomSheet>
            }
        })}
    }
}

#[component]
fn ExternalForm(
    existing: Option<ExternalAccountDto>,
    open: RwSignal<bool>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let name = RwSignal::new(
        existing
            .as_ref()
            .map(|e| e.name.clone())
            .unwrap_or_default(),
    );
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let used = existing.as_ref().is_some_and(|e| e.used);
    let id = StoredValue::new(existing.map(|e| e.id));
    let done = move |message: &'static str| {
        toast::success(message);
        open.set(false);
        on_saved.run(());
    };
    let submit = move |_| match id.get_value() {
        Some(id) => run_action(
            busy,
            error,
            api::rename_external_account(RenameInput {
                id,
                name: name.get_untracked(),
            }),
            move |()| done("Compte renommé"),
        ),
        None => run_action(
            busy,
            error,
            api::create_external_account(NameInput {
                name: name.get_untracked(),
            }),
            move |_: String| done("Compte hors patrimoine ajouté"),
        ),
    };
    let remove = move |_| {
        let Some(id) = id.get_value() else { return };
        run_action(
            busy,
            error,
            api::delete_external_account(IdInput { id }),
            move |()| done("Compte supprimé"),
        );
    };
    view! {
        <div class="flex flex-col gap-4">
            <Field label="Nom" value=name placeholder="Compte joint" />
            <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
            <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit><Busy busy=busy />"Enregistrer"</button>
            {id.get_value().is_some().then(|| if used {
                view! { <p class="text-xs text-muted">"Utilisé par au moins un virement : supprimez d'abord ces virements pour pouvoir le supprimer."</p> }.into_any()
            } else {
                view! { <button class=BTN_DANGER disabled=move || busy.get() on:click=remove><Busy busy=busy />"Supprimer"</button> }.into_any()
            })}
        </div>
    }
}
