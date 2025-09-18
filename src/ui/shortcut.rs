use crate::event::event::Event::UpdateShortcut;
use crate::event::event::{Event, emit_event};
use egui::CollapsingHeader;
use global_hotkey::hotkey::{HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::str::FromStr;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ShortcutManager {
    #[serde(skip)]
    pub hotkey_manager: GlobalHotKeyManager,
    pub hotkeys: Vec<Shortcut>,
}

impl Default for ShortcutManager {
    fn default() -> Self {
        let hotkey1 = HotKey::new(Some(Modifiers::SHIFT), global_hotkey::hotkey::Code::KeyD);
        let hotkey2 = HotKey::new(Some(Modifiers::SHIFT), global_hotkey::hotkey::Code::KeyS);
        Self {
            hotkey_manager: GlobalHotKeyManager::new().unwrap(),
            hotkeys: vec![Shortcut::new(hotkey1), Shortcut::new(hotkey2)],
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Shortcut {
    pub hotkey: HotKey,
    #[serde(skip)]
    pub hotkey_string: String,
    #[serde(skip)]
    pub label: Option<String>,
    #[serde(skip)]
    pub event: Option<Event>,
}

impl Shortcut {
    pub fn new(hotkey: HotKey) -> Self {
        Self {
            hotkey,
            hotkey_string: hotkey.into_string(),
            label: None,
            event: None,
        }
    }

    pub fn refresh_string(&mut self) {
        self.hotkey_string = self.hotkey.into_string();
    }
}

impl ShortcutManager {
    pub fn init(&mut self) {
        let labels = vec!["Toggle Decorations", "Toggle MousePassthrough"];
        let events = vec![Event::ToggleDecorations, Event::ToggleMousePassthrough];
        for ((x, label), event) in self.hotkeys.iter_mut().zip(labels).zip(events) {
            x.label.replace(label.to_string());
            x.event.replace(event);
            x.refresh_string();
            self.hotkey_manager.register(x.hotkey).unwrap();
        }
    }

    pub fn check_events(&self) {
        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv()
            && event.state == HotKeyState::Pressed
        {
            for x in self.hotkeys.iter() {
                if event.id == x.hotkey.id {
                    emit_event(x.event.clone().unwrap());
                    return;
                }
            }
        }
    }

    pub fn hotkey_exists(&self, hotkey: HotKey) -> bool {
        self.hotkeys.iter().any(|x| x.hotkey.id == hotkey.id)
    }

    pub fn update_hotkey(&mut self, index: usize, hotkey: HotKey) {
        if !self.hotkey_exists(hotkey) {
            let _ = self.hotkey_manager.unregister(hotkey);
            self.hotkeys[index].hotkey = hotkey;
            let _ = self.hotkey_manager.register(hotkey);
        }
    }

    pub fn show_config(&mut self, ui: &mut egui::Ui) {
        CollapsingHeader::new("Hotkey Config").show(ui, |ui| {
            for (i, x) in self.hotkeys.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "{} {}",
                        x.label.as_ref().unwrap(),
                        x.hotkey.into_string(),
                    ));
                    let response = ui.text_edit_singleline(&mut x.hotkey_string);
                    if response.changed() {
                        if let Ok(hotkey) = HotKey::from_str(&x.hotkey_string) {
                            emit_event(UpdateShortcut(i, hotkey))
                        }
                    };
                });
            }
        });
    }
}
