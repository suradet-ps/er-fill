//! ER-Refill frontend library (Leptos 0.8, CSR).
//!
//! Document-first layout (DESIGN.md "Structure"): the top bar, the command
//! bar (the only control surface), and the stage - a paper-styled canvas
//! where the A4 requisition sheet previews exactly what prints. The whole
//! frontend talks to the backend only through [`api`].
//!
//! The crate is split lib + bin so components are wasm-testable: `main.rs`
//! is only `er_refill_app::run()`.

pub mod api;
pub mod components;
pub mod settings_io;
pub mod state;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use er_refill_models::AppSettings;
use leptos::prelude::*;

use components::command_bar::CommandBar;
use components::document_sheet::{DocumentSheet, SheetBanner};
use components::empty_state::{CatalogEmpty, NoDataHint};
use components::login_screen::{LoginScreen, StartupSplash};
use components::requisition_dialogs::RequisitionDialogs;
use components::settings_modal::SettingsModal;
use components::top_bar::{BrandMarkDefs, TopBar};
use state::{AppState, Banner};

/// How often the frontend polls the backend's live health state.
const HEALTH_POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Self-scheduling poll loop cell: `None` until the trigger is installed.
type PollLoop = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

/// Mounts the app into the document body.
pub fn run() {
    leptos::mount::mount_to_body(|| view! { <App /> });
}

/// Top bar + command bar + stage (DESIGN.md "Structure").
#[component]
fn App() -> impl IntoView {
    let state = AppState::new();

    // Initial load: connection status + settings + session. The operator
    // then chooses one of the four ranges before the first fetch.
    let load_state = state.clone();
    leptos::task::spawn_local(async move {
        load_initial_state(load_state).await;
    });

    // Poll the backend's live reachability - the status dot must reflect a
    // dead database within seconds, not "the config file exists".
    let poll_state = state.clone();
    let poll_health = Rc::new(move || {
        let poll_state = poll_state.clone();
        leptos::task::spawn_local(async move {
            if let Ok(health) = api::connection_health().await {
                poll_state.health.set(health);
            }
        });
    });
    // Self-scheduling loop: `poll_loop` holds the trigger, which schedules
    // itself again via the timer (one pending timer at a time, no leak).
    let poll_loop: PollLoop = Rc::new(RefCell::new(None));
    let date_state = state.clone();
    let trigger = {
        let poll_loop = Rc::clone(&poll_loop);
        Rc::new(move || {
            date_state.refresh_date_for_today();
            poll_health();
            if let Some(next) = poll_loop.borrow().as_ref().map(Rc::clone) {
                let _ = set_timeout_with_handle(move || next(), HEALTH_POLL_INTERVAL);
            }
        })
    };
    *poll_loop.borrow_mut() = Some(Rc::clone(&trigger) as Rc<dyn Fn()>);
    trigger();

    let stage_state = state.clone();
    let shell_state = state.clone();
    view! {
        <div class="app">
            <BrandMarkDefs />
            {move || {
                let state = shell_state.clone();
                if !state.auth_checked.get() {
                    view! { <StartupSplash /> }.into_any()
                } else if state.session.get().is_none() {
                    view! { <LoginScreen state=state.clone() /> }.into_any()
                } else {
                    let stage = stage_state.clone();
                    view! {
                        <TopBar state=state.clone() />
                        <CommandBar state=state.clone() />
                        <main class="stage">
                            <SheetBanner state=stage.clone() />
                            {move || {
                                if stage.catalog_is_empty() {
                                    view! { <CatalogEmpty state=stage.clone() /> }.into_any()
                                } else {
                                    match stage.sections.get() {
                                        Some(sections) => {
                                            view! {
                                                <DocumentSheet
                                                    state=stage.clone()
                                                    sections=sections
                                                />
                                            }
                                                .into_any()
                                        }
                                        None => {
                                            view! { <NoDataHint state=stage.clone() /> }.into_any()
                                        }
                                    }
                                }
                            }}
                        </main>
                    }
                        .into_any()
                }
            }}
            <RequisitionDialogs state=state.clone() />
            <SettingsModal state=state.clone() />
        </div>
    }
}

/// First load: status, settings, connection probe, and session. After this
/// resolves the UI shows either the login gate or the main document.
async fn load_initial_state(state: AppState) {
    let configured = api::connection_status().await;
    state.configured.set(configured);

    match api::load_settings().await {
        Ok(settings) => state.settings.set(Some(settings)),
        Err(err) => {
            state.settings.set(Some(AppSettings::default()));
            state.banner.set(Some(Banner::error(err.message)));
        }
    }

    if configured {
        match api::connection_summary().await {
            Ok(summary) => state.connection_summary.set(summary),
            Err(err) => state.connection_error.set(Some(err.message)),
        }
        match api::reconnect().await {
            Ok(_) => {
                state.health.set(state::ConnectionHealth::Connected);
                state.connection_error.set(None);
            }
            Err(err) => {
                state.health.set(state::ConnectionHealth::Disconnected);
                state.connection_error.set(Some(err.message));
            }
        }
    } else {
        state.connection_summary.set(None);
        state.connection_error.set(None);
        state.health.set(state::ConnectionHealth::Unconfigured);
    }

    if let Ok(Some(user)) = api::current_session().await {
        state.finish_login(user);
    }
    state.auth_checked.set(true);
}
