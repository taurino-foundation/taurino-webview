use crate::utils::{
    error::Error,
    types::{
        CursorIcon, Icon, Monitor, ProgressBarState, ProgressBarStatus, Theme,
        UserAttentionType,
    },
};
use dpi::{
    LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize, Position,
    Size,
};
use tao::{
    dpi::{
        LogicalPosition as TaoLogicalPosition, LogicalSize as TaoLogicalSize,
        PhysicalPosition as TaoPhysicalPosition,
        PhysicalSize as TaoPhysicalSize, Position as TaoPosition,
        Size as TaoSize,
    },
    monitor::MonitorHandle,
    window::{
        CursorIcon as TaoCursorIcon, Icon as TaoWindowIcon,
        ProgressBarState as TaoProgressBarState,
        ProgressState as TaoProgressState, Theme as TaoTheme,
        UserAttentionType as TaoUserAttentionType,
    },
};

// Your TaoIcon wrapper (already defined)
pub struct TaoIcon(pub TaoWindowIcon);

impl TryFrom<Icon<'_>> for TaoIcon {
    type Error = Error;
    fn try_from(icon: Icon<'_>) -> std::result::Result<Self, Self::Error> {
        TaoWindowIcon::from_rgba(icon.rgba.to_vec(), icon.width, icon.height)
            .map(Self)
            .map_err(|e| Error::InvalidTaoIcon(Box::new(e)))
    }
}

pub fn map_theme(theme: &TaoTheme) -> Theme {
    match theme {
        TaoTheme::Light => Theme::Light,
        TaoTheme::Dark => Theme::Dark,
        _ => Theme::Light,
    }
}

#[cfg(target_os = "macos")]
fn tao_activation_policy(
    activation_policy: ActivationPolicy,
) -> TaoActivationPolicy {
    match activation_policy {
        ActivationPolicy::Regular => TaoActivationPolicy::Regular,
        ActivationPolicy::Accessory => TaoActivationPolicy::Accessory,
        ActivationPolicy::Prohibited => TaoActivationPolicy::Prohibited,
        _ => unimplemented!(),
    }
}

pub struct MonitorHandleWrapper(pub MonitorHandle);

impl From<MonitorHandleWrapper> for Monitor {
    fn from(monitor: MonitorHandleWrapper) -> Monitor {
        Self {
            name: monitor.0.name(),
            position: PhysicalPositionWrapper(monitor.0.position()).into(),
            size: PhysicalSizeWrapper(monitor.0.size()).into(),
            /* work_area: monitor.0.work_area(), */
            work_area: todo!(),
            scale_factor: monitor.0.scale_factor(),
        }
    }
}

pub struct PhysicalPositionWrapper<T>(pub TaoPhysicalPosition<T>);

impl<T> From<PhysicalPositionWrapper<T>> for PhysicalPosition<T> {
    fn from(position: PhysicalPositionWrapper<T>) -> Self {
        Self {
            x: position.0.x,
            y: position.0.y,
        }
    }
}

impl<T> From<PhysicalPosition<T>> for PhysicalPositionWrapper<T> {
    fn from(position: PhysicalPosition<T>) -> Self {
        Self(TaoPhysicalPosition {
            x: position.x,
            y: position.y,
        })
    }
}

struct LogicalPositionWrapper<T>(TaoLogicalPosition<T>);

impl<T> From<LogicalPosition<T>> for LogicalPositionWrapper<T> {
    fn from(position: LogicalPosition<T>) -> Self {
        Self(TaoLogicalPosition {
            x: position.x,
            y: position.y,
        })
    }
}

pub struct PhysicalSizeWrapper<T>(pub TaoPhysicalSize<T>);

impl<T> From<PhysicalSizeWrapper<T>> for PhysicalSize<T> {
    fn from(size: PhysicalSizeWrapper<T>) -> Self {
        Self {
            width: size.0.width,
            height: size.0.height,
        }
    }
}

impl<T> From<PhysicalSize<T>> for PhysicalSizeWrapper<T> {
    fn from(size: PhysicalSize<T>) -> Self {
        Self(TaoPhysicalSize {
            width: size.width,
            height: size.height,
        })
    }
}

struct LogicalSizeWrapper<T>(TaoLogicalSize<T>);

impl<T> From<LogicalSize<T>> for LogicalSizeWrapper<T> {
    fn from(size: LogicalSize<T>) -> Self {
        Self(TaoLogicalSize {
            width: size.width,
            height: size.height,
        })
    }
}

pub struct SizeWrapper(pub TaoSize);

impl From<Size> for SizeWrapper {
    fn from(size: Size) -> Self {
        match size {
            Size::Logical(s) => {
                Self(TaoSize::Logical(LogicalSizeWrapper::from(s).0))
            }
            Size::Physical(s) => {
                Self(TaoSize::Physical(PhysicalSizeWrapper::from(s).0))
            }
        }
    }
}

