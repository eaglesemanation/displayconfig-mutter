use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List monitors
    List(ListArgs),
    /// Set config
    Set(SetArgs),
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// If specified - will list all available modes for a monitor with matching connector name
    #[arg(short, long)]
    pub connector: Option<String>,
}

#[derive(Debug, Args)]
pub struct SetArgs {
    /// Name of monitor connector, e.g. DP-1, HDMI-2
    #[arg(short, long)]
    pub connector: String,
    /// Save config to the disk after applying it. Will prompt for user input to verify if it's
    /// correct
    #[arg(short, long)]
    pub persistent: bool,
    /// New resolution, e.g. 1920x1080, 3840x2160
    #[arg(short, long, group = "res", value_parser = resolution_parser)]
    pub resolution: Option<(u32, u32)>,
    /// Automatically select highest available refresh rate
    #[arg(long, group = "res")]
    pub max_resolution: bool,
    /// New monitor refresh rate. This is selected on a best effort basis. e.g. if you
    /// select 60Hz, while monitor only supports 59.98Hz, it will be selected instead.
    #[arg(long, group = "refresh")]
    pub refresh_rate: Option<f64>,
    /// Automatically select highest refresh rate for selected resolution
    #[arg(long, group = "refresh")]
    pub max_refresh_rate: bool,
    /// Controls variable refresh rate
    #[arg(long)]
    pub vrr: Option<bool>,
    /// UI Scaling, as precentage, e.g. 100, 150, 200. This is selected based on a closest
    /// available scaling with a rounding step of 25%. e.g. if you select 125, while selected
    /// resolution only allows for either 124% or 149% - first one will be selected.
    #[arg(long)]
    pub scaling: Option<u32>,
    /// Controls high dynamic range color mode
    #[arg(long)]
    pub hdr: Option<bool>,
}

fn resolution_parser(s: &str) -> Result<(u32, u32), String> {
    let res: Vec<_> = s.split(&['x', 'X']).map(str::parse::<u32>).collect();
    if res.len() != 2 {
        return Err(format!("could not parse resolution string, expected format is <widht>x<height>, e.g. 1920x1080"));
    }
    let width = res[0].as_ref().map_err(|_| format!("could not parse resolution, width is not a number"))?;
    let height = res[1].as_ref().map_err(|_| format!("could not parse resolution, height is not a number"))?;
    Ok((*width, *height))
}
