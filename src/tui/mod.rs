mod create;
mod list;

use crate::chroot::ChrootUnit;
use crate::config::Config;
use crate::error::ChrootManagerError;
use crate::tui::create::select_arch;
use cursive::traits::{Nameable, Resizable};
use cursive::views::*;
use cursive::Cursive;

pub fn run(config: &Config) -> Result<(), ChrootManagerError> {
    config.ensure_chroot_base_dir()?;
    let mut siv = cursive::default();

    let units = ChrootUnit::find_units(config)?;

    let mut select = SelectView::new();
    for unit in units {
        select.add_item(unit.name.as_str(), unit.clone());
    }

    siv.add_layer(
        Dialog::around(
            LinearLayout::horizontal()
                .child(
                    select
                        .on_submit(list::show_unit_info)
                        .with_name("select")
                        .min_width(15),
                )
                .child(DummyView)
                .child(
                    LinearLayout::vertical()
                        .child(Button::new("Create", select_arch))
                        .child(Button::new("Quit", Cursive::quit)),
                ),
        )
        .title("Select a profile"),
    );

    siv.run();
    Ok(())
}
