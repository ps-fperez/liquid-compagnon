//! Classes Tailwind partagées (boutons, champs). Des constantes plutôt que des composants :
//! les boutons restent de simples `<button>` / `<a>` HTML.

/// Action principale : une seule par écran.
pub const BTN_PRIMARY: &str = "inline-flex min-h-12 items-center justify-center gap-2 rounded-xl bg-primary px-5 font-medium text-bg transition active:scale-[0.98] disabled:opacity-50";
pub const BTN_SECONDARY: &str = "inline-flex min-h-12 items-center justify-center gap-2 rounded-xl border border-line bg-surface-raised px-5 font-medium text-ink transition active:scale-[0.98] disabled:opacity-50";
pub const BTN_GHOST: &str = "inline-flex min-h-11 items-center justify-center gap-2 rounded-xl px-4 font-medium text-secondary-light transition active:bg-surface-raised";
pub const BTN_DANGER: &str = "inline-flex min-h-12 items-center justify-center gap-2 rounded-xl border border-loss/40 px-5 font-medium text-loss transition active:scale-[0.98]";
/// Bouton icône carré (44 px).
pub const BTN_ICON: &str = "inline-flex size-11 items-center justify-center rounded-xl text-muted transition active:bg-surface-raised";

pub const INPUT: &str = "min-h-12 w-full rounded-xl border border-line bg-surface px-4 text-ink placeholder:text-muted/60 outline-none transition focus:border-primary focus:shadow-glow-focus";
pub const LABEL: &str = "text-sm text-muted";
pub const SECTION_TITLE: &str = "px-1 text-xs font-medium uppercase tracking-wider text-muted";
