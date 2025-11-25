use crate::event::event::{
    is_minimized, toggle_decorations, toggle_mouse_passthrough, update_mouse_passthrough,
};
use crate::ui::update_queue::enqueue_update;
use egui::CollapsingHeader;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::str::FromStr;
use strum::{EnumIter, IntoEnumIterator};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ShortcutManager {
    #[serde(skip)]
    pub hotkey_manager: GlobalHotKeyManager,
    pub hotkeys: Vec<Shortcut>,
}

impl Default for ShortcutManager {
    fn default() -> Self {
        let hotkeys = ShortcutEvent::iter().map(Shortcut::new).collect();
        Self {
            hotkey_manager: GlobalHotKeyManager::new().unwrap(),
            hotkeys,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Shortcut {
    pub hotkey: HotKey,
    #[serde(skip)]
    pub hotkey_string: String,
    pub event: ShortcutEvent,
}

#[derive(serde::Deserialize, serde::Serialize, EnumIter, PartialEq, Debug, Clone)]
pub enum ShortcutEvent {
    ToggleDecorations,
    ToggleMousePassthrough,
    ToggleMinimized,
    QuickAreaPickMode,
}

impl ShortcutEvent {
    pub fn get_label(&self) -> &'static str {
        match self {
            ShortcutEvent::ToggleDecorations => "Toggle Decorations",
            ShortcutEvent::ToggleMousePassthrough => "Toggle MousePassthrough",
            ShortcutEvent::ToggleMinimized => "Toggle Minimized",
            ShortcutEvent::QuickAreaPickMode => "Quick Area Pick Mode",
        }
    }

    pub fn default_hotkey(&self) -> HotKey {
        match self {
            ShortcutEvent::ToggleDecorations => HotKey::new(Some(Modifiers::SHIFT), Code::KeyD),
            ShortcutEvent::ToggleMousePassthrough => {
                HotKey::new(Some(Modifiers::SHIFT), Code::KeyS)
            }
            ShortcutEvent::ToggleMinimized => HotKey::new(Some(Modifiers::SHIFT), Code::KeyM),
            ShortcutEvent::QuickAreaPickMode => HotKey::new(Some(Modifiers::SHIFT), Code::KeyA),
        }
    }

    pub fn emit_event(&self) {
        match self {
            ShortcutEvent::ToggleDecorations => toggle_decorations(),
            ShortcutEvent::ToggleMousePassthrough => toggle_mouse_passthrough(),
            ShortcutEvent::ToggleMinimized => {
                enqueue_update(|ctx, _| {
                    let is_minimized = is_minimized(ctx);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(!is_minimized));
                });
            }
            ShortcutEvent::QuickAreaPickMode => {
                enqueue_update(|_, app| {
                    update_mouse_passthrough(false);
                    app.settings.quick_area_pick_mode = !app.settings.quick_area_pick_mode;
                });
            }
        }
    }
}

impl Shortcut {
    pub fn new(event: ShortcutEvent) -> Self {
        let hotkey = event.default_hotkey();
        Self {
            hotkey,
            hotkey_string: hotkey.into_string(),
            event,
        }
    }

    pub fn refresh_string(&mut self) {
        self.hotkey_string = self.hotkey.into_string();
    }

    pub fn get_label(&self) -> &'static str {
        self.event.get_label()
    }
}

impl ShortcutManager {
    pub fn init(&mut self) {
        ShortcutEvent::iter().for_each(|event| {
            if self.hotkeys.iter().all(|x| x.event != event) {
                self.hotkeys.push(Shortcut::new(event));
            }
        });

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
                    x.event.emit_event();
                    return;
                }
            }
        }
    }

    pub fn hotkey_exists(&self, hotkey: HotKey) -> bool {
        self.hotkeys.iter().any(|x| x.hotkey.id == hotkey.id)
    }

    pub fn update_hotkey(&mut self, event: ShortcutEvent, hotkey: HotKey) {
        if self.hotkey_exists(hotkey) {
            return;
        }

        let _ = self.hotkey_manager.unregister(hotkey);

        if let Some(shortcut) = self.hotkeys.iter_mut().find(|x| x.event == event) {
            shortcut.hotkey = hotkey;
        } else {
            let mut new_shortcut = Shortcut::new(event);
            new_shortcut.hotkey = hotkey;
            self.hotkeys.push(new_shortcut);
        }

        let _ = self.hotkey_manager.register(hotkey);
    }

    pub fn show_config(&mut self, ui: &mut egui::Ui) {
        CollapsingHeader::new("Hotkey Config").show(ui, |ui| {
            for x in self.hotkeys.iter_mut() {
                ui.horizontal(|ui| {
                    ui.label(format!("{} {}", x.get_label(), x.hotkey.into_string(),));
                    let response = ui.text_edit_singleline(&mut x.hotkey_string);
                    if response.changed() {
                        if let Ok(hotkey) = HotKey::from_str(&x.hotkey_string) {
                            let event = x.event.clone();
                            enqueue_update(move |_, app| {
                                app.settings.shortcut.update_hotkey(event, hotkey)
                            });
                        }
                    };
                    if response.lost_focus() {
                        x.hotkey_string = x.hotkey.into_string();
                    }
                });
            }
        });
    }
}
