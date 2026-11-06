//! The ESP-IDF environment, and the tour's wallpaper (`src/bin/tour.rs`): the ESP32 has no RAM
//! for a decoded picture, so it is converted here into raw RGB565, full and dimmed, which the
//! board reads from flash. The picture is `QUANTUS_TOUR_WALLPAPER`, else the first BMP in the
//! simulator's `sd/wallpapers`, else a gradient.

use std::path::{Path, PathBuf};

use quantus_ui::render::{HEIGHT, WIDTH};
use quantus_ui::wallpaper::Wallpaper;

fn main() {
    embuild::espidf::sysenv::output();

    let folder = Path::new("../sim/sd/wallpapers");
    println!("cargo:rerun-if-env-changed=QUANTUS_TOUR_WALLPAPER");
    println!("cargo:rerun-if-changed={}", folder.display());
    let path = std::env::var_os("QUANTUS_TOUR_WALLPAPER").map(PathBuf::from).or_else(|| first_bmp(folder));
    let (name, wallpaper) = match &path {
        Some(path) => {
            println!("cargo:rerun-if-changed={}", path.display());
            let bytes = std::fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            let wallpaper = Wallpaper::from_bmp(&bytes)
                .unwrap_or_else(|| panic!("{}: not a 480×320 BMP", path.display()));
            let name = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("Bild").to_string();
            (name, wallpaper)
        }
        None => ("Verlauf".to_string(), gradient()),
    };
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    std::fs::write(out.join("tour_full.raw"), wallpaper.full.to_raw()).expect("write");
    std::fs::write(out.join("tour_dim.raw"), wallpaper.dim.to_raw()).expect("write");
    std::fs::write(out.join("tour_name.txt"), name).expect("write");
}

/// The first BMP in `folder` by name, as the simulator lists them.
fn first_bmp(folder: &Path) -> Option<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(folder)
        .ok()?
        .filter_map(|file| Some(file.ok()?.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("bmp")))
        .collect();
    files.sort();
    files.into_iter().next()
}

fn gradient() -> Wallpaper {
    use quantus_ui::render::colors::rgb;
    let pixels = (0..HEIGHT)
        .flat_map(|y| (0..WIDTH).map(move |x| rgb((x * 255 / WIDTH) as u8, 80, (y * 255 / HEIGHT) as u8)))
        .collect();
    Wallpaper::new(pixels).expect("screen-sized")
}
