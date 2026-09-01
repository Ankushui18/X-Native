// Navigation Rail - Left-side workflow navigation
// Replaces simple LEFT_TABS with comprehensive workflow selector

use crate::ui::tokens::*;

#[derive(Clone, Copy, PartialEq)]
pub enum NavDestination {
    Files,
    Assets,
    Components,
    Variables,
    Styles,
    Libraries,
    Settings,
}

impl NavDestination {
    pub fn icon(&self) -> &str {
        match self {
            Self::Files => "◇",      // Diamond outline
            Self::Assets => "◆",     // Diamond filled
            Self::Components => "◈",  // Double diamond
            Self::Variables => "◎",   // Concentric circles
            Self::Styles => "◌",      // Circle outline
            Self::Libraries => "⌁",   // Coil/waves
            Self::Settings => "⚙",    // Gear
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Files => "Files",
            Self::Assets => "Assets",
            Self::Components => "Components",
            Self::Variables => "Variables",
            Self::Styles => "Styles",
            Self::Libraries => "Libraries",
            Self::Settings => "Settings",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Files => "Browse files and pages",
            Self::Assets => "Design assets and resources",
            Self::Components => "Component library",
            Self::Variables => "Design tokens and modes",
            Self::Styles => "Text and effect styles",
            Self::Libraries => "Team libraries",
            Self::Settings => "Application preferences",
        }
    }
}

pub struct NavRail {
    pub active: NavDestination,
    pub width: f64,
}

impl NavRail {
    pub fn new() -> Self {
        Self {
            active: NavDestination::Files,
            width: NAV_RAIL_W,
        }
    }

    pub fn set_active(&mut self, destination: NavDestination) {
        self.active = destination;
    }

    /// Get all navigation items
    pub fn items() -> Vec<NavDestination> {
        vec![
            NavDestination::Files,
            NavDestination::Assets,
            NavDestination::Components,
            NavDestination::Variables,
            NavDestination::Styles,
            NavDestination::Libraries,
            NavDestination::Settings,
        ]
    }

    /// Render width including hover area
    pub fn render_width(&self) -> f64 {
        self.width + SPACE_2 * 2.0
    }
}
