use agenda_core::{AgendaData, Event, NewEvent};
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_notification::{NotificationExt, PermissionState, Schedule};
use time::OffsetDateTime;

struct AgendaStore {
    path: PathBuf,
    data: Mutex<AgendaData>,
}

fn persist(path: &Path, data: &AgendaData) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "O caminho dos dados da agenda é inválido.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Não foi possível criar a pasta de dados: {error}"))?;

    let temporary_path = path.with_extension("json.tmp");
    let contents = serde_json::to_vec_pretty(data)
        .map_err(|error| format!("Não foi possível serializar a agenda: {error}"))?;
    fs::write(&temporary_path, contents)
        .map_err(|error| format!("Não foi possível salvar a agenda: {error}"))?;
    fs::rename(&temporary_path, path)
        .map_err(|error| format!("Não foi possível concluir o salvamento da agenda: {error}"))
}

fn schedule_reminder(app: &AppHandle, event: &Event) -> Result<(), String> {
    if event.reminder_minutes == 0 {
        return Ok(());
    }

    #[cfg(mobile)]
    {
        let notification = app.notification();
        let permission = notification
            .permission_state()
            .map_err(|error| format!("Não foi possível verificar a permissão para notificações: {error}"))?;
        let permission = if permission == PermissionState::Granted {
            permission
        } else {
            notification.request_permission().map_err(|error| {
                format!("Não foi possível solicitar permissão para notificações: {error}")
            })?
        };
        if permission != PermissionState::Granted {
            return Err(
                "Permita notificações nas configurações do Android para criar este lembrete."
                    .to_string(),
            );
        }

        let reminder_at = event.starts_at - i64::from(event.reminder_minutes) * 60;
        let date = OffsetDateTime::from_unix_timestamp(reminder_at)
            .map_err(|error| format!("Horário do lembrete inválido: {error}"))?;
        app.notification()
            .builder()
            .id(event.notification_id)
            .title(format!("Lembrete: {}", event.title))
            .body(if event.description.is_empty() {
                "Seu evento está prestes a começar.".to_string()
            } else {
                event.description.clone()
            })
            .schedule(Schedule::At {
                date,
                repeating: false,
                allow_while_idle: true,
            })
            .show()
            .map_err(|error| format!("Não foi possível agendar a notificação: {error}"))?;
    }

    #[cfg(not(mobile))]
    let _ = app;

    Ok(())
}

fn cancel_reminder(app: &AppHandle, event: &Event) -> Result<(), String> {
    if event.reminder_minutes == 0 {
        return Ok(());
    }

    #[cfg(mobile)]
    app.notification()
        .cancel(vec![event.notification_id])
        .map_err(|error| format!("Não foi possível cancelar o lembrete: {error}"))?;

    #[cfg(not(mobile))]
    let _ = app;

    Ok(())
}

#[tauri::command]
fn list_events(store: State<'_, AgendaStore>) -> Result<Vec<Event>, String> {
    let data = store
        .data
        .lock()
        .map_err(|_| "O armazenamento da agenda está indisponível.".to_string())?;
    Ok(data.events.clone())
}

#[tauri::command]
fn create_event(
    app: AppHandle,
    store: State<'_, AgendaStore>,
    input: NewEvent,
) -> Result<Event, String> {
    let title = input.title.trim();
    if title.is_empty() || title.len() > 120 {
        return Err("O título deve ter entre 1 e 120 caracteres.".to_string());
    }
    if input.description.len() > 1000 {
        return Err("A descrição deve ter no máximo 1000 caracteres.".to_string());
    }
    if input.reminder_minutes > 10_080 {
        return Err(
            "O lembrete não pode ser definido para mais de 7 dias antes do evento.".to_string(),
        );
    }
    if input.tags.len() > 10 {
        return Err("Cada evento pode ter no máximo 10 etiquetas.".to_string());
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("Não foi possível ler o relógio do aparelho: {error}"))?
        .as_secs() as i64;
    let reminder_at = input.starts_at - i64::from(input.reminder_minutes) * 60;
    if input.starts_at <= now || (input.reminder_minutes > 0 && reminder_at <= now) {
        return Err("Escolha um horário futuro que permita receber o lembrete.".to_string());
    }

    let tags = input
        .tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<_>>();
    if tags.iter().any(|tag| tag.len() > 32) {
        return Err("Cada etiqueta deve ter no máximo 32 caracteres.".to_string());
    }

    let mut data = store
        .data
        .lock()
        .map_err(|_| "O armazenamento da agenda está indisponível.".to_string())?;
    let id = uuid::Uuid::new_v4();
    let bytes = id.as_bytes();
    let mut notification_id =
        (u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) & 0x7fff_ffff) as i32;
    while notification_id == 0
        || data
            .events
            .iter()
            .any(|event| event.notification_id == notification_id)
    {
        notification_id = notification_id.wrapping_add(1).max(1);
    }

    let event = Event {
        id: id.to_string(),
        title: title.to_string(),
        description: input.description.trim().to_string(),
        starts_at: input.starts_at,
        reminder_minutes: input.reminder_minutes,
        tags,
        notification_id,
    };
    schedule_reminder(&app, &event)?;

    let previous = data.clone();
    data.events.push(event.clone());
    data.events.sort_by_key(|item| item.starts_at);
    if let Err(error) = persist(&store.path, &data) {
        *data = previous;
        if let Err(cleanup_error) = cancel_reminder(&app, &event) {
            return Err(format!(
                "{error} Além disso, não foi possível cancelar o lembrete recém-criado: {cleanup_error}"
            ));
        }
        return Err(error);
    }
    Ok(event)
}

#[tauri::command]
fn delete_event(app: AppHandle, store: State<'_, AgendaStore>, id: String) -> Result<(), String> {
    let mut data = store
        .data
        .lock()
        .map_err(|_| "O armazenamento da agenda está indisponível.".to_string())?;
    let index = data
        .events
        .iter()
        .position(|event| event.id == id)
        .ok_or_else(|| "Este evento não existe mais.".to_string())?;
    let event = data.events[index].clone();
    cancel_reminder(&app, &event)?;
    let previous = data.clone();
    data.events.remove(index);
    if let Err(error) = persist(&store.path, &data) {
        *data = previous;
        if let Err(restore_error) = schedule_reminder(&app, &event) {
            return Err(format!(
                "{error} Além disso, não foi possível restaurar o lembrete: {restore_error}"
            ));
        }
        return Err(error);
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            fs::create_dir_all(&directory)?;
            let path = directory.join("agenda.json");
            let data = match fs::read(&path) {
                Ok(contents) => serde_json::from_slice(&contents)?,
                Err(error) if error.kind() == ErrorKind::NotFound => AgendaData::default(),
                Err(error) => return Err(error.into()),
            };
            app.manage(AgendaStore {
                path,
                data: Mutex::new(data),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_events,
            create_event,
            delete_event
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar a aplicação Tauri");
}
