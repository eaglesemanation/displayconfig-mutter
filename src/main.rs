use std::{cmp::Ordering, collections::HashSet};

use anyhow::anyhow;
use clap::Parser;
use displayconfig_mutter::{
    cli::{self, Cli, ColorMode, SetArgs},
    display_config::{
        apply_monitors_config,
        get_current_state::{
            self, LogicalMonitorTransform, Mode, MonitorColorMode, RefreshRateMode,
        },
        DisplayConfigProxy,
    },
};
use tabled::{
    builder::Builder,
    settings::{object::Rows, Alignment, Modify, Style},
};
use tokio::fs;
use zbus::zvariant::{
    self,
    serialized::{Context, Data},
    LE,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let conn = zbus::Connection::session().await?;
    let proxy = DisplayConfigProxy::new(&conn).await?;
    let current_state = proxy.get_current_state().await?;

    match cli.command {
        cli::Command::List(cli::ListArgs { connector }) => {
            match connector {
                Some(connector) => list_modes(current_state, connector)?,
                None => list_monitors(current_state)?,
            };
        }
        cli::Command::Set(args) => {
            let current_monitor = current_state
                .monitors
                .iter()
                .find(|monitor| monitor.id.connector == args.connector)
                .ok_or(anyhow!(
                    "could not find a display with \"{}\" connector name",
                    args.connector
                ))?;
            let mut available_modes = current_monitor.modes.clone();
            available_modes.sort();
            available_modes.reverse();
            let current_mode = available_modes
                .iter()
                .find(|mode| mode.properties.is_current.is_some_and(|f| f));
            let prefered_mode = current_mode
                .or_else(|| {
                    available_modes
                        .iter()
                        .find(|mode| mode.properties.is_preferred.is_some_and(|f| f))
                })
                .ok_or(anyhow!(
                    "could not find neither current mode nor preferred mode"
                ))?;

            let mut logical_monitors = new_logical_monitors(&current_state)?;
            if args.primary {
                for monitor in &mut logical_monitors {
                    monitor.primary = false;
                }
            }

            let (
                target_logical_monitor_idx,
                mut target_width,
                mut target_height,
                target_x,
                target_y,
            ) = {
                // Nested scope for mutating the target logical monitor without cloning
                let mut logical_monitor_matches =
                    logical_monitors.iter_mut().peekable().enumerate().filter(
                        |(_, logical_monitor)| {
                            logical_monitor
                                .monitors
                                .iter()
                                .any(|monitor| monitor.connector == args.connector)
                        },
                    );
                let (target_logical_monitor_idx, target_logical_monitor) =
                    match logical_monitor_matches.next() {
                        None => {
                            // Logical monitor is not present for matched monitor, add new one
                            logical_monitors.push(apply_monitors_config::LogicalMonitor {
                                x: 0,
                                y: 0,
                                scale: 1.0,
                                transform: LogicalMonitorTransform::Normal,
                                primary: false,
                                monitors: vec![apply_monitors_config::Monitor {
                                    connector: current_monitor.id.connector.clone(),
                                    mode: prefered_mode.id.clone(),
                                    properties: apply_monitors_config::MonitorProperties {
                                        underscanning: current_monitor.properties.is_underscanning,
                                        color_mode: current_monitor.properties.color_mode,
                                    },
                                }],
                            });
                            (
                                logical_monitors.len() - 1,
                                logical_monitors.last_mut().unwrap(/* just pushed */),
                            )
                        }
                        Some(first) => {
                            if logical_monitor_matches.next().is_some() {
                                return Err(anyhow!("Multiple logical monitors manage the same monitor \"{} {}\" ({}). I don't know how to handle this", current_monitor.id.vendor, current_monitor.id.product, current_monitor.id.connector));
                            } else {
                                first
                            }
                        }
                    };
                if args.primary {
                    target_logical_monitor.primary = true;
                }
                let target_monitor = target_logical_monitor.monitors.iter_mut().next().expect("Failed to validate that logical monitor has at least 1 monitor attached to it, aborting");

                let matching_mode = match_mode(&args, &available_modes, prefered_mode)?;
                target_monitor.mode = matching_mode.id.clone();

                target_logical_monitor.scale =
                    match_scale(&args, matching_mode, target_logical_monitor.scale)?;

                let color_mode = match_color_mode(
                    &args,
                    &current_monitor.properties.supported_color_modes,
                    current_monitor.properties.color_mode,
                )?;
                target_monitor.properties.color_mode = match color_mode {
                    MonitorColorMode::Default => None,
                    mode => Some(mode),
                };

                (
                    target_logical_monitor_idx,
                    matching_mode.width,
                    matching_mode.height,
                    target_logical_monitor.x,
                    target_logical_monitor.y,
                )
            };

            if args.disable {
                let dropped_primary = logical_monitors[target_logical_monitor_idx].primary;
                logical_monitors.remove(target_logical_monitor_idx);
                if dropped_primary {
                    if let Some(mon) = logical_monitors.first_mut() {
                        mon.primary = true;
                    }
                }
                target_width = 0;
                target_height = 0;
            }

            logical_monitors.sort_by(|l, r| match l.y.cmp(&r.y) {
                Ordering::Equal => l.x.cmp(&r.x),
                cmp => cmp,
            });
            let (current_width, current_height) = {
                if let Some(mode) = current_mode {
                    (mode.width, mode.height)
                } else {
                    (0, 0)
                }
            };
            // TODO: Research better algorithms for inserting rectangles in such a way that edges
            // keep touching. This is me mostly guessing and probably misshandling a bunch of edge
            // cases
            for (idx, logical_monitor) in logical_monitors.iter_mut().enumerate() {
                if idx == target_logical_monitor_idx && !args.disable {
                    continue;
                }
                if logical_monitor.x > target_x || (target_x == 0 && logical_monitor.x >= 0) {
                    logical_monitor.x += target_width - current_width;
                }
                if logical_monitor.y > target_y {
                    logical_monitor.y += target_height - current_height;
                }
            }

            proxy
                .apply_monitors_config(
                    current_state.serial,
                    if args.persistent {
                        apply_monitors_config::Method::Persistent
                    } else {
                        apply_monitors_config::Method::Temporary
                    },
                    logical_monitors,
                    apply_monitors_config::Properties {
                        layout_mode: None,
                        monitors_for_lease: None,
                    },
                )
                .await?;
        }
        cli::Command::SaveFile(args) => {
            let logical_monitors = new_logical_monitors(&current_state)?;

            let context = Context::new_dbus(LE, 0);
            let encoded = zvariant::to_bytes(context, &logical_monitors)?;

            if let Some(parent) = args.file_path.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::write(args.file_path, encoded).await?;
        }
        cli::Command::LoadFile(args) => {
            let encoded = fs::read(args.file_path).await?;

            let context = Context::new_dbus(LE, 0);
            let data = Data::new(encoded, context);

            let logical_monitors: Vec<apply_monitors_config::LogicalMonitor> =
                data.deserialize()?.0;

            proxy
                .apply_monitors_config(
                    current_state.serial,
                    if args.persistent {
                        apply_monitors_config::Method::Persistent
                    } else {
                        apply_monitors_config::Method::Temporary
                    },
                    logical_monitors,
                    apply_monitors_config::Properties {
                        layout_mode: None,
                        monitors_for_lease: None,
                    },
                )
                .await?;
        }
    }

    Ok(())
}

