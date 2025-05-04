use std::{cmp::Ordering, collections::HashSet};

use anyhow::anyhow;
use clap::Parser;
use displayconfig_mutter::{cli::{self, Cli}, display_config::{apply_monitors_config, get_current_state::{self, MonitorColorMode, RefreshRateMode}, DisplayConfigProxy}};
use tabled::{builder::Builder, settings::{object::Rows, Alignment, Modify, Style}};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let conn = zbus::Connection::session().await?;
    let proxy = DisplayConfigProxy::new(&conn).await?;
    let current_state = proxy.get_current_state().await?;

    match cli.command {
        cli::Command::List(cli::ListArgs{connector}) => {
            match connector {
                Some(connector) => list_modes(current_state, connector)?,
                None => list_monitors(current_state)?
            };
        },
        cli::Command::Set(args) => {
            let method = if args.persistent {
                apply_monitors_config::Method::Persistent
            } else {
                apply_monitors_config::Method::Temporary
            };

            let monitor = current_state.monitors.iter()
                .find(|monitor| monitor.id.connector == args.connector)
                .ok_or(anyhow!("could not find a display with \"{}\" connector name", args.connector))?;
            let logical_monitor = current_state.logical_monitors.iter()
                .find(|logical_monitor| logical_monitor.monitors.contains(&monitor.id))
                .ok_or(anyhow!("could not find a logical monitor that is attached to a display with \"{}\" connector name", args.connector))?;

            let mut available_modes = monitor.modes.clone();
            available_modes.sort();
            available_modes.reverse();
            let current_mode = available_modes.iter()
                .find(|mode| mode.properties.is_current.is_some_and(|f| f))
                .ok_or(anyhow!("could not find current configuration of \"{}\"", args.connector))?;

            let (width, height) = match (args.max_resolution, args.resolution) {
                (true, _) => {
                    available_modes
                        .first().map(|mode| (mode.width as u32, mode.height as u32))
                        .ok_or(anyhow!("no modes available for \"{}\"", args.connector))?
                },
                (_, Some(res)) => res,
                _ => (current_mode.width as u32, current_mode.height as u32),
            };

            let mut available_refresh_rates: Vec<_> = available_modes.iter().filter_map(|mode| {
                if mode.width as u32 == width && mode.height as u32 == height {
                    Some(mode.refresh_rate)
                } else {
                    None
                }
            }).collect();
            let refresh_rate_cmp = |l: &f64, r: &f64, target: f64| {
                let l = (l - target).abs() * 100.0;
                let r = (r - target).abs() * 100.0;
                (l as u32).cmp(&(r as u32))
            };
            let refresh_rate = match (args.max_refresh_rate, args.refresh_rate) {
                (true, _) => available_refresh_rates.first().ok_or(anyhow!("could not find any refresh rate for {}x{} resolution", width, height))?,
                (_, Some(refresh_rate)) => {
                    available_refresh_rates.sort_by(|l, r| refresh_rate_cmp(l, r, refresh_rate));
                    available_refresh_rates.first().ok_or(anyhow!("could not find refresh rate for {}x{} resolution that is close to {}", width, height, refresh_rate))?
                },
                _ => {
                    available_refresh_rates.sort_by(|l, r| refresh_rate_cmp(l, r, current_mode.refresh_rate));
                    available_refresh_rates.first().ok_or(anyhow!("could not find refresh rate for {}x{} resolution that is close to current one", width, height))?
                }
            };

            let matching_mode = if args.vrr.is_some_and(|flag| flag) {
                available_modes.iter()
                    .find(|mode| 
                        mode.width as u32 == width && mode.height as u32 == height 
                        && mode.refresh_rate == *refresh_rate 
                        && mode.properties.refresh_rate_mode.is_some_and(|mode| mode == RefreshRateMode::Variable))
                    .ok_or(anyhow!("VRR is not available"))?
            } else {
                available_modes.iter()
                    .find(|mode| 
                        mode.width as u32 == width && mode.height as u32 == height 
                        && mode.refresh_rate == *refresh_rate 
                        && (mode.properties.refresh_rate_mode.is_none() || mode.properties.refresh_rate_mode.is_some_and(|mode| mode == RefreshRateMode::Fixed)))
                    .expect("already matched a mode, but couldn't find one without VRR")
            };

            let mut supported_scales = matching_mode.supported_scales.clone();
            let wanted_scale = args.scaling.map(|scale_precent| scale_precent as f64 / 100.0).unwrap_or(logical_monitor.scale);
            supported_scales.sort_by(|l, r| {
                let l = (l * 100.0) as i32;
                let r = (r * 100.0) as i32;
                let wanted_scale = (wanted_scale * 100.0) as i32;
                (l - wanted_scale as i32).abs().cmp(&(r - wanted_scale as i32).abs())
            });
            let scale = supported_scales.first().ok_or(anyhow!("display \"{}\" does not have any supported scales", args.connector))?;
            if (wanted_scale * 4.0).round() != (scale * 4.0).round() {
                return Err(anyhow!("display \"{}\" does not have any scale close to {}%", args.connector, (wanted_scale * 100.0) as u32));
            }

            let hdr_supported = monitor.properties.supported_color_modes.as_ref().is_some_and(|modes| modes.contains(&MonitorColorMode::BT2100));
            let color_mode = args.hdr.map(|hdr| if hdr {MonitorColorMode::BT2100} else {MonitorColorMode::Default})
                .unwrap_or(monitor.properties.color_mode.unwrap_or(MonitorColorMode::Default));
            let color_mode = match(color_mode, hdr_supported) {
                (MonitorColorMode::BT2100, false) => return Err(anyhow!("display \"{}\" does not support HDR", args.connector)),
                (MonitorColorMode::Default, false) => None,
                (mode, true) => Some(mode),
            };

            proxy.apply_monitors_config(
                current_state.serial, 
                method, 
                vec![
                    apply_monitors_config::LogicalMonitor{
                        x: logical_monitor.x,
                        y: logical_monitor.y,
                        scale: *scale,
                        transform: logical_monitor.transform,
                        primary: logical_monitor.primary,
                        monitors: vec![apply_monitors_config::Monitor {
                            connector: monitor.id.connector.clone(),
                            mode: matching_mode.id.clone(),
                            properties: apply_monitors_config::MonitorProperties {
                                underscanning: None,
                                color_mode: color_mode,
                            }
                        }]
                    }
                ], 
                apply_monitors_config::Properties{
                    layout_mode: None,
                    monitors_for_lease: None, 
                },
            ).await?;
        }
    }

    Ok(())
}

