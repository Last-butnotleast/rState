use std::{cell::RefCell, error::Error, rc::Rc};

use gpui::{App, Application};

mod domain;
mod smoke;
mod view;

fn main() -> Result<(), Box<dyn Error>> {
    let smoke = std::env::args().any(|argument| argument == "--smoke");
    let result = Rc::new(RefCell::new(None));
    let output = result.clone();
    Application::new().run(move |cx: &mut App| {
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        match view::open_documents(cx) {
            Ok(windows) => {
                cx.activate(true);
                if smoke {
                    cx.spawn(async move |cx| {
                        let outcome = smoke::run(windows, cx).await;
                        if let Err(error) = &outcome {
                            eprintln!("Document smoke failed: {error}");
                        }
                        output.replace(Some(outcome));
                        if let Err(error) = cx.update(|cx| cx.quit()) {
                            eprintln!("Unable to quit document smoke: {error}");
                        }
                    })
                    .detach();
                }
            }
            Err(error) => {
                output.replace(Some(Err(error)));
                cx.quit();
            }
        }
    });
    let outcome = result.take();
    match outcome {
        Some(Err(error)) => Err(std::io::Error::other(error).into()),
        None if smoke => Err(std::io::Error::other("Smoke run exited before completion").into()),
        None | Some(Ok(())) => Ok(()),
    }
}
