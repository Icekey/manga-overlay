use crate::event::event::Event::UpdateShortcut;
use crate::event::event::{Event, ShortcutEvent, emit_event};
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
            hotkeys: vec![
                Shortcut::new(hotkey1, ShortcutEvent::ToggleDecorations),
                Shortcut::new(hotkey2, ShortcutEvent::ToggleMousePassthrough),
            ],
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Shortcut {
    pub hotkey: HotKey,
    pub hotkey_string: String,
    pub event: Option<ShortcutEvent>,
}

impl ShortcutEvent {
    pub fn get_label(&self) -> &'static str {
        match self {
            ShortcutEvent::ToggleDecorations => "Toggle Decorations",
            ShortcutEvent::ToggleMousePassthrough => "Toggle MousePassthrough",
        }
    }
}

impl Shortcut {
    pub fn new(hotkey: HotKey, event: ShortcutEvent) -> Self {
        Self {
            hotkey,
            hotkey_string: hotkey.into_string(),
            event: Some(event),
        }
    }

    pub fn refresh_string(&mut self) {
        self.hotkey_string = self.hotkey.into_string();
    }

    pub fn get_label(&self) -> &'static str {
        self.event.as_ref().unwrap().get_label()
    }
}

impl ShortcutManager {
    pub fn init(&mut self) {
        for x in self.hotkeys.iter_mut() {
            x.refresh_string();
            self.hotkey_manager.register(x.hotkey).unwrap();
        }
    }

    pub fn unregister(&mut self) {
        for x in self.hotkeys.iter_mut() {
            self.hotkey_manager.unregister(x.hotkey).unwrap();
        }
    }

    pub fn check_events(&self) {
        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv()
            && event.state == HotKeyState::Pressed
        {
            for x in self.hotkeys.iter() {
                if event.id == x.hotkey.id {
                    emit_event(Event::from(x.event.clone().unwrap()));
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
                    ui.label(format!("{} {}", x.get_label(), x.hotkey.into_string(),));
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
