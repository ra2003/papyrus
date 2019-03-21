#[macro_use]
extern crate papyrus;

use azul::prelude::*;
use linefeed::memory::MemoryTerminal;
use papyrus::widgets::pad::*;
use std::sync::{Arc, Mutex};

type TypedPadState = PadState<String>;

struct MyApp {
    repl_term: TypedPadState,
}

impl std::borrow::BorrowMut<TypedPadState> for MyApp {
    fn borrow_mut(&mut self) -> &mut TypedPadState {
        &mut self.repl_term
    }
}

impl std::borrow::Borrow<TypedPadState> for MyApp {
    fn borrow(&self) -> &TypedPadState {
        &self.repl_term
    }
}

impl Layout for MyApp {
    fn layout(&self, info: LayoutInfo<Self>) -> Dom<Self> {
        Dom::div()
            .with_child(ReplTerminal::new(info.window, &self.repl_term, &self).dom(&self.repl_term))
    }
}

fn main() {
    let term = MemoryTerminal::new();

    let repl = repl_with_term!(term.clone(), String);

    let mut app = App::new(
        MyApp {
            repl_term: PadState::new(repl, Arc::new(Mutex::new(12345.to_string()))),
        },
        AppConfig {
            enable_logging: Some(LevelFilter::Error),
            log_file_path: Some("debug.log".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    let window = if cfg!(debug_assertions) {
        app.create_hot_reload_window(
            WindowCreateOptions::default(),
            css::hot_reload_override_native(
                "styles/test.css",
                std::time::Duration::from_millis(1000),
            ),
        )
        .unwrap()

    // Window::new(WindowCreateOptions::default(), css::native()).unwrap()
    } else {
        app.create_window(WindowCreateOptions::default(), css::native())
            .unwrap()
    };

    app.run(window).unwrap();
}
