#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchId {
    League,
    Stats,
    Goalies,
    Depth,
    Team,
    Player,
    Scores,
    Schedule,
    Transactions,
    Playoffs,
    Game,
    Favorites,
    Watchlist,
    Groups,
    Fantasy,
    Simulate,
    Poach,
    Reports,
    Records,
    Career,
    Docs,
    Admin,
}

impl WorkbenchId {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::League => "league",
            Self::Stats => "stats",
            Self::Goalies => "goalies",
            Self::Depth => "depth",
            Self::Team => "team",
            Self::Player => "player",
            Self::Scores => "scores",
            Self::Schedule => "schedule",
            Self::Transactions => "transactions",
            Self::Playoffs => "playoffs",
            Self::Game => "game",
            Self::Favorites => "favorites",
            Self::Watchlist => "watchlist",
            Self::Groups => "groups",
            Self::Fantasy => "fantasy",
            Self::Simulate => "simulate",
            Self::Poach => "poach",
            Self::Reports => "reports",
            Self::Records => "records",
            Self::Career => "career",
            Self::Docs => "docs",
            Self::Admin => "admin",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchGroup {
    League,
    Analytics,
    Teams,
    Players,
    Live,
    MyBench,
    Fantasy,
    Reports,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchZone {
    ActivityRail,
    Center,
    LeftPane,
    RightPane,
    TopRibbon,
    BottomStatus,
    Overlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchDocumentKind {
    Main,
    Drilldown,
    Context,
    Admin,
    Docs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchFieldId {
    Workspace,
    Route,
    EntityKind,
    Player,
    Team,
    Game,
    FavoriteGroup,
    WatchStatus,
    AlertType,
    StatKey,
    ReportType,
    SourceState,
    Date,
    GameState,
    Opponent,
    HomeAway,
    Position,
    League,
    Category,
    Availability,
    Sort,
    DataKind,
    MutationResult,
}

impl WorkbenchFieldId {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Route => "route",
            Self::EntityKind => "entity-kind",
            Self::Player => "player",
            Self::Team => "team",
            Self::Game => "game",
            Self::FavoriteGroup => "favorite-group",
            Self::WatchStatus => "watch-status",
            Self::AlertType => "alert-type",
            Self::StatKey => "stat-key",
            Self::ReportType => "report-type",
            Self::SourceState => "source-state",
            Self::Date => "date",
            Self::GameState => "game-state",
            Self::Opponent => "opponent",
            Self::HomeAway => "home-away",
            Self::Position => "position",
            Self::League => "league",
            Self::Category => "category",
            Self::Availability => "availability",
            Self::Sort => "sort",
            Self::DataKind => "data-kind",
            Self::MutationResult => "mutation-result",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchFieldScope {
    Entity,
    Workspace,
    Route,
    Source,
    Mutation,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchValueKind {
    Bool,
    Integer,
    Decimal,
    Text,
    Enum,
    Date,
    EntityRef,
    Route,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchFieldSource {
    ViewModel,
    RouteSummary,
    Catalog,
    CommandResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchFieldOperator {
    Equals,
    Range,
    In,
    Search,
    Sort,
    Group,
    Pin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchFieldSummary {
    None,
    Count,
    MinMax,
    Latest,
    Status,
    Sparkline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkbenchField {
    pub id: WorkbenchFieldId,
    pub label: &'static str,
    pub scope: WorkbenchFieldScope,
    pub value_kind: WorkbenchValueKind,
    pub source: WorkbenchFieldSource,
    pub operators: &'static [WorkbenchFieldOperator],
    pub summary: WorkbenchFieldSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchPaneModelId {
    ActivityCatalog,
    FavoritesNavigator,
    WatchlistQueue,
    GroupsNavigator,
    SavedQueries,
    RecentEntities,
    FantasyRoster,
    ScheduleInspector,
    PlayerInspector,
    TeamInspector,
    StatFilterInspector,
    GoalieInspector,
    GameInspector,
    ScoringTrend,
    OutlookSummary,
    PoachFilters,
    FantasySimulation,
    RecordsInspector,
    CareerCohort,
    DataSourceInspector,
    DocsHelp,
}

impl WorkbenchPaneModelId {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::ActivityCatalog => "activity-catalog",
            Self::FavoritesNavigator => "favorites-navigator",
            Self::WatchlistQueue => "watchlist-queue",
            Self::GroupsNavigator => "groups-navigator",
            Self::SavedQueries => "saved-queries",
            Self::RecentEntities => "recent-entities",
            Self::FantasyRoster => "fantasy-roster",
            Self::ScheduleInspector => "schedule-inspector",
            Self::PlayerInspector => "player-inspector",
            Self::TeamInspector => "team-inspector",
            Self::StatFilterInspector => "stat-filter-inspector",
            Self::GoalieInspector => "goalie-inspector",
            Self::GameInspector => "game-inspector",
            Self::ScoringTrend => "scoring-trend",
            Self::OutlookSummary => "outlook-summary",
            Self::PoachFilters => "poach-filters",
            Self::FantasySimulation => "fantasy-simulation",
            Self::RecordsInspector => "records-inspector",
            Self::CareerCohort => "career-cohort",
            Self::DataSourceInspector => "data-source-inspector",
            Self::DocsHelp => "docs-help",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchPaneKind {
    Navigator,
    Inspector,
    Filter,
    Summary,
    Timeline,
    Compare,
    Queue,
    SourceState,
    ActionStatus,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkbenchPaneModel {
    pub id: WorkbenchPaneModelId,
    pub label: &'static str,
    pub kind: WorkbenchPaneKind,
    pub supported_zones: &'static [WorkbenchZone],
    pub fields: &'static [WorkbenchFieldId],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchSurface {
    Tui,
    Web,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchPaneBindingId {
    FavoritesLeft,
    WatchlistLeft,
    GroupsLeft,
    SavedQueriesLeft,
    RecentEntitiesLeft,
    FantasyRosterLeft,
    ScheduleRight,
    PlayerRight,
    TeamRight,
    StatFilterRight,
    GoalieRight,
    GameRight,
    ScoringTrendRight,
    OutlookSummaryRight,
    PoachFiltersRight,
    FantasySimulationRight,
    RecordsRight,
    CareerRight,
    DataSourceRight,
    DocsHelpRight,
}

impl WorkbenchPaneBindingId {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::FavoritesLeft => "favorites-left",
            Self::WatchlistLeft => "watchlist-left",
            Self::GroupsLeft => "groups-left",
            Self::SavedQueriesLeft => "saved-queries-left",
            Self::RecentEntitiesLeft => "recent-entities-left",
            Self::FantasyRosterLeft => "fantasy-roster-left",
            Self::ScheduleRight => "schedule-right",
            Self::PlayerRight => "player-right",
            Self::TeamRight => "team-right",
            Self::StatFilterRight => "stat-filter-right",
            Self::GoalieRight => "goalie-right",
            Self::GameRight => "game-right",
            Self::ScoringTrendRight => "scoring-trend-right",
            Self::OutlookSummaryRight => "outlook-summary-right",
            Self::PoachFiltersRight => "poach-filters-right",
            Self::FantasySimulationRight => "fantasy-simulation-right",
            Self::RecordsRight => "records-right",
            Self::CareerRight => "career-right",
            Self::DataSourceRight => "data-source-right",
            Self::DocsHelpRight => "docs-help-right",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchPaneInteraction {
    ReadOnly,
    LocalState,
    PostBackedActionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkbenchPaneBinding {
    pub id: WorkbenchPaneBindingId,
    pub label: &'static str,
    pub pane_model: WorkbenchPaneModelId,
    pub zone: WorkbenchZone,
    pub supported_surfaces: &'static [WorkbenchSurface],
    pub fields: &'static [WorkbenchFieldId],
    pub priority: u8,
    pub interaction: WorkbenchPaneInteraction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkbenchEntry {
    pub id: WorkbenchId,
    pub label: &'static str,
    pub group: WorkbenchGroup,
    pub aliases: &'static [&'static str],
    pub default_zone: WorkbenchZone,
    pub document_kind: WorkbenchDocumentKind,
    pub pane_models: &'static [WorkbenchPaneModelId],
    pub fields: &'static [WorkbenchFieldId],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchRibbonScope {
    Live,
    ActiveDate,
    Workspace,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchStatusScope {
    Command,
    Selection,
    MutationResult,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchExperienceId {
    TonightBench,
    ScoringRoom,
    TeamRoom,
    FantasyRoom,
    AdminRoom,
}

impl WorkbenchExperienceId {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::TonightBench => "tonight-bench",
            Self::ScoringRoom => "scoring-room",
            Self::TeamRoom => "team-room",
            Self::FantasyRoom => "fantasy-room",
            Self::AdminRoom => "admin-room",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkbenchExperience {
    pub id: WorkbenchExperienceId,
    pub label: &'static str,
    pub supported_surfaces: &'static [WorkbenchSurface],
    pub center: WorkbenchId,
    pub left_pane: Option<WorkbenchPaneBindingId>,
    pub right_pane: Option<WorkbenchPaneBindingId>,
    pub ribbon_scope: WorkbenchRibbonScope,
    pub status_scope: WorkbenchStatusScope,
    pub fields: &'static [WorkbenchFieldId],
}

pub const WORKBENCH_FIELDS: &[WorkbenchField] = &[
    field(
        WorkbenchFieldId::Workspace,
        "Workspace",
        WorkbenchFieldScope::Workspace,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::Catalog,
        &[WorkbenchFieldOperator::Equals, WorkbenchFieldOperator::Pin],
        WorkbenchFieldSummary::None,
    ),
    field(
        WorkbenchFieldId::Route,
        "Route",
        WorkbenchFieldScope::Route,
        WorkbenchValueKind::Route,
        WorkbenchFieldSource::RouteSummary,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Search,
        ],
        WorkbenchFieldSummary::None,
    ),
    field(
        WorkbenchFieldId::EntityKind,
        "Entity kind",
        WorkbenchFieldScope::Entity,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Group,
        ],
        WorkbenchFieldSummary::Count,
    ),
    field(
        WorkbenchFieldId::Player,
        "Player",
        WorkbenchFieldScope::Entity,
        WorkbenchValueKind::EntityRef,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Search,
        ],
        WorkbenchFieldSummary::None,
    ),
    field(
        WorkbenchFieldId::Team,
        "Team",
        WorkbenchFieldScope::Entity,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::In,
            WorkbenchFieldOperator::Group,
        ],
        WorkbenchFieldSummary::Count,
    ),
    field(
        WorkbenchFieldId::Game,
        "Game",
        WorkbenchFieldScope::Entity,
        WorkbenchValueKind::EntityRef,
        WorkbenchFieldSource::ViewModel,
        &[WorkbenchFieldOperator::Equals],
        WorkbenchFieldSummary::None,
    ),
    field(
        WorkbenchFieldId::FavoriteGroup,
        "Favorite group",
        WorkbenchFieldScope::Entity,
        WorkbenchValueKind::Text,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Search,
            WorkbenchFieldOperator::Group,
        ],
        WorkbenchFieldSummary::Count,
    ),
    field(
        WorkbenchFieldId::WatchStatus,
        "Watch status",
        WorkbenchFieldScope::Entity,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Group,
        ],
        WorkbenchFieldSummary::Status,
    ),
    field(
        WorkbenchFieldId::AlertType,
        "Alert type",
        WorkbenchFieldScope::Entity,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Group,
        ],
        WorkbenchFieldSummary::Count,
    ),
    field(
        WorkbenchFieldId::StatKey,
        "Stat key",
        WorkbenchFieldScope::Workspace,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Search,
            WorkbenchFieldOperator::Sort,
        ],
        WorkbenchFieldSummary::None,
    ),
    field(
        WorkbenchFieldId::ReportType,
        "Report type",
        WorkbenchFieldScope::Workspace,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Group,
        ],
        WorkbenchFieldSummary::None,
    ),
    field(
        WorkbenchFieldId::SourceState,
        "Source state",
        WorkbenchFieldScope::Source,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Group,
        ],
        WorkbenchFieldSummary::Status,
    ),
    field(
        WorkbenchFieldId::Date,
        "Date",
        WorkbenchFieldScope::Workspace,
        WorkbenchValueKind::Date,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Range,
        ],
        WorkbenchFieldSummary::Latest,
    ),
    field(
        WorkbenchFieldId::GameState,
        "Game state",
        WorkbenchFieldScope::Workspace,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Group,
        ],
        WorkbenchFieldSummary::Status,
    ),
    field(
        WorkbenchFieldId::Opponent,
        "Opponent",
        WorkbenchFieldScope::Entity,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Group,
        ],
        WorkbenchFieldSummary::None,
    ),
    field(
        WorkbenchFieldId::HomeAway,
        "Home/away",
        WorkbenchFieldScope::Workspace,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Group,
        ],
        WorkbenchFieldSummary::None,
    ),
    field(
        WorkbenchFieldId::Position,
        "Position",
        WorkbenchFieldScope::Entity,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Group,
        ],
        WorkbenchFieldSummary::Count,
    ),
    field(
        WorkbenchFieldId::League,
        "League",
        WorkbenchFieldScope::Entity,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Group,
        ],
        WorkbenchFieldSummary::Count,
    ),
    field(
        WorkbenchFieldId::Category,
        "Category",
        WorkbenchFieldScope::Workspace,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Group,
        ],
        WorkbenchFieldSummary::Count,
    ),
    field(
        WorkbenchFieldId::Availability,
        "Availability",
        WorkbenchFieldScope::Workspace,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Group,
        ],
        WorkbenchFieldSummary::Status,
    ),
    field(
        WorkbenchFieldId::Sort,
        "Sort",
        WorkbenchFieldScope::Workspace,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::ViewModel,
        &[WorkbenchFieldOperator::Sort],
        WorkbenchFieldSummary::None,
    ),
    field(
        WorkbenchFieldId::DataKind,
        "Data kind",
        WorkbenchFieldScope::System,
        WorkbenchValueKind::Enum,
        WorkbenchFieldSource::ViewModel,
        &[
            WorkbenchFieldOperator::Equals,
            WorkbenchFieldOperator::Group,
        ],
        WorkbenchFieldSummary::Status,
    ),
    field(
        WorkbenchFieldId::MutationResult,
        "Mutation result",
        WorkbenchFieldScope::Mutation,
        WorkbenchValueKind::Text,
        WorkbenchFieldSource::CommandResult,
        &[WorkbenchFieldOperator::Equals],
        WorkbenchFieldSummary::Latest,
    ),
];

pub const WORKBENCH_PANE_MODELS: &[WorkbenchPaneModel] = &[
    pane_model(
        WorkbenchPaneModelId::ActivityCatalog,
        "Activity catalog",
        WorkbenchPaneKind::Navigator,
        &[WorkbenchZone::ActivityRail, WorkbenchZone::Overlay],
        &[WorkbenchFieldId::Workspace],
    ),
    pane_model(
        WorkbenchPaneModelId::FavoritesNavigator,
        "Favorites navigator",
        WorkbenchPaneKind::Navigator,
        &[WorkbenchZone::LeftPane, WorkbenchZone::Center],
        &[
            WorkbenchFieldId::FavoriteGroup,
            WorkbenchFieldId::EntityKind,
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::SourceState,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::WatchlistQueue,
        "Watchlist queue",
        WorkbenchPaneKind::Queue,
        &[WorkbenchZone::LeftPane, WorkbenchZone::RightPane],
        &[
            WorkbenchFieldId::WatchStatus,
            WorkbenchFieldId::AlertType,
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::GroupsNavigator,
        "Groups navigator",
        WorkbenchPaneKind::Navigator,
        &[WorkbenchZone::LeftPane],
        &[
            WorkbenchFieldId::FavoriteGroup,
            WorkbenchFieldId::EntityKind,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::SavedQueries,
        "Saved queries",
        WorkbenchPaneKind::Filter,
        &[WorkbenchZone::LeftPane, WorkbenchZone::Overlay],
        &[
            WorkbenchFieldId::Workspace,
            WorkbenchFieldId::StatKey,
            WorkbenchFieldId::Route,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::RecentEntities,
        "Recent entities",
        WorkbenchPaneKind::Navigator,
        &[WorkbenchZone::LeftPane, WorkbenchZone::Overlay],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Game,
            WorkbenchFieldId::Route,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::FantasyRoster,
        "Fantasy roster",
        WorkbenchPaneKind::Summary,
        &[WorkbenchZone::LeftPane, WorkbenchZone::Center],
        &[
            WorkbenchFieldId::Category,
            WorkbenchFieldId::Position,
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::ScheduleInspector,
        "Schedule inspector",
        WorkbenchPaneKind::Timeline,
        &[WorkbenchZone::RightPane, WorkbenchZone::Center],
        &[
            WorkbenchFieldId::Date,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Opponent,
            WorkbenchFieldId::HomeAway,
            WorkbenchFieldId::GameState,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::PlayerInspector,
        "Player inspector",
        WorkbenchPaneKind::Inspector,
        &[WorkbenchZone::RightPane],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Position,
            WorkbenchFieldId::League,
            WorkbenchFieldId::SourceState,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::TeamInspector,
        "Team inspector",
        WorkbenchPaneKind::Compare,
        &[WorkbenchZone::RightPane],
        &[
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Position,
            WorkbenchFieldId::Opponent,
            WorkbenchFieldId::SourceState,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::StatFilterInspector,
        "Stat/filter inspector",
        WorkbenchPaneKind::Filter,
        &[WorkbenchZone::RightPane, WorkbenchZone::Overlay],
        &[
            WorkbenchFieldId::StatKey,
            WorkbenchFieldId::ReportType,
            WorkbenchFieldId::Sort,
            WorkbenchFieldId::SourceState,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::GoalieInspector,
        "Goalie inspector",
        WorkbenchPaneKind::Inspector,
        &[WorkbenchZone::RightPane],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::StatKey,
            WorkbenchFieldId::SourceState,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::GameInspector,
        "Game inspector",
        WorkbenchPaneKind::Timeline,
        &[WorkbenchZone::RightPane],
        &[
            WorkbenchFieldId::Game,
            WorkbenchFieldId::GameState,
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::SourceState,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::ScoringTrend,
        "Scoring trend",
        WorkbenchPaneKind::Summary,
        &[WorkbenchZone::RightPane],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::StatKey,
            WorkbenchFieldId::SourceState,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::OutlookSummary,
        "Outlook summary",
        WorkbenchPaneKind::Summary,
        &[WorkbenchZone::RightPane],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::SourceState,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::PoachFilters,
        "Poach filters",
        WorkbenchPaneKind::Filter,
        &[WorkbenchZone::RightPane, WorkbenchZone::Overlay],
        &[
            WorkbenchFieldId::Availability,
            WorkbenchFieldId::Position,
            WorkbenchFieldId::Category,
            WorkbenchFieldId::WatchStatus,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::FantasySimulation,
        "Fantasy simulation",
        WorkbenchPaneKind::ActionStatus,
        &[WorkbenchZone::RightPane, WorkbenchZone::Center],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Category,
            WorkbenchFieldId::MutationResult,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::RecordsInspector,
        "Records inspector",
        WorkbenchPaneKind::Inspector,
        &[WorkbenchZone::RightPane],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Opponent,
            WorkbenchFieldId::StatKey,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::CareerCohort,
        "Career cohort",
        WorkbenchPaneKind::Compare,
        &[WorkbenchZone::RightPane, WorkbenchZone::Center],
        &[
            WorkbenchFieldId::League,
            WorkbenchFieldId::Date,
            WorkbenchFieldId::Sort,
            WorkbenchFieldId::Player,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::DataSourceInspector,
        "Data/source inspector",
        WorkbenchPaneKind::SourceState,
        &[WorkbenchZone::RightPane, WorkbenchZone::Overlay],
        &[
            WorkbenchFieldId::DataKind,
            WorkbenchFieldId::SourceState,
            WorkbenchFieldId::MutationResult,
        ],
    ),
    pane_model(
        WorkbenchPaneModelId::DocsHelp,
        "Docs/help",
        WorkbenchPaneKind::Help,
        &[WorkbenchZone::RightPane, WorkbenchZone::Overlay],
        &[WorkbenchFieldId::Workspace, WorkbenchFieldId::Route],
    ),
];

pub const WORKBENCH_PANE_BINDINGS: &[WorkbenchPaneBinding] = &[
    pane_binding(
        WorkbenchPaneBindingId::FavoritesLeft,
        "Favorites",
        WorkbenchPaneModelId::FavoritesNavigator,
        WorkbenchZone::LeftPane,
        &[WorkbenchSurface::Tui, WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::FavoriteGroup,
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::SourceState,
        ],
        10,
        WorkbenchPaneInteraction::ReadOnly,
    ),
    pane_binding(
        WorkbenchPaneBindingId::WatchlistLeft,
        "Watchlist queue",
        WorkbenchPaneModelId::WatchlistQueue,
        WorkbenchZone::LeftPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::WatchStatus,
            WorkbenchFieldId::AlertType,
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
        ],
        20,
        WorkbenchPaneInteraction::ReadOnly,
    ),
    pane_binding(
        WorkbenchPaneBindingId::GroupsLeft,
        "Groups",
        WorkbenchPaneModelId::GroupsNavigator,
        WorkbenchZone::LeftPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::FavoriteGroup,
            WorkbenchFieldId::EntityKind,
        ],
        30,
        WorkbenchPaneInteraction::ReadOnly,
    ),
    pane_binding(
        WorkbenchPaneBindingId::SavedQueriesLeft,
        "Saved queries",
        WorkbenchPaneModelId::SavedQueries,
        WorkbenchZone::LeftPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::Workspace,
            WorkbenchFieldId::StatKey,
            WorkbenchFieldId::Route,
        ],
        40,
        WorkbenchPaneInteraction::LocalState,
    ),
    pane_binding(
        WorkbenchPaneBindingId::RecentEntitiesLeft,
        "Recent entities",
        WorkbenchPaneModelId::RecentEntities,
        WorkbenchZone::LeftPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Game,
            WorkbenchFieldId::Route,
        ],
        50,
        WorkbenchPaneInteraction::LocalState,
    ),
    pane_binding(
        WorkbenchPaneBindingId::FantasyRosterLeft,
        "Fantasy roster",
        WorkbenchPaneModelId::FantasyRoster,
        WorkbenchZone::LeftPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::Category,
            WorkbenchFieldId::Position,
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
        ],
        60,
        WorkbenchPaneInteraction::ReadOnly,
    ),
    pane_binding(
        WorkbenchPaneBindingId::ScheduleRight,
        "Schedule",
        WorkbenchPaneModelId::ScheduleInspector,
        WorkbenchZone::RightPane,
        &[WorkbenchSurface::Tui, WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::Date,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Opponent,
            WorkbenchFieldId::GameState,
        ],
        10,
        WorkbenchPaneInteraction::ReadOnly,
    ),
    pane_binding(
        WorkbenchPaneBindingId::PlayerRight,
        "Player inspector",
        WorkbenchPaneModelId::PlayerInspector,
        WorkbenchZone::RightPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Position,
            WorkbenchFieldId::SourceState,
        ],
        20,
        WorkbenchPaneInteraction::ReadOnly,
    ),
    pane_binding(
        WorkbenchPaneBindingId::TeamRight,
        "Team inspector",
        WorkbenchPaneModelId::TeamInspector,
        WorkbenchZone::RightPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Position,
            WorkbenchFieldId::Opponent,
            WorkbenchFieldId::SourceState,
        ],
        30,
        WorkbenchPaneInteraction::ReadOnly,
    ),
    pane_binding(
        WorkbenchPaneBindingId::StatFilterRight,
        "Stat/filter inspector",
        WorkbenchPaneModelId::StatFilterInspector,
        WorkbenchZone::RightPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::StatKey,
            WorkbenchFieldId::ReportType,
            WorkbenchFieldId::Sort,
            WorkbenchFieldId::SourceState,
        ],
        40,
        WorkbenchPaneInteraction::ReadOnly,
    ),
    pane_binding(
        WorkbenchPaneBindingId::GoalieRight,
        "Goalie inspector",
        WorkbenchPaneModelId::GoalieInspector,
        WorkbenchZone::RightPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::StatKey,
            WorkbenchFieldId::SourceState,
        ],
        50,
        WorkbenchPaneInteraction::ReadOnly,
    ),
    pane_binding(
        WorkbenchPaneBindingId::GameRight,
        "Game inspector",
        WorkbenchPaneModelId::GameInspector,
        WorkbenchZone::RightPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::Game,
            WorkbenchFieldId::GameState,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::SourceState,
        ],
        60,
        WorkbenchPaneInteraction::ReadOnly,
    ),
    pane_binding(
        WorkbenchPaneBindingId::ScoringTrendRight,
        "Scoring trend",
        WorkbenchPaneModelId::ScoringTrend,
        WorkbenchZone::RightPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::StatKey,
            WorkbenchFieldId::SourceState,
        ],
        70,
        WorkbenchPaneInteraction::ReadOnly,
    ),
    pane_binding(
        WorkbenchPaneBindingId::OutlookSummaryRight,
        "Outlook summary",
        WorkbenchPaneModelId::OutlookSummary,
        WorkbenchZone::RightPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::SourceState,
        ],
        80,
        WorkbenchPaneInteraction::ReadOnly,
    ),
    pane_binding(
        WorkbenchPaneBindingId::PoachFiltersRight,
        "Poach filters",
        WorkbenchPaneModelId::PoachFilters,
        WorkbenchZone::RightPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::Availability,
            WorkbenchFieldId::Position,
            WorkbenchFieldId::Category,
            WorkbenchFieldId::WatchStatus,
        ],
        90,
        WorkbenchPaneInteraction::ReadOnly,
    ),
    pane_binding(
        WorkbenchPaneBindingId::FantasySimulationRight,
        "Fantasy simulation",
        WorkbenchPaneModelId::FantasySimulation,
        WorkbenchZone::RightPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Category,
            WorkbenchFieldId::MutationResult,
        ],
        100,
        WorkbenchPaneInteraction::PostBackedActionStatus,
    ),
    pane_binding(
        WorkbenchPaneBindingId::RecordsRight,
        "Records inspector",
        WorkbenchPaneModelId::RecordsInspector,
        WorkbenchZone::RightPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Opponent,
            WorkbenchFieldId::StatKey,
        ],
        110,
        WorkbenchPaneInteraction::ReadOnly,
    ),
    pane_binding(
        WorkbenchPaneBindingId::CareerRight,
        "Career cohort",
        WorkbenchPaneModelId::CareerCohort,
        WorkbenchZone::RightPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::League,
            WorkbenchFieldId::Date,
            WorkbenchFieldId::Sort,
            WorkbenchFieldId::Player,
        ],
        120,
        WorkbenchPaneInteraction::ReadOnly,
    ),
    pane_binding(
        WorkbenchPaneBindingId::DataSourceRight,
        "Data/source",
        WorkbenchPaneModelId::DataSourceInspector,
        WorkbenchZone::RightPane,
        &[WorkbenchSurface::Web],
        &[
            WorkbenchFieldId::DataKind,
            WorkbenchFieldId::SourceState,
            WorkbenchFieldId::MutationResult,
        ],
        130,
        WorkbenchPaneInteraction::PostBackedActionStatus,
    ),
    pane_binding(
        WorkbenchPaneBindingId::DocsHelpRight,
        "Docs/help",
        WorkbenchPaneModelId::DocsHelp,
        WorkbenchZone::RightPane,
        &[WorkbenchSurface::Web],
        &[WorkbenchFieldId::Workspace, WorkbenchFieldId::Route],
        140,
        WorkbenchPaneInteraction::ReadOnly,
    ),
];

