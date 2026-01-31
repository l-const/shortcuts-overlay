use clap::Parser;

/// CLI options for the shortcuts overlay.

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Opt {
    /// Overlay width in pixels (client size). If omitted, default 1200 is used.
    #[arg(long)]
    pub width: Option<u32>,

    /// Overlay height in pixels (client size). If omitted, default 800 is used.
    #[arg(long)]
    pub height: Option<u32>,
    /// Overlay anchor position. If omitted, default is center.
    /// Available: center, topleft, topright, bottomleft, bottomright,
    ///                               top, bottom, left, right
    #[arg(long)]
    pub anchor: Option<String>,
}
