//! Compile the real thin preview adapter without a Tauri application build.
extern crate self as tauri;
#[derive(Clone)]
pub struct AppHandle;
pub trait Emitter {
    fn emit<S: serde::Serialize + Clone>(&self, _event: &str, _payload: S) -> Result<(), String> {
        Ok(())
    }
}
impl Emitter for AppHandle {}
#[allow(dead_code)]
#[path = "../../src-tauri/src/playback.rs"]
mod playback;
