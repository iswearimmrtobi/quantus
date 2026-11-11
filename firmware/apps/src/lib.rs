//! Apps on the handheld (firmware/docs/apps.md): scripts in Rhai, built in or from the SD card,
//! run inside the hui's frame as an [`Applet`](quantus_ui::applet::Applet). The hui never sees
//! the interpreter.

pub mod api;
pub mod applet;
pub mod header;
pub mod script;

pub use applet::ScriptApplet;
pub use header::{info, Header, API};
pub use script::{Callback, Context, Error, Output, Request, Script, Value, Yield};
