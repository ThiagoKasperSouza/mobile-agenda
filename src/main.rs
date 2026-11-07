use agenda_core::{Event, NewEvent};
use js_sys::{Date, Promise, Reflect};
use leptos::prelude::*;
use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{ScrollBehavior, ScrollIntoViewOptions, ScrollLogicalPosition};

#[wasm_bindgen(inline_js = r#"
export async function invokeTauri(command, args) {
  const invoke = window.__TAURI__?.core?.invoke;
  if (!invoke) {
    throw new Error("A API do Tauri não está disponível. Abra o app pelo Tauri.");
  }
  return invoke(command, args);
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = invokeTauri)]
    fn invoke_tauri(command: &str, args: JsValue) -> Promise;
}

async fn invoke<T: DeserializeOwned>(command: &str, args: JsValue) -> Result<T, String> {
    let response = JsFuture::from(invoke_tauri(command, args))
        .await
        .map_err(js_error)?;
    serde_wasm_bindgen::from_value(response).map_err(|error| error.to_string())
}

fn js_error(error: JsValue) -> String {
    error
        .as_string()
        .or_else(|| {
            Reflect::get(&error, &JsValue::from_str("message"))
                .ok()
                .and_then(|message| message.as_string())
        })
        .unwrap_or_else(|| "Ocorreu um erro ao comunicar com o aplicativo.".to_string())
}

#[derive(Serialize)]
struct DeleteEventArgs<'a> {
    id: &'a str,
}

#[derive(Serialize)]
struct CreateEventArgs {
    input: NewEvent,
}

fn today_key() -> String {
    let date = Date::new_0();
    format!(
        "{:04}-{:02}-{:02}",
        date.get_full_year(),
        date.get_month() + 1,
        date.get_date()
    )
}

fn date_parts(key: &str) -> Option<(u32, u32, u32)> {
    let mut parts = key.split('-');
    Some((
        parts.next()?.parse().ok()?,
        parts.next()?.parse::<u32>().ok()?.checked_sub(1)?,
        parts.next()?.parse().ok()?,
    ))
}

fn month_start_key(key: &str) -> String {
    date_parts(key)
        .map(|(year, month, _)| format!("{year:04}-{:02}-01", month + 1))
        .unwrap_or_else(|| key.to_string())
}

fn days_in_month(year: u32, month: u32) -> u32 {
    Date::new_with_year_month_day(year, month as i32 + 1, 0).get_date()
}

fn shift_month(year: u32, month: u32, day: u32, offset: i32) -> (String, String) {
    let month_date = Date::new_with_year_month_day(year, month as i32 + offset, 1);
    let target_year = month_date.get_full_year();
    let target_month = month_date.get_month();
    let selected_date = Date::new_with_year_month_day(
        target_year,
        target_month as i32,
        day.min(days_in_month(target_year, target_month)) as i32,
    );
    (
        format!("{target_year:04}-{:02}-01", target_month + 1),
        format!(
            "{:04}-{:02}-{:02}",
            selected_date.get_full_year(),
            selected_date.get_month() + 1,
            selected_date.get_date()
        ),
    )
}

fn month_days(year: u32, month: u32) -> Vec<(String, String, String)> {
    let mut days = Vec::with_capacity(days_in_month(year, month) as usize);
    for day_number in 1..=days_in_month(year, month) {
        let day = Date::new_with_year_month_day(year, month as i32, day_number as i32);
        let key = format!(
            "{:04}-{:02}-{:02}",
            day.get_full_year(),
            day.get_month() + 1,
            day.get_date()
        );
        let weekday = match day.get_day() {
            0 => "Dom",
            1 => "Seg",
            2 => "Ter",
            3 => "Qua",
            4 => "Qui",
            5 => "Sex",
            _ => "Sáb",
        };
        days.push((key, weekday.to_string(), day.get_date().to_string()));
    }
    days
}

