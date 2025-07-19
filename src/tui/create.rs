use crate::chroot::ChrootUnit;
use crate::profile::amd64::Amd64Profile;
use crate::profile::arch::Arch;
use crate::profile::arm64::Arm64Profile;
use cursive::traits::*;
use cursive::views::*;
use cursive::Cursive;
use std::sync::Arc;
use strum::IntoEnumIterator;

pub fn select_arch(s: &mut Cursive) {
    let mut select = SelectView::new();

    for unit in Arch::iter() {
        select.add_item(unit.arch(), unit.clone());
    }
    s.add_layer(
        Dialog::around(
            LinearLayout::vertical().child(
                select
                    .on_submit(select_profile)
                    .with_name("arch")
                    .min_width(15),
            ),
        )
        .button("Cancel", |s| {
            s.pop_layer();
        })
        .title("Select your architecture"),
    )
}

fn select_profile(s: &mut Cursive, arch: &Arch) {
    let mut select = SelectView::<Arch>::new();
    match arch {
        Arch::Amd64(_) => {
            for profile in Amd64Profile::iter() {
                select.add_item(profile.to_string(), Arch::Amd64(profile));
            }
        }
        Arch::Arm64(_) => {
            for profile in Arm64Profile::iter() {
                select.add_item(profile.to_string(), Arch::Arm64(profile));
            }
        }
    }
    s.add_layer(
        Dialog::around(select.on_submit(choose_name))
            .button("Cancel", |s| {
                s.pop_layer();
            })
            .title("Select a profile"),
    );
}

fn choose_name(s: &mut Cursive, arch: &Arch) {
    // s.pop_layer();
    s.add_layer(
        Dialog::new()
            .title("Choose a chroot name")
            .padding_lrtb(1, 1, 1, 0)
            .content(EditView::new().with_name("chroot_name").fixed_width(30))
            .button("Ok", {
                let arch = arch.to_owned();
                move |s| get_name(s, &arch)
            })
            .button("Cancel", |s| {
                s.pop_layer();
            }),
    )
}

fn get_name(s: &mut Cursive, arch: &Arch) {
    let name = s
        .call_on_name("chroot_name", |view: &mut EditView| view.get_content())
        .unwrap();
    s.add_layer(
        LinearLayout::vertical().child(
            Dialog::text(format!("Chroot name: {name}\narch: {arch}"))
                .padding_lrtb(1, 1, 1, 0)
                .title("Valid your choices")
                .button("Ok", {
                    let arch = arch.to_owned();
                    move |s| create(s, &arch.clone(), name.clone())
                })
                .button("Cancel", |s| {
                    s.pop_layer();
                }),
        ),
    )
}

fn create(s: &mut Cursive, arch: &Arch, name: Arc<String>) {
    let arch = arch.to_owned();
    let chroot_unit = ChrootUnit::new(name.to_string(), Some(&arch)).unwrap();
    s.add_layer(TextView::new(format!("{chroot_unit:?}")));
}
