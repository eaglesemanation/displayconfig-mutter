use std::collections::HashMap;

pub mod get_current_state {
    use std::cmp;

    use serde::{Deserialize, Serialize};
    use serde_repr::{Deserialize_repr, Serialize_repr};
    use zbus::zvariant::{DeserializeDict, SerializeDict, Type};

    #[derive(Debug, Clone, Type, Serialize, Deserialize)]
    pub struct Response {
        /// configuration serial
        pub serial: u32,
        /// available monitors
        pub monitors: Vec<Monitor>,
        /// current logical monitor configuration
        pub logical_monitors: Vec<LogicalMonitor>,
        /// display configuration properties
        pub properties: Properties,
    }

    #[derive(Debug, Clone, DeserializeDict, SerializeDict, Type)]
    #[zvariant(signature = "dict", rename_all="kebab-case")]
    pub struct Properties {
        pub layout_mode: Option<LayoutMode>,
        /// True if the layout mode can be changed. Absence of this means the layout mode cannot be changed.
        pub supports_changing_layout_mode: Option<bool>,
        /// True if all the logical monitors must always use the same scale. Absence of this means logical monitor scales can differ.
        pub global_scale_required: Option<bool>,
    }

    /**
    * Represents in what way logical monitors are laid
    * out on the screen. The layout mode can be either
    * of the ones listed below. Absence of this property
    * means the layout mode cannot be changed, and that
    * "logical" mode is assumed to be used.
    */
    #[derive(Debug, Type, PartialEq, Eq, Default, Clone, Copy, Serialize_repr, Deserialize_repr)]
    #[repr(u32)]
    pub enum LayoutMode {
        /// the dimension of a logical monitor is derived from the monitor modes associated with it, then scaled using the logical monitor scale.
        #[default]
        Logical = 1,
        /// the dimension of a logical monitor is derived from the monitor modes associated with it.
        Physical = 2,
    }

    /// represents connected physical monitor
    #[derive(Debug, Clone, Type, Serialize, Deserialize)]
    pub struct Monitor {
        pub id: MonitorId,
        /// available modes
        pub modes: Vec<Mode>,
        /// optional properties
        pub properties: MonitorProperties,
    }


    #[derive(Debug, Clone, Type, Serialize, Deserialize)]
    pub struct MonitorId {
        /// connector name (e.g. HDMI-1, DP-1, etc)
        pub connector: String,
        /// vendor name
        pub vendor: String,
        /// product name
        pub product: String,
        /// product serial
        pub serial: String,
    }

    impl PartialEq for MonitorId {
        fn eq(&self, other: &Self) -> bool {
            self.connector == other.connector
        }
    }

    #[derive(Debug, Clone, DeserializeDict, SerializeDict, Type)]
    #[zvariant(signature = "dict", rename_all="kebab-case")]
    pub struct MonitorProperties {
        /// physical width of monitor in millimeters
        pub width_mm: Option<i32>,
        /// physical height of monitor in millimeters
        pub height_mm: Option<i32>,
        /// whether underscanning is enabled (absence of this means underscanning not being supported)
        pub is_underscanning: Option<bool>,
        /// the maximum size a screen may have (absence of this means unlimited screen size)
        pub max_screen_size: Option<(i32, i32)>,
        /// whether the monitor is built in, e.g. a laptop panel (absence of this means it is not built in)
        pub is_builtin: Option<bool>,
        /// a human readable display name of the monitor
        pub display_name: Option<String>,
        /// the state of the privacy screen (absence of this means it is not being supported) first value indicates whether it's enabled and second value whether it's hardware locked (and so can't be changed via gsettings)
        pub privacy_screen_state: Option<(bool, bool)>,
        /// minimum refresh rate of monitor when Variable Refresh Rate is active (absence of this means unknown)
        pub min_refresh_rate: Option<i32>,
        /// whether the monitor is for lease or not
        pub is_for_lease: Option<bool>,
        /// current color mode
        pub color_mode: Option<MonitorColorMode>,
        /// list of supported color modes
        pub supported_color_modes: Option<Vec<MonitorColorMode>>,
    }