pub struct PositionWrapper(pub TaoPosition);

impl From<Position> for PositionWrapper {
    fn from(position: Position) -> Self {
        match position {
            Position::Logical(s) => {
                Self(TaoPosition::Logical(LogicalPositionWrapper::from(s).0))
            }
            Position::Physical(s) => {
                Self(TaoPosition::Physical(PhysicalPositionWrapper::from(s).0))
            }
        }
    }
}

pub(crate) fn find_monitor_for_position(
    monitors: impl Iterator<Item = MonitorHandle>,
    window_position: TaoPosition,
) -> Option<MonitorHandle> {
    monitors.into_iter().find(|m| {
        let monitor_pos = m.position();
        let monitor_size = m.size();

        // type annotations required for 32bit targets.
        let window_position =
            window_position.to_physical::<i32>(m.scale_factor());

        monitor_pos.x <= window_position.x
            && window_position.x < monitor_pos.x + monitor_size.width as i32
            && monitor_pos.y <= window_position.y
            && window_position.y < monitor_pos.y + monitor_size.height as i32
    })
}

#[derive(Debug, Clone)]
pub struct UserAttentionTypeWrapper(pub TaoUserAttentionType);

impl From<UserAttentionType> for UserAttentionTypeWrapper {
    fn from(request_type: UserAttentionType) -> Self {
        let o = match request_type {
            UserAttentionType::Critical => TaoUserAttentionType::Critical,
            UserAttentionType::Informational => {
                TaoUserAttentionType::Informational
            }
        };
        Self(o)
    }
}

#[derive(Debug)]
pub struct CursorIconWrapper(pub TaoCursorIcon);

impl From<CursorIcon> for CursorIconWrapper {
    fn from(icon: CursorIcon) -> Self {
        use CursorIcon::*;
        let i = match icon {
            Default => TaoCursorIcon::Default,
            Crosshair => TaoCursorIcon::Crosshair,
            Hand => TaoCursorIcon::Hand,
            Arrow => TaoCursorIcon::Arrow,
            Move => TaoCursorIcon::Move,
            Text => TaoCursorIcon::Text,
            Wait => TaoCursorIcon::Wait,
            Help => TaoCursorIcon::Help,
            Progress => TaoCursorIcon::Progress,
            NotAllowed => TaoCursorIcon::NotAllowed,
            ContextMenu => TaoCursorIcon::ContextMenu,
            Cell => TaoCursorIcon::Cell,
            VerticalText => TaoCursorIcon::VerticalText,
            Alias => TaoCursorIcon::Alias,
            Copy => TaoCursorIcon::Copy,
            NoDrop => TaoCursorIcon::NoDrop,
            Grab => TaoCursorIcon::Grab,
            Grabbing => TaoCursorIcon::Grabbing,
            AllScroll => TaoCursorIcon::AllScroll,
            ZoomIn => TaoCursorIcon::ZoomIn,
            ZoomOut => TaoCursorIcon::ZoomOut,
            EResize => TaoCursorIcon::EResize,
            NResize => TaoCursorIcon::NResize,
            NeResize => TaoCursorIcon::NeResize,
            NwResize => TaoCursorIcon::NwResize,
            SResize => TaoCursorIcon::SResize,
            SeResize => TaoCursorIcon::SeResize,
            SwResize => TaoCursorIcon::SwResize,
            WResize => TaoCursorIcon::WResize,
            EwResize => TaoCursorIcon::EwResize,
            NsResize => TaoCursorIcon::NsResize,
            NeswResize => TaoCursorIcon::NeswResize,
            NwseResize => TaoCursorIcon::NwseResize,
            ColResize => TaoCursorIcon::ColResize,
            RowResize => TaoCursorIcon::RowResize,
            #[allow(unreachable_patterns)]
            _ => TaoCursorIcon::Default,
        };
        Self(i)
    }
}

pub struct ProgressStateWrapper(pub TaoProgressState);

impl From<ProgressBarStatus> for ProgressStateWrapper {
    fn from(status: ProgressBarStatus) -> Self {
        let state = match status {
            ProgressBarStatus::None => TaoProgressState::None,
            ProgressBarStatus::Normal => TaoProgressState::Normal,
            ProgressBarStatus::Indeterminate => TaoProgressState::Indeterminate,
            ProgressBarStatus::Paused => TaoProgressState::Paused,
            ProgressBarStatus::Error => TaoProgressState::Error,
        };
        Self(state)
    }
}

pub struct ProgressBarStateWrapper(pub TaoProgressBarState);

impl From<ProgressBarState> for ProgressBarStateWrapper {
    fn from(progress_state: ProgressBarState) -> Self {
        Self(TaoProgressBarState {
            progress: progress_state.progress,
            state: progress_state
                .status
                .map(|state| ProgressStateWrapper::from(state).0),
            desktop_filename: progress_state.desktop_filename,
        })
    }
}
