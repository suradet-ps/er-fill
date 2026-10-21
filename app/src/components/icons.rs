//! ER-Fill's monoline SVG icon set.
//!
//! A deliberately softer, clinical set: 24×24 viewBox, `currentColor`,
//! 1.8px stroke, round caps and joins. The glyphs sit on a common optical
//! grid so they stay crisp at 16px inside buttons and 24px in settings.
//! Icons are decorative: `aria-hidden` and never carry text alternatives;
//! nearby text always explains the state.

use leptos::prelude::*;

/// Generates one icon component: an `<svg>` shell with the given nodes.
macro_rules! icon {
    ($(#[$doc:meta])* $name:ident, $($nodes:tt)*) => {
        $(#[$doc])*
        #[component]
        pub fn $name(class: &'static str) -> impl IntoView {
            view! {
                <svg
                    class=class
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.8"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    aria-hidden="true"
                >
                    $($nodes)*
                </svg>
            }
        }
    };
}

icon!(
    /// Check - confirmations ("บันทึกแล้ว").
    IconCheck,
    <path d="M4.5 12.5 9.5 17.5 19.5 6.5" />
);

icon!(
    /// Gear - settings action.
    IconSettings,
    <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
    <circle cx="12" cy="12" r="3" />
);

icon!(
    /// Printer - print action.
    IconPrinter,
    <path d="M7 8V3.5h10V8" />
    <path d="M7 17H5.5A2.5 2.5 0 0 1 3 14.5v-4A2.5 2.5 0 0 1 5.5 8h13A2.5 2.5 0 0 1 21 10.5v4a2.5 2.5 0 0 1-2.5 2.5H17" />
    <rect x="7" y="14" width="10" height="7" rx="1.2" />
    <path d="M17.5 11.5h.01" />
);

icon!(
    /// Refresh - refetch action.
    IconRefresh,
    <path d="M20 8a8 8 0 0 0-13.7-2.3L4 8" />
    <path d="M4 3v5h5" />
    <path d="M4 16a8 8 0 0 0 13.7 2.3L20 16" />
    <path d="M20 21v-5h-5" />
);

icon!(
    /// Calendar - window panel.
    IconCalendar,
    <rect x="3" y="5" width="18" height="16" rx="3" />
    <path d="M8 3v4" />
    <path d="M16 3v4" />
    <path d="M3 10h18" />
    <path d="M8 14h.01M12 14h.01M16 14h.01M8 18h.01M12 18h.01M16 18h.01" />
);

icon!(
    /// Clock - the window band.
    IconClock,
    <circle cx="12" cy="12" r="9" />
    <path d="M12 7v5l3 2" />
);

icon!(
    /// List - summary section.
    IconList,
    <path d="M9 6h11" />
    <path d="M9 12h11" />
    <path d="M9 18h11" />
    <path d="M4.5 6h.01" />
    <path d="M4.5 12h.01" />
    <path d="M4.5 18h.01" />
);

icon!(
    /// Plus - add actions.
    IconPlus,
    <path d="M12 5v14" />
    <path d="M5 12h14" />
);

icon!(
    /// Trash - remove actions.
    IconTrash,
    <path d="M4 7h16" />
    <path d="M9 7V4.5h6V7" />
    <path d="M6.5 7l.8 12a2 2 0 0 0 2 1.9h5.4a2 2 0 0 0 2-1.9l.8-12" />
    <path d="M10 11v6M14 11v6" />
);

icon!(
    /// Chevron up - move item up.
    IconChevronUp,
    <path d="m18 15-6-6-6 6" />
);

icon!(
    /// Chevron down - move item down.
    IconChevronDown,
    <path d="m6 9 6 6 6-6" />
);

icon!(
    /// X - close actions.
    IconX,
    <path d="M18 6 6 18" />
    <path d="m6 6 12 12" />
);

icon!(
    /// Alert triangle - warning banner.
    IconAlertTriangle,
    <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3z" />
    <path d="M12 9v4" />
    <path d="M12 17h.01" />
);

icon!(
    /// Alert circle - error banner.
    IconAlertCircle,
    <circle cx="12" cy="12" r="10" />
    <path d="M12 8v4" />
    <path d="M12 16h.01" />
);

icon!(
    /// Search - drug picker.
    IconSearch,
    <circle cx="11" cy="11" r="7" />
    <path d="m20 20-4-4" />
);

icon!(
    /// Database - connection settings.
    IconDatabase,
    <ellipse cx="12" cy="5" rx="9" ry="3" />
    <path d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5" />
    <path d="M3 12c0 1.66 4.03 3 9 3s9-1.34 9-3" />
);

icon!(
    /// User - staff identity.
    IconUser,
    <circle cx="12" cy="8" r="3.5" />
    <path d="M5 20a7 7 0 0 1 14 0" />
);

icon!(
    /// Lock - password field and read-only guarantees.
    IconLock,
    <rect x="4" y="10" width="16" height="11" rx="3" />
    <path d="M8 10V7a4 4 0 0 1 8 0v3" />
    <path d="M12 14v3" />
);

icon!(
    /// Shield check - read-only / protected data.
    IconShieldCheck,
    <path d="M12 3 5 6v5c0 4.6 2.9 8.1 7 10 4.1-1.9 7-5.4 7-10V6Z" />
    <path d="m9 12 2 2 4-4" />
);

icon!(
    /// Arrow right - advance to the next step.
    IconArrowRight,
    <path d="M4 12h15" />
    <path d="m14 7 5 5-5 5" />
);

icon!(
    /// Log out - end the session.
    IconLogOut,
    <path d="M10 4H5a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h5" />
    <path d="m14 8 4 4-4 4" />
    <path d="M18 12H9" />
);

icon!(
    /// Pencil - edit actions.
    IconPencil,
    <path d="M4 20l4.4-1.1L19.7 7.6a2.1 2.1 0 0 0-3-3L5.4 15.9 4 20Z" />
    <path d="M14.5 6.5l3 3" />
    <path d="M4 20h5" />
);

icon!(
    /// Clipboard list - the requisition document.
    IconClipboardList,
    <rect x="5" y="5" width="14" height="16" rx="3" />
    <path d="M9 5V4a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v1" />
    <path d="M9 11h6" />
    <path d="M9 15h6" />
    <path d="M9 19h4" />
);

icon!(
    /// File text - the printed header block.
    IconFileText,
    <path d="M7 3h7l4 4v14H7z" />
    <path d="M14 3v5h4" />
    <path d="M10 13h5" />
    <path d="M10 17h5" />
);