pub const WORKBENCH_CATALOG: &[WorkbenchEntry] = &[
    entry(
        WorkbenchId::League,
        "League",
        WorkbenchGroup::League,
        &["home"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Main,
        &[
            WorkbenchPaneModelId::ScheduleInspector,
            WorkbenchPaneModelId::DataSourceInspector,
        ],
        &[WorkbenchFieldId::Team, WorkbenchFieldId::SourceState],
    ),
    entry(
        WorkbenchId::Stats,
        "Stats",
        WorkbenchGroup::Analytics,
        &["leaders", "query"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Main,
        &[
            WorkbenchPaneModelId::SavedQueries,
            WorkbenchPaneModelId::StatFilterInspector,
        ],
        &[
            WorkbenchFieldId::StatKey,
            WorkbenchFieldId::Position,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Sort,
        ],
    ),
    entry(
        WorkbenchId::Goalies,
        "Goalies",
        WorkbenchGroup::Analytics,
        &["g"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Main,
        &[WorkbenchPaneModelId::GoalieInspector],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::StatKey,
        ],
    ),
    entry(
        WorkbenchId::Depth,
        "Depth",
        WorkbenchGroup::Teams,
        &["depth-chart"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Main,
        &[WorkbenchPaneModelId::TeamInspector],
        &[WorkbenchFieldId::Team, WorkbenchFieldId::Position],
    ),
    entry(
        WorkbenchId::Team,
        "Team",
        WorkbenchGroup::Teams,
        &["club"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Drilldown,
        &[
            WorkbenchPaneModelId::TeamInspector,
            WorkbenchPaneModelId::ScheduleInspector,
        ],
        &[
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Position,
            WorkbenchFieldId::Opponent,
        ],
    ),
    entry(
        WorkbenchId::Player,
        "Player",
        WorkbenchGroup::Players,
        &["skater"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Drilldown,
        &[
            WorkbenchPaneModelId::PlayerInspector,
            WorkbenchPaneModelId::RecordsInspector,
        ],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Position,
        ],
    ),
    entry(
        WorkbenchId::Scores,
        "Scores",
        WorkbenchGroup::Live,
        &["tonight"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Main,
        &[
            WorkbenchPaneModelId::FavoritesNavigator,
            WorkbenchPaneModelId::ScheduleInspector,
        ],
        &[
            WorkbenchFieldId::Date,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::GameState,
            WorkbenchFieldId::SourceState,
        ],
    ),
    entry(
        WorkbenchId::Schedule,
        "Schedule",
        WorkbenchGroup::Live,
        &["calendar"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Main,
        &[WorkbenchPaneModelId::ScheduleInspector],
        &[
            WorkbenchFieldId::Date,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Opponent,
            WorkbenchFieldId::HomeAway,
        ],
    ),
    entry(
        WorkbenchId::Transactions,
        "Transactions",
        WorkbenchGroup::Live,
        &["tx", "txs"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Main,
        &[WorkbenchPaneModelId::RecentEntities],
        &[WorkbenchFieldId::Date, WorkbenchFieldId::Team],
    ),
    entry(
        WorkbenchId::Playoffs,
        "Playoffs",
        WorkbenchGroup::Live,
        &["bracket"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Main,
        &[WorkbenchPaneModelId::GameInspector],
        &[
            WorkbenchFieldId::Game,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Opponent,
        ],
    ),
    entry(
        WorkbenchId::Game,
        "Game",
        WorkbenchGroup::Live,
        &["box"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Drilldown,
        &[WorkbenchPaneModelId::GameInspector],
        &[
            WorkbenchFieldId::Game,
            WorkbenchFieldId::GameState,
            WorkbenchFieldId::SourceState,
        ],
    ),
    entry(
        WorkbenchId::Favorites,
        "Favorites",
        WorkbenchGroup::MyBench,
        &["faves"],
        WorkbenchZone::LeftPane,
        WorkbenchDocumentKind::Context,
        &[WorkbenchPaneModelId::FavoritesNavigator],
        &[
            WorkbenchFieldId::FavoriteGroup,
            WorkbenchFieldId::EntityKind,
            WorkbenchFieldId::SourceState,
        ],
    ),
    entry(
        WorkbenchId::Watchlist,
        "Watchlist",
        WorkbenchGroup::MyBench,
        &["watch"],
        WorkbenchZone::LeftPane,
        WorkbenchDocumentKind::Context,
        &[WorkbenchPaneModelId::WatchlistQueue],
        &[
            WorkbenchFieldId::WatchStatus,
            WorkbenchFieldId::AlertType,
            WorkbenchFieldId::Player,
        ],
    ),
    entry(
        WorkbenchId::Groups,
        "Groups",
        WorkbenchGroup::MyBench,
        &["group"],
        WorkbenchZone::LeftPane,
        WorkbenchDocumentKind::Context,
        &[WorkbenchPaneModelId::GroupsNavigator],
        &[
            WorkbenchFieldId::FavoriteGroup,
            WorkbenchFieldId::EntityKind,
        ],
    ),
    entry(
        WorkbenchId::Fantasy,
        "Fantasy",
        WorkbenchGroup::Fantasy,
        &["roster", "gaps"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Main,
        &[
            WorkbenchPaneModelId::FantasyRoster,
            WorkbenchPaneModelId::PoachFilters,
        ],
        &[
            WorkbenchFieldId::Category,
            WorkbenchFieldId::Position,
            WorkbenchFieldId::Player,
        ],
    ),
    entry(
        WorkbenchId::Simulate,
        "Simulation",
        WorkbenchGroup::Fantasy,
        &["sim"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Main,
        &[WorkbenchPaneModelId::FantasySimulation],
        &[WorkbenchFieldId::Player, WorkbenchFieldId::Category],
    ),
    entry(
        WorkbenchId::Poach,
        "Poach",
        WorkbenchGroup::Fantasy,
        &["waivers"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Main,
        &[WorkbenchPaneModelId::PoachFilters],
        &[
            WorkbenchFieldId::Availability,
            WorkbenchFieldId::Position,
            WorkbenchFieldId::Category,
        ],
    ),
    entry(
        WorkbenchId::Reports,
        "Reports",
        WorkbenchGroup::Reports,
        &["report"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Main,
        &[WorkbenchPaneModelId::StatFilterInspector],
        &[WorkbenchFieldId::ReportType, WorkbenchFieldId::SourceState],
    ),
    entry(
        WorkbenchId::Records,
        "Records",
        WorkbenchGroup::Reports,
        &["record"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Drilldown,
        &[WorkbenchPaneModelId::RecordsInspector],
        &[
            WorkbenchFieldId::Player,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::StatKey,
        ],
    ),
    entry(
        WorkbenchId::Career,
        "Career cohorts",
        WorkbenchGroup::Players,
        &["cohort"],
        WorkbenchZone::Center,
        WorkbenchDocumentKind::Main,
        &[WorkbenchPaneModelId::CareerCohort],
        &[
            WorkbenchFieldId::League,
            WorkbenchFieldId::Sort,
            WorkbenchFieldId::Player,
        ],
    ),
    entry(
        WorkbenchId::Docs,
        "Docs",
        WorkbenchGroup::System,
        &["help"],
        WorkbenchZone::Overlay,
        WorkbenchDocumentKind::Docs,
        &[WorkbenchPaneModelId::DocsHelp],
        &[WorkbenchFieldId::Workspace, WorkbenchFieldId::Route],
    ),
    entry(
        WorkbenchId::Admin,
        "Admin",
        WorkbenchGroup::System,
        &["fetch"],
        WorkbenchZone::Overlay,
        WorkbenchDocumentKind::Admin,
        &[WorkbenchPaneModelId::DataSourceInspector],
        &[
            WorkbenchFieldId::DataKind,
            WorkbenchFieldId::SourceState,
            WorkbenchFieldId::MutationResult,
        ],
    ),
];

pub const WORKBENCH_EXPERIENCES: &[WorkbenchExperience] = &[
    experience(
        WorkbenchExperienceId::TonightBench,
        "Tonight bench",
        &[WorkbenchSurface::Tui, WorkbenchSurface::Web],
        WorkbenchId::Scores,
        Some(WorkbenchPaneBindingId::FavoritesLeft),
        Some(WorkbenchPaneBindingId::ScheduleRight),
        WorkbenchRibbonScope::Live,
        WorkbenchStatusScope::Command,
        &[
            WorkbenchFieldId::Date,
            WorkbenchFieldId::FavoriteGroup,
            WorkbenchFieldId::GameState,
            WorkbenchFieldId::SourceState,
        ],
    ),
    experience(
        WorkbenchExperienceId::ScoringRoom,
        "Scoring room",
        &[WorkbenchSurface::Web],
        WorkbenchId::Stats,
        Some(WorkbenchPaneBindingId::SavedQueriesLeft),
        Some(WorkbenchPaneBindingId::StatFilterRight),
        WorkbenchRibbonScope::Workspace,
        WorkbenchStatusScope::Selection,
        &[
            WorkbenchFieldId::StatKey,
            WorkbenchFieldId::ReportType,
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Position,
        ],
    ),
    experience(
        WorkbenchExperienceId::TeamRoom,
        "Team room",
        &[WorkbenchSurface::Web],
        WorkbenchId::Depth,
        Some(WorkbenchPaneBindingId::RecentEntitiesLeft),
        Some(WorkbenchPaneBindingId::TeamRight),
        WorkbenchRibbonScope::ActiveDate,
        WorkbenchStatusScope::Selection,
        &[
            WorkbenchFieldId::Team,
            WorkbenchFieldId::Position,
            WorkbenchFieldId::Opponent,
        ],
    ),
    experience(
        WorkbenchExperienceId::FantasyRoom,
        "Fantasy room",
        &[WorkbenchSurface::Web],
        WorkbenchId::Fantasy,
        Some(WorkbenchPaneBindingId::FantasyRosterLeft),
        Some(WorkbenchPaneBindingId::PoachFiltersRight),
        WorkbenchRibbonScope::Workspace,
        WorkbenchStatusScope::MutationResult,
        &[
            WorkbenchFieldId::Category,
            WorkbenchFieldId::Availability,
            WorkbenchFieldId::Position,
        ],
    ),
    experience(
        WorkbenchExperienceId::AdminRoom,
        "Admin room",
        &[WorkbenchSurface::Web],
        WorkbenchId::Admin,
        Some(WorkbenchPaneBindingId::WatchlistLeft),
        Some(WorkbenchPaneBindingId::DataSourceRight),
        WorkbenchRibbonScope::System,
        WorkbenchStatusScope::System,
        &[
            WorkbenchFieldId::DataKind,
            WorkbenchFieldId::SourceState,
            WorkbenchFieldId::MutationResult,
        ],
    ),
];

pub fn workbench_entry(id: WorkbenchId) -> Option<&'static WorkbenchEntry> {
    WORKBENCH_CATALOG.iter().find(|entry| entry.id == id)
}

pub fn workbench_field(id: WorkbenchFieldId) -> Option<&'static WorkbenchField> {
    WORKBENCH_FIELDS.iter().find(|field| field.id == id)
}

pub fn workbench_pane_model(id: WorkbenchPaneModelId) -> Option<&'static WorkbenchPaneModel> {
    WORKBENCH_PANE_MODELS.iter().find(|pane| pane.id == id)
}

pub fn workbench_pane_binding(id: WorkbenchPaneBindingId) -> Option<&'static WorkbenchPaneBinding> {
    WORKBENCH_PANE_BINDINGS
        .iter()
        .find(|binding| binding.id == id)
}

pub fn workbench_experience(id: WorkbenchExperienceId) -> Option<&'static WorkbenchExperience> {
    WORKBENCH_EXPERIENCES
        .iter()
        .find(|experience| experience.id == id)
}

const fn field(
    id: WorkbenchFieldId,
    label: &'static str,
    scope: WorkbenchFieldScope,
    value_kind: WorkbenchValueKind,
    source: WorkbenchFieldSource,
    operators: &'static [WorkbenchFieldOperator],
    summary: WorkbenchFieldSummary,
) -> WorkbenchField {
    WorkbenchField {
        id,
        label,
        scope,
        value_kind,
        source,
        operators,
        summary,
    }
}

const fn pane_model(
    id: WorkbenchPaneModelId,
    label: &'static str,
    kind: WorkbenchPaneKind,
    supported_zones: &'static [WorkbenchZone],
    fields: &'static [WorkbenchFieldId],
) -> WorkbenchPaneModel {
    WorkbenchPaneModel {
        id,
        label,
        kind,
        supported_zones,
        fields,
    }
}

#[allow(clippy::too_many_arguments)] // static binding row constructor; keeps table rows compact.
const fn pane_binding(
    id: WorkbenchPaneBindingId,
    label: &'static str,
    pane_model: WorkbenchPaneModelId,
    zone: WorkbenchZone,
    supported_surfaces: &'static [WorkbenchSurface],
    fields: &'static [WorkbenchFieldId],
    priority: u8,
    interaction: WorkbenchPaneInteraction,
) -> WorkbenchPaneBinding {
    WorkbenchPaneBinding {
        id,
        label,
        pane_model,
        zone,
        supported_surfaces,
        fields,
        priority,
        interaction,
    }
}

#[allow(clippy::too_many_arguments)] // static catalog row constructor; struct literals are noisier.
const fn entry(
    id: WorkbenchId,
    label: &'static str,
    group: WorkbenchGroup,
    aliases: &'static [&'static str],
    default_zone: WorkbenchZone,
    document_kind: WorkbenchDocumentKind,
    pane_models: &'static [WorkbenchPaneModelId],
    fields: &'static [WorkbenchFieldId],
) -> WorkbenchEntry {
    WorkbenchEntry {
        id,
        label,
        group,
        aliases,
        default_zone,
        document_kind,
        pane_models,
        fields,
    }
}

#[allow(clippy::too_many_arguments)] // static experience row constructor; keeps table rows readable.
const fn experience(
    id: WorkbenchExperienceId,
    label: &'static str,
    supported_surfaces: &'static [WorkbenchSurface],
    center: WorkbenchId,
    left_pane: Option<WorkbenchPaneBindingId>,
    right_pane: Option<WorkbenchPaneBindingId>,
    ribbon_scope: WorkbenchRibbonScope,
    status_scope: WorkbenchStatusScope,
    fields: &'static [WorkbenchFieldId],
) -> WorkbenchExperience {
    WorkbenchExperience {
        id,
        label,
        supported_surfaces,
        center,
        left_pane,
        right_pane,
        ribbon_scope,
        status_scope,
        fields,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn l0_workbench_catalog_ids_are_unique() {
        let mut seen = HashSet::new();
        for entry in WORKBENCH_CATALOG {
            assert!(
                seen.insert(entry.id.slug()),
                "duplicate catalog id {}",
                entry.id.slug()
            );
        }
    }

    #[test]
    fn l0_workbench_aliases_are_unique() {
        let mut seen = HashSet::new();
        for entry in WORKBENCH_CATALOG {
            for alias in entry.aliases {
                assert!(seen.insert(*alias), "duplicate workbench alias {alias}");
            }
        }
    }

    #[test]
    fn l0_workbench_fields_are_unique_and_actionable() {
        let mut seen = HashSet::new();
        for field in WORKBENCH_FIELDS {
            assert!(
                seen.insert(field.id.slug()),
                "duplicate workbench field {}",
                field.id.slug()
            );
            assert!(!field.label.is_empty());
            assert!(
                !field.operators.is_empty(),
                "field {} has no operators",
                field.id.slug()
            );
        }
    }

    #[test]
    fn l0_workbench_pane_models_reference_known_fields_and_zones() {
        let field_ids: HashSet<_> = WORKBENCH_FIELDS.iter().map(|field| field.id).collect();
        let mut pane_ids = HashSet::new();

        for pane in WORKBENCH_PANE_MODELS {
            assert!(
                pane_ids.insert(pane.id.slug()),
                "duplicate pane model {}",
                pane.id.slug()
            );
            assert!(!pane.supported_zones.is_empty());
            assert!(!pane.fields.is_empty());
            for field in pane.fields {
                assert!(
                    field_ids.contains(field),
                    "pane {} references unknown field {}",
                    pane.id.slug(),
                    field.slug()
                );
            }
        }
    }

    #[test]
    fn l0_workbench_pane_models_cover_required_kinds() {
        let kinds: HashSet<_> = WORKBENCH_PANE_MODELS.iter().map(|pane| pane.kind).collect();

        for required in [
            WorkbenchPaneKind::Navigator,
            WorkbenchPaneKind::Inspector,
            WorkbenchPaneKind::Filter,
            WorkbenchPaneKind::Summary,
            WorkbenchPaneKind::Timeline,
            WorkbenchPaneKind::SourceState,
            WorkbenchPaneKind::ActionStatus,
        ] {
            assert!(kinds.contains(&required), "missing pane kind {required:?}");
        }
    }

    #[test]
    fn l0_workbench_pane_bindings_reference_known_models_fields_and_zones() {
        let pane_ids: HashSet<_> = WORKBENCH_PANE_MODELS.iter().map(|pane| pane.id).collect();
        let field_ids: HashSet<_> = WORKBENCH_FIELDS.iter().map(|field| field.id).collect();
        let mut binding_ids = HashSet::new();

        for binding in WORKBENCH_PANE_BINDINGS {
            assert!(
                binding_ids.insert(binding.id.slug()),
                "duplicate pane binding {}",
                binding.id.slug()
            );
            assert!(!binding.label.is_empty());
            assert!(
                binding.priority > 0,
                "binding {:?} has no priority",
                binding.id
            );
            assert!(
                !binding.supported_surfaces.is_empty(),
                "binding {} must declare supported surfaces",
                binding.id.slug()
            );
            assert!(
                pane_ids.contains(&binding.pane_model),
                "binding {} references unknown pane model {}",
                binding.id.slug(),
                binding.pane_model.slug()
            );
            let pane = workbench_pane_model(binding.pane_model)
                .expect("binding pane model already checked as present");
            assert!(
                pane.supported_zones.contains(&binding.zone),
                "binding {} places pane {} in unsupported zone {:?}",
                binding.id.slug(),
                binding.pane_model.slug(),
                binding.zone
            );
            assert!(!binding.fields.is_empty());
            for field in binding.fields {
                assert!(
                    field_ids.contains(field),
                    "binding {} references unknown field {}",
                    binding.id.slug(),
                    field.slug()
                );
                assert!(
                    pane.fields.contains(field),
                    "binding {} uses field {} not declared by pane model {}",
                    binding.id.slug(),
                    field.slug(),
                    pane.id.slug()
                );
            }
        }
    }

    #[test]
    fn l0_workbench_pane_bindings_do_not_model_get_mutations() {
        for binding in WORKBENCH_PANE_BINDINGS {
            let pane = workbench_pane_model(binding.pane_model)
                .expect("binding pane model must be present");
            if pane.kind == WorkbenchPaneKind::ActionStatus {
                assert_eq!(
                    binding.interaction,
                    WorkbenchPaneInteraction::PostBackedActionStatus,
                    "action/status binding {} must document POST-backed status semantics",
                    binding.id.slug()
                );
            }
        }
    }

    #[test]
    fn l0_workbench_catalog_references_known_panes_and_fields() {
        let pane_ids: HashSet<_> = WORKBENCH_PANE_MODELS.iter().map(|pane| pane.id).collect();
        let field_ids: HashSet<_> = WORKBENCH_FIELDS.iter().map(|field| field.id).collect();

        for entry in WORKBENCH_CATALOG {
            assert!(!entry.label.is_empty());
            assert!(!entry.pane_models.is_empty());
            assert!(!entry.fields.is_empty());
            for pane in entry.pane_models {
                assert!(
                    pane_ids.contains(pane),
                    "entry {} references unknown pane {}",
                    entry.id.slug(),
                    pane.slug()
                );
            }
            for field in entry.fields {
                assert!(
                    field_ids.contains(field),
                    "entry {} references unknown field {}",
                    entry.id.slug(),
                    field.slug()
                );
            }
        }
    }

    #[test]
    fn l0_workbench_bound_experiences_are_valid_compositions() {
        let entry_ids: HashSet<_> = WORKBENCH_CATALOG.iter().map(|entry| entry.id).collect();
        let binding_ids: HashSet<_> = WORKBENCH_PANE_BINDINGS
            .iter()
            .map(|binding| binding.id)
            .collect();
        let field_ids: HashSet<_> = WORKBENCH_FIELDS.iter().map(|field| field.id).collect();
        let mut experience_ids = HashSet::new();

        for experience in WORKBENCH_EXPERIENCES {
            assert!(
                experience_ids.insert(experience.id.slug()),
                "duplicate experience {}",
                experience.id.slug()
            );
            assert!(
                !experience.supported_surfaces.is_empty(),
                "experience {} must declare supported surfaces",
                experience.id.slug()
            );
            assert!(entry_ids.contains(&experience.center));
            for pane in [experience.left_pane, experience.right_pane]
                .into_iter()
                .flatten()
            {
                assert!(
                    binding_ids.contains(&pane),
                    "experience {} references unknown pane binding {}",
                    experience.id.slug(),
                    pane.slug()
                );
            }
            assert!(!experience.fields.is_empty());
            for field in experience.fields {
                assert!(
                    field_ids.contains(field),
                    "experience {} references unknown field {}",
                    experience.id.slug(),
                    field.slug()
                );
            }
        }
    }
}