    #[derive(Debug, Type, PartialEq, Eq, Default, Clone, Copy, Serialize_repr, Deserialize_repr)]
    #[repr(u32)]
    pub enum MonitorColorMode {
        #[default]
        Default = 0,
        /// HDR
        BT2100 = 1
    }

    #[derive(Debug, Clone, Type, Serialize, Deserialize)]
    pub struct Mode {
        /// mode ID
        pub id: String,
        /// width in physical pixels
        pub width: i32,
        /// height in physical pixels
        pub height: i32,
        /// refresh rate
        pub refresh_rate: f64,
        /// scale preferred as per calculations
        pub preferred_scale: f64,
        /// scales supported by this mode
        pub supported_scales: Vec<f64>,
        /// optional properties
        pub properties: ModeProperties,
    }

    impl PartialEq for Mode {
        fn eq(&self, other: &Self) -> bool {
            self.width == other.width && self.height == other.height && self.refresh_rate == other.refresh_rate && self.properties.refresh_rate_mode == other.properties.refresh_rate_mode
        }
    }
    impl Eq for Mode {}

    impl PartialOrd for Mode {
        fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for Mode {
        fn cmp(&self, other: &Self) -> cmp::Ordering {
            match self.width.cmp(&other.width) {
                cmp::Ordering::Equal => {},
                ord => return ord,
            }
            match self.height.cmp(&other.height) {
                cmp::Ordering::Equal => {},
                ord => return ord,
            }
            match (self.refresh_rate.round_ties_even() as u32).cmp(&(other.refresh_rate.round_ties_even() as u32)) {
                cmp::Ordering::Equal => {},
                ord => return ord,
            }
            self.properties.refresh_rate_mode.cmp(&other.properties.refresh_rate_mode)
        }
    }

    #[derive(Debug, Type, PartialOrd, Ord, PartialEq, Eq, Clone, Copy, Default, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    #[zvariant(signature = "s")]
    pub enum RefreshRateMode {
        #[default]
        Fixed,
        Variable
    }

    #[derive(Debug, Clone, Type, SerializeDict, DeserializeDict)]
    #[zvariant(signature = "dict", rename_all="kebab-case")]
    pub struct ModeProperties {
        /// the mode is currently active mode
        pub is_current: Option<bool>,
        /// the mode is the preferred mode
        pub is_preferred: Option<bool>,
        /// the mode is an interlaced mode
        pub is_interlaced: Option<bool>,
        /// the refresh rate mode, either "variable" or "fixed" (absence of this means "fixed")
        pub refresh_rate_mode: Option<RefreshRateMode>,
    }

    /// logical monitor transform
    #[derive(Debug, Type, PartialEq, Eq, Default, Clone, Copy, Serialize_repr, Deserialize_repr)]
    #[repr(u32)]
    pub enum LogicalMonitorTransform {
        #[default]
        Normal = 0,
        Turned90Deg = 1,
        Turned180Deg = 2,
        Turned270Deg = 3,
        Flipped = 4,
        Turned90DegFlipped = 5,
        Turned180DegFlipped = 6,
        Turned270DegFlipped = 7,
    }

    #[derive(Debug, Clone, Type, Serialize, Deserialize)]
    pub struct LogicalMonitor {
        /// x position
        pub x: i32,
        /// y position
        pub y: i32,
        /// scale
        pub scale: f64,
        /// logical monitor transform
        pub transform: LogicalMonitorTransform,
        /// true if this is the primary logical monitor
        pub primary: bool,
        /// monitors displaying this logical monitor
        pub monitors: Vec<MonitorId>,
        /// possibly other properties
        pub properties: LogicalMonitorProperties,
    }

    #[derive(Debug, Clone, DeserializeDict, SerializeDict, Type)]
    #[zvariant(signature = "dict", rename_all="kebab-case")]
    pub struct LogicalMonitorProperties {}
}

pub mod apply_monitors_config {
    use serde::{Deserialize, Serialize};
    use serde_repr::{Deserialize_repr, Serialize_repr};
    use zbus::zvariant::{DeserializeDict, SerializeDict, Type};

    use super::get_current_state::{LayoutMode, LogicalMonitorTransform, MonitorColorMode, MonitorId};

