mod list;

use crate::config::Config;
use crate::error::ChrootManagerError;
use crate::tui::list::list;
use cursive::views::*;
use cursive::Cursive;

pub fn run(config: &Config) -> Result<(), ChrootManagerError> {
    let mut siv = cursive::default();

    siv.add_layer(
        Dialog::around(
            LinearLayout::horizontal()
                .child(list(config)?)
                .child(DummyView)
                .child(LinearLayout::vertical().child(Button::new("Quit", Cursive::quit))),
        )
        .title("Select a profile"),
    );

    siv.run();
    Ok(())
}
