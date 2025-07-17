use crate::chroot::ChrootUnit;
use crate::config::Config;
use crate::error::ChrootManagerError;
use cursive::traits::*;
use cursive::views::*;
use cursive::{Cursive, CursiveExt};
use std::process::Command;

pub fn list(
    config: &Config,
) -> Result<ResizedView<NamedView<SelectView<ChrootUnit>>>, ChrootManagerError> {
    let units = ChrootUnit::find_units(config)?;

    let mut select = SelectView::new();

    for unit in units {
        select.add_item(unit.name.as_str(), unit.clone());
    }

    let unit_selected = select
        .on_submit(show_unit_info)
        .with_name("select")
        .min_width(15);

    Ok(unit_selected)
}

fn show_unit_info(s: &mut Cursive, unit: &ChrootUnit) {
    let current = unit.clone();
    s.add_layer(
        Dialog::text(format!(
            "Profile: {}\nPath: {}\n",
            unit.stage3_profile,
            unit.chroot_path.display()
        ))
        .title(unit.name.to_string())
        .button("Enter", move |s| enter_chroot(s, &current))
        .button("Cancel", |s| {
            s.pop_layer();
        }),
    )
}

fn enter_chroot(s: &mut Cursive, unit: &ChrootUnit) {
    s.quit();
    Command::new("reset").status().ok();
    unit.mount_filesystems()
        .unwrap()
        .enter_chroot_interactive()
        .unwrap();
    s.pop_layer();
    s.run();
}
