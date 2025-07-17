use crate::chroot::ChrootUnit;
use crate::config::Config;
use crate::error::ChrootManagerError;
use cursive::traits::*;
use cursive::views::*;
use cursive::{Cursive, CursiveExt};
use std::process::Command;

pub fn run(config: &Config) -> Result<(), ChrootManagerError> {
    let mut siv = cursive::default();

    siv.add_layer(
        Dialog::around(
            LinearLayout::horizontal()
                .child(list(config)?)
                .child(DummyView)
                .child(Button::new("Quit", Cursive::quit)),
        )
        .title("Select a profile"),
    );

    siv.run();
    Ok(())
}

fn list(
    config: &Config,
) -> Result<ResizedView<NamedView<SelectView<ChrootUnit>>>, ChrootManagerError> {
    let units = ChrootUnit::find_units(config)?;

    let mut select = SelectView::new();

    for unit in units {
        select.add_item(unit.name.as_str(), unit.clone());
    }

    let unit_selected = select
        .on_submit(|s, unit| {
            let current = unit.clone();
            s.add_layer(
                Dialog::text(format!(
                    "Profile: {}\nPath: {}\n",
                    unit.stage3_profile,
                    unit.chroot_path.display()
                ))
                .title(format!("{}", unit.name))
                .button("Enter", move |s| {
                    //let dump = s.dump();
                    s.quit();
                    Command::new("reset").status().ok();
                    current
                        .mount_filesystems()
                        .unwrap()
                        .enter_chroot_interactive()
                        .unwrap();
                    //s.restore(dump);
                    s.pop_layer();
                    s.run();
                })
                .button("Cancel", |s| {
                    s.pop_layer();
                }),
            );
        })
        .with_name("select")
        .full_screen();
    Ok(unit_selected)
}
