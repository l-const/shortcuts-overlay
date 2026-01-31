use crate::config::OverlayConfig;
use smithay_client_toolkit::shell::wlr_layer::Anchor;

pub(crate) fn to_anchor(anchor_str: Option<String>) -> Anchor {
    match anchor_str.as_deref() {
        Some("center") | Some("Center") | None => Anchor::empty(),
        Some("topleft") | Some("TopLeft") => Anchor::TOP | Anchor::LEFT,
        Some("topright") | Some("TopRight") => Anchor::TOP | Anchor::RIGHT,
        Some("bottomleft") | Some("BottomLeft") => Anchor::BOTTOM | Anchor::LEFT,
        Some("bottomright") | Some("BottomRight") => Anchor::BOTTOM | Anchor::RIGHT,
        Some("top") | Some("Top") => Anchor::TOP,
        Some("bottom") | Some("Bottom") => Anchor::BOTTOM,
        Some("left") | Some("Left") => Anchor::LEFT,
        Some("right") | Some("Right") => Anchor::RIGHT,
        Some(other) => {
            log::warn!("Unknown anchor value '{}', defaulting to center", other);
            Anchor::empty()
        }
    }
}

pub(crate) fn merge_cli_opts_config(
    config: &OverlayConfig,
    opts: &crate::args::Opt,
) -> OverlayConfig {
    let mut config = config.clone();

    if let Some(width) = opts.width {
        config.width = width;
    }

    if let Some(height) = opts.height {
        config.height = height;
    }

    if let Some(anchor) = &opts.anchor {
        config.anchor = anchor.clone();
    }

    config
}
