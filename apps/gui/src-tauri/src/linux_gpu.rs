//! GPU drawing for the web view on Linux with NVIDIA's driver under Wayland.
//!
//! WebKitGTK before 2.54 turns hardware acceleration off there unless it is forced, so the whole
//! window is painted by one CPU thread (docs/PERFORMANCE.md, *The page was drawn on the CPU*).
//! Forcing it needs NVIDIA's explicit sync off as well: with it on, the window is refused with
//! a Wayland protocol error and the app exits.

/// WebKitGTK reads this before 2.54; later versions accelerate without it and ignore it.
const FORCE_GPU_RENDERER: &str = "WEBKIT_FORCE_DMABUF_RENDERER";
const DISABLE_EXPLICIT_SYNC: &str = "__NV_DISABLE_EXPLICIT_SYNC";
/// Set by a person who chose how the web view draws; the app leaves that choice alone.
const DRAWING_CHOSEN_BY_USER: [&str; 3] = [
    "WEBKIT_DISABLE_COMPOSITING_MODE",
    "WEBKIT_DISABLE_DMABUF_RENDERER",
    FORCE_GPU_RENDERER,
];
const ON: &str = "1";
const NVIDIA_KERNEL_MODULE: &str = "/sys/module/nvidia";
const X11_BACKEND: &str = "x11";

/// What the app adds to its own environment so the web view draws on the GPU. Nothing when the
/// machine is not NVIDIA on Wayland, when the person opted out (`<APP>_NO_GPU_FORCE=1`) or chose
/// the drawing mode themselves, or when explicit sync is set to stay on: forcing the renderer
/// then would stop the app from starting, and a slow window is better than none.
fn overrides(
    nvidia: bool,
    opt_out: &str,
    get: impl Fn(&str) -> Option<String>,
) -> Vec<(&'static str, &'static str)> {
    let set = |name: &str| get(name).filter(|value| !value.is_empty());
    let wayland = set("WAYLAND_DISPLAY").is_some()
        && !set("GDK_BACKEND").is_some_and(|backend| backend.starts_with(X11_BACKEND));
    let opted_out = set(opt_out).is_some_and(|value| value == ON);
    let chosen = DRAWING_CHOSEN_BY_USER
        .iter()
        .any(|name| set(name).is_some());
    let sync = set(DISABLE_EXPLICIT_SYNC);
    let sync_stays_on = sync.as_deref().is_some_and(|value| value != ON);
    if !nvidia || !wayland || opted_out || chosen || sync_stays_on {
        return Vec::new();
    }
    let mut added = vec![(FORCE_GPU_RENDERER, ON)];
    if sync.is_none() {
        added.push((DISABLE_EXPLICIT_SYNC, ON));
    }
    added
}

/// Call first thing in `run`, before GTK and the web view read the environment.
pub fn prefer_gpu_drawing() {
    let opt_out = format!("{}_NO_GPU_FORCE", maitty_machines::APP_ENV);
    let nvidia = std::path::Path::new(NVIDIA_KERNEL_MODULE).exists();
    for (name, value) in overrides(nvidia, &opt_out, |name| std::env::var(name).ok()) {
        // SAFETY: called at the start of `run`, before any other thread exists to read or
        // write the environment.
        unsafe { std::env::set_var(name, value) };
    }
}

#[cfg(test)]
mod tests {
    use super::{DISABLE_EXPLICIT_SYNC, FORCE_GPU_RENDERER, overrides};

    const OPT_OUT: &str = "APP_NO_GPU_FORCE";

    fn with(environment: &[(&str, &str)], nvidia: bool) -> Vec<(&'static str, &'static str)> {
        overrides(nvidia, OPT_OUT, |name| {
            environment
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        })
    }

    const WAYLAND: (&str, &str) = ("WAYLAND_DISPLAY", "wayland-1");

    #[test]
    fn nvidia_on_wayland_gets_the_renderer_forced_and_explicit_sync_off() {
        assert_eq!(
            with(&[WAYLAND], true),
            [(FORCE_GPU_RENDERER, "1"), (DISABLE_EXPLICIT_SYNC, "1")]
        );
    }

    #[test]
    fn a_session_that_already_turned_explicit_sync_off_only_needs_the_renderer() {
        assert_eq!(
            with(&[WAYLAND, (DISABLE_EXPLICIT_SYNC, "1")], true),
            [(FORCE_GPU_RENDERER, "1")]
        );
    }

    #[test]
    fn explicit_sync_kept_on_is_never_combined_with_the_forced_renderer() {
        assert!(with(&[WAYLAND, (DISABLE_EXPLICIT_SYNC, "0")], true).is_empty());
    }

    #[test]
    fn other_graphics_cards_and_x11_are_left_alone() {
        assert!(with(&[WAYLAND], false).is_empty());
        assert!(with(&[], true).is_empty());
        assert!(with(&[WAYLAND, ("GDK_BACKEND", "x11")], true).is_empty());
        assert!(
            !with(&[WAYLAND, ("GDK_BACKEND", "wayland,x11,*")], true).is_empty(),
            "x11 as a fallback still runs on Wayland"
        );
    }

    #[test]
    fn the_person_can_opt_out_or_choose_the_drawing_mode_themselves() {
        assert!(with(&[WAYLAND, (OPT_OUT, "1")], true).is_empty());
        assert!(with(&[WAYLAND, ("WEBKIT_DISABLE_COMPOSITING_MODE", "1")], true).is_empty());
        assert!(with(&[WAYLAND, ("WEBKIT_DISABLE_DMABUF_RENDERER", "1")], true).is_empty());
        assert!(
            with(&[WAYLAND, (FORCE_GPU_RENDERER, "0")], true).is_empty(),
            "a renderer setting of their own is kept, and nothing is added around it"
        );
    }
}