fn month_name(month: u32) -> &'static str {
    const MONTHS: [&str; 12] = [
        "Janeiro",
        "Fevereiro",
        "Março",
        "Abril",
        "Maio",
        "Junho",
        "Julho",
        "Agosto",
        "Setembro",
        "Outubro",
        "Novembro",
        "Dezembro",
    ];
    MONTHS[month as usize]
}

fn format_time(timestamp: i64) -> String {
    let date = Date::new(&JsValue::from_f64(timestamp as f64 * 1000.0));
    format!("{:02}:{:02}", date.get_hours(), date.get_minutes())
}

fn default_datetime(day: &str) -> String {
    if day == today_key() {
        let date = Date::new_0();
        date.set_seconds(0);
        date.set_minutes(0);
        date.set_hours(date.get_hours() + 1);
        return format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}",
            date.get_full_year(),
            date.get_month() + 1,
            date.get_date(),
            date.get_hours(),
            date.get_minutes()
        );
    }

    format!("{day}T09:00")
}

#[component]
fn App() -> impl IntoView {
    let (events, set_events) = signal(Vec::<Event>::new());
    let initial_day = today_key();
    let (selected_day, set_selected_day) = signal(initial_day.clone());
    let (calendar_day, set_calendar_day) = signal(initial_day.clone());
    let (calendar_month, set_calendar_month) = signal(month_start_key(&initial_day));
    let (selected_tag, set_selected_tag) = signal(String::new());
    let (show_form, set_show_form) = signal(false);
    let (show_calendar, set_show_calendar) = signal(false);
    let (error_message, set_error_message) = signal(String::new());
    let (loading, set_loading) = signal(true);
    let title_ref = NodeRef::<leptos::html::Input>::new();
    let description_ref = NodeRef::<leptos::html::Textarea>::new();
    let start_ref = NodeRef::<leptos::html::Input>::new();
    let reminder_ref = NodeRef::<leptos::html::Select>::new();
    let tags_ref = NodeRef::<leptos::html::Input>::new();
    let day_strip_ref = NodeRef::<leptos::html::Nav>::new();

    {
        let day_strip_ref = day_strip_ref;
        Effect::new(move |_| {
            let selected = selected_day.get();
            if let Some(day_strip) = day_strip_ref.get() {
                match day_strip.query_selector(&format!("[data-day='{selected}']")) {
                    Ok(Some(day)) => {
                        let options = ScrollIntoViewOptions::new();
                        options.set_behavior(ScrollBehavior::Smooth);
                        options.set_block(ScrollLogicalPosition::Nearest);
                        options.set_inline(ScrollLogicalPosition::Center);
                        day.scroll_into_view_with_scroll_into_view_options(&options);
                    }
                    Ok(None) => {}
                    Err(error) => web_sys::console::error_1(&error),
                }
            }
        });
    }

    {
        let set_events = set_events;
        let set_error_message = set_error_message;
        let set_loading = set_loading;
        spawn_local(async move {
            match invoke::<Vec<Event>>("list_events", JsValue::UNDEFINED).await {
                Ok(mut loaded) => {
                    loaded.sort_by_key(|event| event.starts_at);
                    set_events.set(loaded);
                }
                Err(error) => set_error_message.set(error),
            }
            set_loading.set(false);
        });
    }

    let save_event = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let title = title_ref
            .get()
            .map(|input| input.value())
            .unwrap_or_default();
        let description = description_ref
            .get()
            .map(|input| input.value())
            .unwrap_or_default();
        let start_value = start_ref
            .get()
            .map(|input| input.value())
            .unwrap_or_default();
        let reminder_minutes = reminder_ref
            .get()
            .and_then(|input| input.value().parse::<u32>().ok())
            .unwrap_or(10);
        let tags = tags_ref
            .get()
            .map(|input| input.value())
            .unwrap_or_default();
        let starts_at = Date::parse(&start_value);

        if title.trim().is_empty() || !starts_at.is_finite() {
            set_error_message.set("Preencha o título e a data do evento.".to_string());
            return;
        }

        let input = NewEvent {
            title: title.trim().to_string(),
            description: description.trim().to_string(),
            starts_at: (starts_at / 1000.0) as i64,
            reminder_minutes,
            tags: tags
                .split(',')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(str::to_string)
                .collect(),
        };
        let args = match serde_wasm_bindgen::to_value(&CreateEventArgs { input }) {
            Ok(args) => args,
            Err(error) => {
                set_error_message.set(error.to_string());
                return;
            }
        };

        let set_events = set_events;
        let set_error_message = set_error_message;
        let set_show_form = set_show_form;
        spawn_local(async move {
            match invoke::<Event>("create_event", args).await {
                Ok(event) => {
                    set_events.update(|events| {
                        events.push(event);
                        events.sort_by_key(|event| event.starts_at);
                    });
                    set_error_message.set(String::new());
                    set_show_form.set(false);
                }
                Err(error) => set_error_message.set(error),
            }
        });
    };

    let upcoming = move || {
        let now = (Date::new_0().get_time() / 1000.0) as i64;
        let mut found: Vec<Event> = events
            .get()
            .into_iter()
            .filter(|event| {
                event.reminder_minutes > 0
                    && event.starts_at - i64::from(event.reminder_minutes) * 60 >= now
            })
            .collect();
        found.sort_by_key(|event| event.starts_at - i64::from(event.reminder_minutes) * 60);
        found.truncate(4);
        found
    };

    view! {
        <main class="app-shell">
            <header class="topbar">
                <a class="brand" href="#" aria-label="Agenda, início">
                    <span class="brand-mark">"a."</span>
                    <span>"agenda"</span>
                </a>
                <div class="topbar-actions">
                    <span class="today-label">"Sua semana, com mais leveza"</span>
                    <button class="avatar-button" aria-label="Perfil">"T"</button>
                </div>
            </header>

            <div class="workspace">
                <aside class="sidebar">
                    <section class="sidebar-section">
                        <h2>"ATALHOS"</h2>
                        <button class="sidebar-link active" on:click=move |_| set_selected_tag.set(String::new())>
                            <span class="link-icon">"▦"</span>" Minha agenda"
                        </button>
                        <button class="sidebar-link" on:click=move |_| set_selected_day.set(today_key())>
                            <span class="link-icon">"◷"</span>" Hoje"
                        </button>
                    </section>
                    <section class="sidebar-section">
                        <div class="section-heading">
                            <h2>"ETIQUETAS"</h2>
                            <span class="tiny-count">{move || events.get().iter().flat_map(|event| event.tags.clone()).collect::<std::collections::HashSet<_>>().len()}</span>
                        </div>
                        <button class="tag-filter" class:chosen=move || selected_tag.get().is_empty()
                            on:click=move |_| set_selected_tag.set(String::new())>
                            <span class="tag-dot all-dot"></span>"Todas"
                        </button>
                        {move || {
                            let mut tags: Vec<String> = events.get().into_iter().flat_map(|event| event.tags).collect();
                            tags.sort_by_key(|tag| tag.to_lowercase());
                            tags.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
                            tags.into_iter().map(|tag| {
                                let chosen_filter = tag.clone();
                                let selected_filter = tag.clone();
                                view! {
                                    <button class="tag-filter" class:chosen=move || selected_tag.get().eq_ignore_ascii_case(&chosen_filter)
                                        on:click=move |_| set_selected_tag.set(selected_filter.clone())>
                                        <span class="tag-dot"></span>{tag}
                                    </button>
                                }
                            }).collect_view()
                        }}
                    </section>
                    <section class="sidebar-section reminders-section">
                        <div class="section-heading">
                            <h2>"LEMBRETES"</h2>
                            <span class="tiny-count">{move || upcoming().len()}</span>
                        </div>
                        {move || {
                            let items = upcoming();
                            if items.is_empty() {
                                view! { <p class="empty-note">"Seus próximos lembretes aparecem aqui."</p> }.into_any()
                            } else {
                                items.into_iter().map(|event| view! {
                                    <button class="reminder-item" on:click=move |_| set_selected_day.set(date_key(event.starts_at))>
                                        <span class="reminder-time">{format_time(event.starts_at - event.reminder_minutes as i64 * 60)}</span>
                                        <span class="reminder-title">{event.title}</span>
                                    </button>
                                }).collect_view().into_any()
                            }
                        }}
                    </section>
                    <footer class="sidebar-footer">
                        <span class="status-dot"></span>"Lembretes neste aparelho"
                    </footer>
                </aside>

                <section class="main-content">
                    <div class="page-heading">
                        <div>
                            <p class="eyebrow">"SEU TEMPO, DO SEU JEITO"</p>
                            <h1>
                                    <select
                                        class="month-select"
                                        aria-label="Selecionar mês"
                                        prop:value=move || date_parts(&selected_day.get())
                                            .map(|(_, month, _)| month.to_string())
                                            .unwrap_or_default()
                                        on:change=move |ev| {
                                            if let (Some((year, _, day_number)), Ok(month)) = (
                                                date_parts(&selected_day.get()),
                                                event_target_value(&ev).parse::<u32>(),
                                            ) {
                                                if month < 12 {
                                                    let day = Date::new_with_year_month_day(
                                                        year,
                                                        month as i32,
                                                        day_number.min(days_in_month(year, month)) as i32,
                                                    );
                                                    set_selected_day.set(format!(
                                                        "{:04}-{:02}-{:02}",
                                                        day.get_full_year(),
                                                        day.get_month() + 1,
                                                        day.get_date()
                                                    ));
                                                }
                                            }
                                        }
                                    >
                                        {(0..12).map(|month| view! {
                                            <option value=month.to_string()>{month_name(month)}</option>
                                        }).collect_view()}
                                    </select>
                                    <span class="heading-caret" aria-hidden="true">"⌄"</span>
                                    <span class="heading-year">
                                        {move || date_parts(&selected_day.get())
                                            .map(|(year, _, _)| year.to_string())
                                            .unwrap_or_default()}
                                    </span>
                            </h1>
                        </div>
                        <button
                            type="button"
                            class="calendar-trigger"
                            aria-label="Abrir calendário mensal"
                            title="Abrir calendário mensal"
                            on:click=move |_| {
                                let day = selected_day.get();
                                set_calendar_day.set(day.clone());
                                set_calendar_month.set(month_start_key(&day));
                                set_show_calendar.set(true);
                            }
                        >
                            <svg viewBox="0 0 24 24" aria-hidden="true">
                                <rect x="3.5" y="5" width="17" height="16" rx="2.5"></rect>
                                <path d="M7.5 3.5v3M16.5 3.5v3M3.5 9.5h17M8 13h.01M12 13h.01M16 13h.01M8 17h.01M12 17h.01"></path>
                            </svg>
                        </button>
                    </div>

                    <nav node_ref=day_strip_ref class="month-days" aria-label="Dias do mês">
                        {move || {
                            date_parts(&selected_day.get())
                                .map(|(year, month, _)| {
                                    month_days(year, month)
                                        .into_iter()
                                        .map(|(key, weekday, number)| {
                                            let is_today = key == today_key();
                                            let day_key = key.clone();
                                            view! {
                                                <button class="day-button" class:selected=move || selected_day.get() == day_key
                                                    class:today=is_today
                                                    data-day=key.clone()
                                                    on:click=move |_| set_selected_day.set(key.clone())>
                                                    <span class="day-name">{weekday}</span>
                                                    <span class="day-number">{number}</span>
                                                </button>
                                            }
                                        })
                                        .collect_view()
                                        .into_any()
                                })
                                .unwrap_or_else(|| ().into_any())
                        }}
                    </nav>

                    <Show when=move || show_calendar.get()>
                        <div class="calendar-backdrop">
                            <section
                                class="calendar-modal"
                                role="dialog"
                                aria-modal="true"
                                aria-labelledby="calendar-modal-title"
                            >
                                <header class="calendar-modal-header">
                                    <div>
                                        <p class="eyebrow">"AGENDA MENSAL"</p>
                                        <h2 id="calendar-modal-title">
                                            {move || {
                                                date_parts(&calendar_month.get())
                                                    .map(|(year, month, _)| format!("{} {year}", month_name(month)))
                                                    .unwrap_or_default()
                                            }}
                                        </h2>
                                    </div>
                                    <button
                                        type="button"
                                        class="calendar-close"
                                        aria-label="Fechar calendário"
                                        on:click=move |_| set_show_calendar.set(false)
                                    >
                                        "×"
                                    </button>
                                </header>

                                <div class="calendar-month-controls">
                                    <button
                                        type="button"
                                        aria-label="Mês anterior"
                                        on:click=move |_| {
                                            if let (Some((year, month, _)), Some((_, _, day))) = (
                                                date_parts(&calendar_month.get()),
                                                date_parts(&calendar_day.get()),
                                            ) {
                                                let (month_key, day_key) = shift_month(year, month, day, -1);
                                                set_calendar_month.set(month_key);
                                                set_calendar_day.set(day_key);
                                            }
                                        }
                                    >
                                        "‹"
                                    </button>
                                    <span>
                                        {move || {
                                            date_parts(&calendar_day.get())
                                                .map(|(year, month, day)| {
                                                    format!("{day} de {} de {year}", month_name(month))
                                                })
                                                .unwrap_or_default()
                                        }}
                                    </span>
                                    <button
                                        type="button"
                                        aria-label="Próximo mês"
                                        on:click=move |_| {
                                            if let (Some((year, month, _)), Some((_, _, day))) = (
                                                date_parts(&calendar_month.get()),
                                                date_parts(&calendar_day.get()),
                                            ) {
                                                let (month_key, day_key) = shift_month(year, month, day, 1);
                                                set_calendar_month.set(month_key);
                                                set_calendar_day.set(day_key);
                                            }
                                        }
                                    >
                                        "›"
                                    </button>
                                </div>

                                <div class="calendar-weekdays" aria-hidden="true">
                                    {["Dom", "Seg", "Ter", "Qua", "Qui", "Sex", "Sáb"]
                                        .into_iter()
                                        .map(|weekday| view! { <span>{weekday}</span> })
                                        .collect_view()}
                                </div>

                                <div class="calendar-grid" role="grid" aria-label="Dias do mês">
                                    {move || {
                                        if let Some((year, month, _)) = date_parts(&calendar_month.get()) {
                                            let first_weekday =
                                                Date::new_with_year_month_day(year, month as i32, 1).get_day();
                                            let event_counts = events.get().into_iter().fold(
                                                std::collections::HashMap::<String, usize>::new(),
                                                |mut counts, event| {
                                                    *counts.entry(date_key(event.starts_at)).or_default() += 1;
                                                    counts
                                                },
                                            );
                                            let mut cells = Vec::new();
                                            for _ in 0..first_weekday {
                                                cells.push(view! {
                                                    <span class="calendar-day-spacer" aria-hidden="true"></span>
                                                }.into_any());
                                            }
                                            for (key, weekday, number) in month_days(year, month) {
                                                let event_count = event_counts.get(&key).copied().unwrap_or_default();
                                                let day_key = key.clone();
                                                let selected_key = key.clone();
                                                let is_today = key == today_key();
                                                let accessible_label = if event_count == 1 {
                                                    format!("{weekday} {number}, 1 evento")
                                                } else {
                                                    format!("{weekday} {number}, {event_count} eventos")
                                                };
                                                cells.push(view! {
                                                    <button
                                                        type="button"
                                                        class="calendar-day"
                                                        class:selected=move || calendar_day.get() == selected_key
                                                        class:today=is_today
                                                        role="gridcell"
                                                        aria-label=accessible_label
                                                        aria-pressed=move || calendar_day.get() == day_key
                                                        on:click=move |_| set_calendar_day.set(key.clone())
                                                    >
                                                        <span>{number}</span>
                                                        {if event_count > 0 {
                                                            view! { <span class="calendar-event-marker" aria-hidden="true"></span> }.into_any()
                                                        } else {
                                                            ().into_any()
                                                        }}
                                                    </button>
                                                }.into_any());
                                            }
                                            cells.collect_view().into_any()
                                        } else {
                                            ().into_any()
                                        }
                                    }}
                                </div>

                                <section class="calendar-events" aria-live="polite">
                                    <h3>
                                        {move || {
                                            date_parts(&calendar_day.get())
                                                .map(|(year, month, day)| {
                                                    format!("{day} de {} de {year}", month_name(month))
                                                })
                                                .unwrap_or_default()
                                        }}
                                    </h3>
                                    {move || {
                                        let day = calendar_day.get();
                                        let day_events: Vec<Event> = events
                                            .get()
                                            .into_iter()
                                            .filter(|event| date_key(event.starts_at) == day)
                                            .collect();
                                        if day_events.is_empty() {
                                            view! {
                                                <p class="calendar-no-events">"Nenhum evento nesta data."</p>
                                            }.into_any()
                                        } else {
                                            day_events.into_iter().map(|event| view! {
                                                <div class="calendar-event-row">
                                                    <time>{format_time(event.starts_at)}</time>
                                                    <span>{event.title}</span>
                                                </div>
                                            }).collect_view().into_any()
                                        }
                                    }}
                                </section>

                                <footer class="calendar-modal-actions">
                                    <button
                                        type="button"
                                        class="secondary-button calendar-select-button"
                                        on:click=move |_| {
                                            set_selected_day.set(calendar_day.get());
                                            set_show_calendar.set(false);
                                        }
                                    >
                                        "Selecionar data"
                                    </button>
                                    <button
                                        type="button"
                                        class="primary-button"
                                        on:click=move |_| {
                                            set_selected_day.set(calendar_day.get());
                                            set_show_calendar.set(false);
                                            set_show_form.set(true);
                                        }
                                    >
                                        "+ Novo evento"
                                    </button>
                                </footer>
                            </section>
                        </div>
                    </Show>

                    <div class="agenda-heading">
                        <div>
                            <h2>"Sua agenda"</h2>
                            <p>{move || format!("{} eventos no dia", events.get().iter().filter(|event| date_key(event.starts_at) == selected_day.get()).count())}</p>
                        </div>
                        <button class="primary-button agenda-add" on:click=move |_| set_show_form.set(true)>
                            <span class="plus">"+" </span>" Novo"
                        </button>
                    </div>

                    <Show when=move || show_form.get()>
                        <form class="event-form" on:submit=save_event>
                            <div class="form-heading">
                                <div><p class="eyebrow">"NOVO COMPROMISSO"</p><h2>"O que vem por aí?"</h2></div>
                                <button type="button" class="icon-button" aria-label="Fechar" on:click=move |_| set_show_form.set(false)>"×"</button>
                            </div>
                            <label>"Nome do evento"<input node_ref=title_ref maxlength="120" placeholder="Ex.: Reunião de planejamento" required /></label>
                            <label>"Descrição"<textarea node_ref=description_ref maxlength="1000" placeholder="Adicione alguns detalhes..."></textarea></label>
                            <div class="form-row">
                                <label>"Data e hora"<input node_ref=start_ref type="datetime-local"
                                    prop:value=move || default_datetime(&selected_day.get()) required /></label>
                                <label>"Avisar antes"<select node_ref=reminder_ref prop:value="10">
                                    <option value="5">"5 minutos"</option>
                                    <option value="10">"10 minutos"</option>
                                    <option value="15">"15 minutos"</option>
                                    <option value="30">"30 minutos"</option>
                                    <option value="60">"1 hora"</option>
                                </select></label>
                            </div>
                            <label>"Etiquetas"<input node_ref=tags_ref placeholder="Trabalho, Pessoal (separe por vírgula)" /></label>
                            <div class="form-actions">
                                <button type="button" class="secondary-button" on:click=move |_| set_show_form.set(false)>"Cancelar"</button>
                                <button type="submit" class="primary-button">"Salvar evento"</button>
                            </div>
                        </form>
                    </Show>

                    <Show when=move || !error_message.get().is_empty()>
                        <div class="error-banner" role="alert">
                            <span>{move || error_message.get()}</span>
                            <button class="icon-button" aria-label="Fechar aviso" on:click=move |_| set_error_message.set(String::new())>"×"</button>
                        </div>
                    </Show>

                    <div class="event-list">
                        <Show when=move || loading.get()>
                            <div class="empty-state"><span class="spinner"></span><p>"Carregando sua agenda..."</p></div>
                        </Show>
                        {move || {
                            let day = selected_day.get();
                            let tag = selected_tag.get();
                            let filtered: Vec<Event> = events.get().into_iter().filter(|event| {
                                date_key(event.starts_at) == day
                                    && (tag.is_empty() || event.tags.iter().any(|value| value.eq_ignore_ascii_case(&tag)))
                            }).collect();
                            if !loading.get() && filtered.is_empty() {
                                view! {
                                    <div class="empty-state">
                                        <span class="empty-illustration">"✳"</span>
                                        <h3>"Dia livre, mente tranquila."</h3>
                                        <p>"Adicione um evento para começar a organizar seu dia."</p>
                                        <button class="text-button" on:click=move |_| set_show_form.set(true)>"+" " Criar evento"</button>
                                    </div>
                                }.into_any()
                            } else {
                                filtered.into_iter().enumerate().map(|(index, event)| {
                                    let event_id = event.id.clone();
                                    let title = event.title.clone();
                                    let color = match index % 4 { 0 => "lilac", 1 => "sand", 2 => "mint", _ => "peach" };
                                    view! {
                                        <article class=format!("event-card {color}")>
                                            <div class="event-card-top">
                                                <div class="event-tags">
                                                    {event.tags.iter().map(|tag| view! { <span class="event-tag">{tag.clone()}</span> }).collect_view()}
                                                </div>
                                                <span class="event-time">{format_time(event.starts_at)}</span>
                                            </div>
                                            <div class="event-card-main">
                                                <div>
                                                    <h3>{title}</h3>
                                                    <p>{if event.description.is_empty() { "Sem descrição".to_string() } else { event.description.clone() }}</p>
                                                </div>
                                                <button class="delete-button" aria-label="Excluir evento"
                                                    on:click=move |_| {
                                                        let id = event_id.clone();
                                                        let set_events = set_events;
                                                        let set_error_message = set_error_message;
                                                        let args = match serde_wasm_bindgen::to_value(&DeleteEventArgs { id: &id }) {
                                                            Ok(args) => args,
                                                            Err(error) => {
                                                                set_error_message.set(error.to_string());
                                                                return;
                                                            }
                                                        };
                                                        spawn_local(async move {
                                                            match invoke::<()>("delete_event", args).await {
                                                                Ok(()) => set_events.update(|events| events.retain(|event| event.id != id)),
                                                                Err(error) => set_error_message.set(error),
                                                            }
                                                        });
                                                    }>"×"</button>
                                            </div>
                                        </article>
                                    }
                                }).collect_view().into_any()
                            }
                        }}
                    </div>

                    <div class="mobile-reminders">
                        <div class="section-heading"><h2>"PRÓXIMOS LEMBRETES"</h2><span class="tiny-count">{move || upcoming().len()}</span></div>
                        {move || upcoming().into_iter().map(|event| view! {
                            <div class="mobile-reminder-row">
                                <span class="reminder-clock">{format_time(event.starts_at - event.reminder_minutes as i64 * 60)}</span>
                                <span>{event.title}</span>
                            </div>
                        }).collect_view()}
                    </div>
                </section>
            </div>
        </main>
    }
}

fn date_key(timestamp: i64) -> String {
    let date = Date::new(&JsValue::from_f64(timestamp as f64 * 1000.0));
    format!(
        "{:04}-{:02}-{:02}",
        date.get_full_year(),
        date.get_month() + 1,
        date.get_date()
    )
}

fn main() {
    mount_to_body(App);
}