/// Find a best effort match of requested resolution + refresh rate + vrr, in that order
fn match_mode<'a>(
    args: &SetArgs,
    available_modes: &'a [Mode],
    current_mode: &Mode,
) -> anyhow::Result<&'a Mode> {
    let (width, height) = match (args.max_resolution, args.resolution) {
        (true, _) => available_modes
            .first()
            .map(|mode| (mode.width as u32, mode.height as u32))
            .ok_or(anyhow!("no modes available for \"{}\"", args.connector))?,
        (_, Some(res)) => res,
        _ => (current_mode.width as u32, current_mode.height as u32),
    };

    let mut available_refresh_rates: Vec<_> = available_modes
        .iter()
        .filter_map(|mode| {
            if mode.width as u32 == width && mode.height as u32 == height {
                Some(mode.refresh_rate)
            } else {
                None
            }
        })
        .collect();
    let refresh_rate_cmp = |l: &f64, r: &f64, target: f64| {
        let l = (l - target).abs() * 100.0;
        let r = (r - target).abs() * 100.0;
        (l as u32).cmp(&(r as u32))
    };
    let refresh_rate = match (args.max_refresh_rate, args.refresh_rate) {
        (true, _) => available_refresh_rates.first().ok_or(anyhow!(
            "could not find any refresh rate for {}x{} resolution",
            width,
            height
        ))?,
        (_, Some(refresh_rate)) => {
            available_refresh_rates.sort_by(|l, r| refresh_rate_cmp(l, r, refresh_rate));
            available_refresh_rates.first().ok_or(anyhow!(
                "could not find refresh rate for {}x{} resolution that is close to {}",
                width,
                height,
                refresh_rate
            ))?
        }
        _ => {
            available_refresh_rates
                .sort_by(|l, r| refresh_rate_cmp(l, r, current_mode.refresh_rate));
            available_refresh_rates.first().ok_or(anyhow!(
                "could not find refresh rate for {}x{} resolution that is close to current one",
                width,
                height
            ))?
        }
    };

    if args.vrr.is_some_and(|flag| flag) {
        available_modes
            .iter()
            .find(|mode| {
                mode.width as u32 == width
                    && mode.height as u32 == height
                    && mode.refresh_rate == *refresh_rate
                    && mode
                        .properties
                        .refresh_rate_mode
                        .is_some_and(|mode| mode == RefreshRateMode::Variable)
            })
            .ok_or(anyhow!("VRR is not available"))
    } else {
        available_modes
            .iter()
            .find(|mode| {
                mode.width as u32 == width
                    && mode.height as u32 == height
                    && mode.refresh_rate == *refresh_rate
                    && (mode.properties.refresh_rate_mode.is_none()
                        || mode
                            .properties
                            .refresh_rate_mode
                            .is_some_and(|mode| mode == RefreshRateMode::Fixed))
            })
            .ok_or(anyhow!(
                "already matched a mode, but couldn't find one without VRR"
            ))
    }
}