    /// may effect the global monitor configuration state
    #[derive(Debug, DeserializeDict, SerializeDict, Type, Default)]
    #[zvariant(signature = "dict", rename_all="kebab-case")]
    pub struct Properties {
        pub layout_mode: Option<LayoutMode>,
        pub monitors_for_lease: Option<Vec<MonitorId>>,
    }

    /// represents the way the configuration should be handled
    #[derive(Debug, Type, PartialEq, Eq, Default, Clone, Copy, Serialize_repr, Deserialize_repr)]
    #[repr(u32)]
    pub enum Method {
        /// Check if provided arguments are valid
        #[default]
        Verify = 0,
        /// Set config without storing it to the disk
        Temporary = 1,
        /// Set config and safe it to disk. Will ask for confirmation from user before saving
        Persistent = 2,
    }

    #[derive(Debug, Type, Serialize, Deserialize)]
    pub struct LogicalMonitor {
        /// layout x position
        pub x: i32,
        /// layout y position
        pub y: i32,
        /// scale
        pub scale: f64,
        pub transform: LogicalMonitorTransform,
        /// true if this is the primary logical monitor
        pub primary: bool,
        pub monitors: Vec<Monitor>,
    }

    #[derive(Debug, Type, Serialize, Deserialize)]
    pub struct Monitor {
        /// connector name
        pub connector: String,
        /// mode ID
        pub mode: String,
        pub properties: MonitorProperties,
    }

    #[derive(Debug, Clone, Default, DeserializeDict, SerializeDict, Type)]
    #[zvariant(signature = "dict", rename_all="kebab-case")]
    pub struct MonitorProperties {
        pub underscanning: Option<bool>,
        pub color_mode: Option<MonitorColorMode>,
    }
}

#[zbus::proxy(
    default_service = "org.gnome.Mutter.DisplayConfig",
    default_path = "/org/gnome/Mutter/DisplayConfig",
    interface = "org.gnome.Mutter.DisplayConfig"
)]
pub trait DisplayConfig {
    /// ApplyMonitorsConfig method
    fn apply_monitors_config(
        &self,
        serial: u32,
        method: apply_monitors_config::Method,
        logical_monitors: Vec<apply_monitors_config::LogicalMonitor>,
        properties: apply_monitors_config::Properties,
    ) -> zbus::Result<()>;

    /// ChangeBacklight method
    fn change_backlight(&self, serial: u32, output: u32, value: i32) -> zbus::Result<i32>;

    /// GetCurrentState method
    fn get_current_state(&self) -> zbus::Result<get_current_state::Response>;

    /// ResetLuminance method
    fn reset_luminance(&self, connector: &str, color_mode: u32) -> zbus::Result<()>;

    /// SetBacklight method
    fn set_backlight(&self, serial: u32, connector: &str, value: i32) -> zbus::Result<()>;

    /// SetLuminance method
    fn set_luminance(&self, connector: &str, color_mode: u32, luminance: f64) -> zbus::Result<()>;

    /// MonitorsChanged signal
    #[zbus(signal)]
    fn monitors_changed(&self) -> zbus::Result<()>;

    /// ApplyMonitorsConfigAllowed property
    #[zbus(property)]
    fn apply_monitors_config_allowed(&self) -> zbus::Result<bool>;

    /// Backlight property
    #[zbus(property)]
    fn backlight(
        &self,
    ) -> zbus::Result<(
        u32,
        Vec<HashMap<String, zbus::zvariant::OwnedValue>>,
    )>;

    /// HasExternalMonitor property
    #[zbus(property)]
    fn has_external_monitor(&self) -> zbus::Result<bool>;

    /// Luminance property
    #[zbus(property)]
    fn luminance(
        &self,
    ) -> zbus::Result<Vec<HashMap<String, zbus::zvariant::OwnedValue>>>;

    /// NightLightSupported property
    #[zbus(property)]
    fn night_light_supported(&self) -> zbus::Result<bool>;

    /// PanelOrientationManaged property
    #[zbus(property)]
    fn panel_orientation_managed(&self) -> zbus::Result<bool>;

    /// PowerSaveMode property
    #[zbus(property)]
    fn power_save_mode(&self) -> zbus::Result<i32>;
    #[zbus(property)]
    fn set_power_save_mode(&self, value: i32) -> zbus::Result<()>;
}