fn list_monitors(current_state: get_current_state::Response) -> anyhow::Result<()> {
    let mut table_builder = Builder::new();
    table_builder
        .push_record(["Connector", "Vendor", "Product name", "Resolution", "Refresh rate", "Scaling", "VRR", "HDR"]);
    for monitor in current_state.monitors {
        let logical_monitor = current_state.logical_monitors.iter().find(|logical_monitor| logical_monitor.monitors.iter().any(|m| m.connector == monitor.id.connector));
        let scaling = match logical_monitor {
            Some(logical_monitor) => format!("{:0}%", logical_monitor.scale * 100.0),
            None => "".to_string()
        };
        let current_mode = monitor.modes.iter().find(|mode| mode.properties.is_current.unwrap_or(false));
        let vrr_supported = monitor.modes.iter().any(|mode| mode.properties.refresh_rate_mode.is_some_and(|rate_mode| rate_mode == RefreshRateMode::Variable));
        let (resolution, refresh_rate, vrr_enabled) = match current_mode {
            Some(mode) => {
                (format!("{}x{}", mode.width, mode.height), mode.refresh_rate.round().to_string(), mode.properties.refresh_rate_mode.is_some_and(|rate_mode| rate_mode == RefreshRateMode::Variable))
            },
            None => {
                ("".into(), "".into(), false)
            },
        };
        let vrr = match (vrr_supported, vrr_enabled) {
            (true, true) => "Enabled",
            (true, _) => "Supported",
            _ => "No",
        };
        let hdr_supported = monitor.properties.supported_color_modes.is_some_and(|color_modes| color_modes.contains(&MonitorColorMode::BT2100));
        let hdr_enabled = monitor.properties.color_mode.is_some_and(|mode| mode == MonitorColorMode::BT2100);
        let hdr = match (hdr_supported, hdr_enabled) {
            (true, true) => "Enabled",
            (true, _) => "Supported",
            _ => "No",
        };
        table_builder.push_record([monitor.id.connector, monitor.id.vendor, monitor.id.product, resolution, refresh_rate, scaling, vrr.into(), hdr.into()]);
    }

    let mut table = table_builder.build();
    table
        .with(Style::modern())
        .with(Modify::new(Rows::new(1..)).with(Alignment::left()));
    println!("{table}");
    Ok(())
}

fn list_modes(current_state: get_current_state::Response, connector: impl AsRef<str>) -> anyhow::Result<()> {
    let mut table_builder = Builder::new();
    table_builder
        .push_record(["Connector", "Resolutions", "Refresh rates", "Scales"]);
    let monitor = current_state.monitors.iter().find(|monitor| monitor.id.connector == connector.as_ref()).ok_or(anyhow!("Could not find a monitor with \"{}\" as a connector", connector.as_ref()))?;

    let mut resolutions = HashSet::new();
    let mut refresh_rates = HashSet::new();
    let mut scales = HashSet::new();
    for mode in &monitor.modes {
        resolutions.insert((mode.width, mode.height));
        refresh_rates.insert(mode.refresh_rate.round_ties_even() as u32);
        for scale in &mode.supported_scales {
            // Round to a closest quarter
            scales.insert(format!("{}%", ((scale * 4.0).round() / 4.0 * 100.0) as u32));
        }
    }

    let mut resolutions: Vec<_> = resolutions.into_iter().collect();
    resolutions.sort_by(|a, b| match a.0.cmp(&b.0) {
        Ordering::Equal => a.1.cmp(&b.1),
        ord => ord
    }); 
    resolutions.reverse();
    let mut refresh_rates: Vec<_> = refresh_rates.into_iter().collect();
    refresh_rates.sort();
    refresh_rates.reverse();
    let mut scales: Vec<_> = scales.into_iter().collect();
    scales.sort();
    table_builder.push_record([
        connector.as_ref().to_string(), 
        resolutions.into_iter().map(|(width, height)| format!("{width}x{height}")).collect::<Vec<_>>().join("\n"), 
        refresh_rates.iter().map(u32::to_string).collect::<Vec<_>>().join("\n"),
        scales.join("\n"),
    ]);

    let mut table = table_builder.build();
    table
        .with(Style::modern())
        .with(Modify::new(Rows::new(1..)).with(Alignment::left()));
    println!("{table}");
    Ok(())
}