/// Looks for a closest matching scale with rounding within 25%
fn match_scale(args: &SetArgs, matching_mode: &Mode, current_scale: f64) -> anyhow::Result<f64> {
    let mut supported_scales = matching_mode.supported_scales.clone();
    let wanted_scale = args
        .scaling
        .map(|scale_percent| scale_percent as f64 / 100.0)
        .unwrap_or(current_scale);
    supported_scales.sort_by(|l, r| {
        let l = (l * 100.0) as i32;
        let r = (r * 100.0) as i32;
        let wanted_scale = (wanted_scale * 100.0) as i32;
        (l - wanted_scale).abs().cmp(&(r - wanted_scale).abs())
    });
    let scale = supported_scales.first().ok_or(anyhow!(
        "display \"{}\" does not have any supported scales",
        args.connector
    ))?;
    if (wanted_scale * 4.0).round() != (scale * 4.0).round() {
        return Err(anyhow!(
            "display \"{}\" does not have any scale close to {}%",
            args.connector,
            (wanted_scale * 100.0) as u32
        ));
    }
    Ok(*scale)
}

fn match_color_mode(
    args: &SetArgs,
    supported_color_modes: &Option<Vec<MonitorColorMode>>,
    current_color_mode: Option<MonitorColorMode>,
) -> anyhow::Result<MonitorColorMode> {
    let fallback_color_modes = vec![MonitorColorMode::Default];
    let supported_color_modes = supported_color_modes
        .as_ref()
        .unwrap_or(&fallback_color_modes);
    let color_mode = match (args.hdr, args.color_mode) {
        (None, None) => current_color_mode.unwrap_or(MonitorColorMode::Default),
        (Some(hdr), None) => {
            if hdr {
                MonitorColorMode::BT2100
            } else {
                MonitorColorMode::Default
            }
        }
        (None, Some(color_mode)) => match color_mode {
            ColorMode::SDR => MonitorColorMode::Default,
            ColorMode::HDR => MonitorColorMode::BT2100,
            ColorMode::SDRNative => MonitorColorMode::SDRNative,
        },
        (Some(_), Some(_)) => panic!("Clap allowed mutually exclusive flags"),
    };

    if !supported_color_modes.contains(&color_mode) {
        return Err(anyhow!(
            "display \"{}\" does not support selected color mode",
            args.connector
        ));
    }
    Ok(color_mode)
}

/// Produces a list of logical monitors in apply_monitors_config format that would keep the same
/// configuration as current_state
fn new_logical_monitors(
    current_state: &get_current_state::Response,
) -> anyhow::Result<Vec<apply_monitors_config::LogicalMonitor>> {
    let mut out = vec![];
    for logical_monitor in &current_state.logical_monitors {
        let mut monitors = vec![];
        for monitor_id in &logical_monitor.monitors {
            let monitor = current_state
                .monitors
                .iter()
                .find(|monitor| &monitor.id == monitor_id)
                .ok_or(anyhow!(
                    "Logical monitor references \"{} {}\" ({}), but it's not found",
                    monitor_id.vendor,
                    monitor_id.product,
                    monitor_id.connector
                ))?;
            let current_mode = monitor
                .modes
                .iter()
                .find(|mode| mode.properties.is_current.unwrap_or(false))
                .ok_or(anyhow!(
                    "Monitor \"{} {}\" ({}) does not have a mode that is marked as current",
                    monitor_id.vendor,
                    monitor_id.product,
                    monitor_id.connector
                ))?;
            monitors.push(apply_monitors_config::Monitor {
                connector: monitor_id.connector.to_owned(),
                mode: current_mode.id.clone(),
                properties: apply_monitors_config::MonitorProperties {
                    underscanning: monitor.properties.is_underscanning,
                    color_mode: monitor.properties.color_mode,
                },
            });
        }
        out.push(apply_monitors_config::LogicalMonitor {
            x: logical_monitor.x,
            y: logical_monitor.y,
            scale: logical_monitor.scale,
            transform: logical_monitor.transform,
            primary: logical_monitor.primary,
            monitors,
        });
    }
    Ok(out)
}

fn list_monitors(current_state: get_current_state::Response) -> anyhow::Result<()> {
    let mut table_builder = Builder::new();
    table_builder.push_record([
        "Connector",
        "Vendor",
        "Product name",
        "Resolution",
        "Refresh rate",
        "Scaling",
        "VRR",
        "HDR",
    ]);
    for monitor in current_state.monitors {
        let logical_monitor = current_state
            .logical_monitors
            .iter()
            .find(|logical_monitor| {
                logical_monitor
                    .monitors
                    .iter()
                    .any(|m| m.connector == monitor.id.connector)
            });
        let scaling = match logical_monitor {
            Some(logical_monitor) => format!("{:0}%", logical_monitor.scale * 100.0),
            None => "".to_string(),
        };
        let current_mode = monitor
            .modes
            .iter()
            .find(|mode| mode.properties.is_current.unwrap_or(false));
        let vrr_supported = monitor.modes.iter().any(|mode| {
            mode.properties
                .refresh_rate_mode
                .is_some_and(|rate_mode| rate_mode == RefreshRateMode::Variable)
        });
        let (resolution, refresh_rate, vrr_enabled) = match current_mode {
            Some(mode) => (
                format!("{}x{}", mode.width, mode.height),
                mode.refresh_rate.round().to_string(),
                mode.properties
                    .refresh_rate_mode
                    .is_some_and(|rate_mode| rate_mode == RefreshRateMode::Variable),
            ),
            None => ("".into(), "".into(), false),
        };
        let vrr = match (vrr_supported, vrr_enabled) {
            (true, true) => "Enabled",
            (true, _) => "Supported",
            _ => "No",
        };
        let hdr_supported = monitor
            .properties
            .supported_color_modes
            .is_some_and(|color_modes| color_modes.contains(&MonitorColorMode::BT2100));
        let hdr_enabled = monitor
            .properties
            .color_mode
            .is_some_and(|mode| mode == MonitorColorMode::BT2100);
        let hdr = match (hdr_supported, hdr_enabled) {
            (true, true) => "Enabled",
            (true, _) => "Supported",
            _ => "No",
        };
        table_builder.push_record([
            monitor.id.connector,
            monitor.id.vendor,
            monitor.id.product,
            resolution,
            refresh_rate,
            scaling,
            vrr.into(),
            hdr.into(),
        ]);
    }

    let mut table = table_builder.build();
    table
        .with(Style::modern())
        .with(Modify::new(Rows::new(1..)).with(Alignment::left()));
    println!("{table}");
    Ok(())
}

fn list_modes(
    current_state: get_current_state::Response,
    connector: impl AsRef<str>,
) -> anyhow::Result<()> {
    let mut table_builder = Builder::new();
    table_builder.push_record(["Connector", "Resolutions", "Refresh rates", "Scales"]);
    let monitor = current_state
        .monitors
        .iter()
        .find(|monitor| monitor.id.connector == connector.as_ref())
        .ok_or(anyhow!(
            "Could not find a monitor with \"{}\" as a connector",
            connector.as_ref()
        ))?;

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
        ord => ord,
    });
    resolutions.reverse();
    let mut refresh_rates: Vec<_> = refresh_rates.into_iter().collect();
    refresh_rates.sort();
    refresh_rates.reverse();
    let mut scales: Vec<_> = scales.into_iter().collect();
    scales.sort();
    table_builder.push_record([
        connector.as_ref().to_string(),
        resolutions
            .into_iter()
            .map(|(width, height)| format!("{width}x{height}"))
            .collect::<Vec<_>>()
            .join("\n"),
        refresh_rates
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
        scales.join("\n"),
    ]);

    let mut table = table_builder.build();
    table
        .with(Style::modern())
        .with(Modify::new(Rows::new(1..)).with(Alignment::left()));
    println!("{table}");
    Ok(())
}
