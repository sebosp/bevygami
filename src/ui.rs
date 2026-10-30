//! The window around the paper: the feathers toolbar with the lessons menu, the instructions and
//! the steps at the bottom, the crease pattern in a corner, the settings and the help.
use bevy::feathers::containers::{flex_spacer, group, group_body, group_header};
use bevy::feathers::controls::*;
use bevy::feathers::display::{caption, label, label_dim};
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeBorderColor};
use bevy::feathers::tokens as feathers_tokens;
use bevy::input_focus::InputFocus;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::{Activate, ValueChange};

use crate::cpview::{self, CpImages};
use crate::session::{Command, Session, SessionSystems};
use crate::settings::{PaperColor, Settings};
use crate::theme::ThemeMode;
use crate::view::ResetView;

/// The panel over the window, it takes the keys.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    #[default]
    None,
    Help,
    Settings,
}

impl Panel {
    pub fn toggle(&mut self, panel: Panel) {
        *self = if *self == panel { Panel::None } else { panel };
    }
}

#[derive(Component, Default, Clone)]
struct LessonTitle;

#[derive(Component, Default, Clone)]
struct StepLabel;

#[derive(Component, Default, Clone)]
struct Instruction;

#[derive(Component, Default, Clone)]
struct About;

#[derive(Component, Default, Clone)]
struct StepDots;

#[derive(Component, Default, Clone)]
struct BackButton;

#[derive(Component, Default, Clone)]
struct FoldButton;

#[derive(Component, Default, Clone)]
struct CreasePattern;

#[derive(Component, Default, Clone)]
struct PatternImage;

#[derive(Component, Default, Clone)]
struct ValleySwatch;

#[derive(Component, Default, Clone)]
struct MountainSwatch;

#[derive(Component, Default, Clone)]
struct HelpOverlay;

#[derive(Component, Default, Clone)]
struct SettingsOverlay;

/// A lesson of the menu.
#[derive(Component, Default, Clone)]
struct LessonItem(usize);

/// A dot of the steps at the bottom, done, current or to come.
#[derive(Component, Default, Clone)]
struct StepDot(usize);

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Setting {
    #[default]
    Arrows,
    Landing,
    CreasePattern,
    ReduceMotion,
}

impl Setting {
    fn get(self, settings: &Settings) -> bool {
        match self {
            Setting::Arrows => settings.arrows,
            Setting::Landing => settings.landing,
            Setting::CreasePattern => settings.crease_pattern,
            Setting::ReduceMotion => settings.reduce_motion,
        }
    }
}

#[derive(Component, Default, Clone)]
struct SettingControl(Setting);

const HELP: &str = "\
FOLDING
  Drag the flap that glows toward the arrow, past half way, and let go: it folds.
  A tap on the flap, Fold, → Space or Enter fold it for you.
  ‹ Back, ← or Backspace unfold the last step: go back and forth as you like.

THE LINES
  Dashed: a valley fold, the paper comes toward you.
  Dash-dot-dot: a mountain fold, the paper goes behind.
  Thin solid lines are the creases already made, fainter the older they are.
  The dotted outline shows where the flap lands, the arrow how it gets there.
  The crease pattern (bottom right) is the sheet unfolded: valley blue, mountain red.

VIEW
  Drag around the paper, or with the right button, to turn it in 3D.
  The wheel or a pinch zooms, R or Reset view brings it back.

F1 or ? opens this help, Esc or a click closes it.";

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Panel>().add_systems(Startup, spawn_ui).add_systems(
            Update,
            (
                init_settings,
                init_images,
                update_texts,
                update_dots,
                show_panels.run_if(resource_changed::<Panel>),
                show_pattern,
                panel_keys,
            )
                .after(SessionSystems),
        );
    }
}

fn spawn_ui(mut commands: Commands, session: Res<Session>, settings: Res<Settings>) {
    let lessons: Vec<Box<dyn SceneList>> = session
        .lessons
        .iter()
        .enumerate()
        .map(|(i, lesson)| -> Box<dyn SceneList> {
            let title = lesson.title;
            bsn! {
                @FeathersMenuItem {
                    @caption: bsn! { @caption(title) }
                }
                LessonItem({i})
                on(pick_lesson)
            }
            .into()
        })
        .collect();
    let theme = ThemeMode::ALL.iter().position(|t| *t == settings.theme);
    let themes = list_rows_from_strings(ThemeMode::ALL.map(ThemeMode::name), theme);
    let paper = PaperColor::ALL.iter().position(|p| *p == settings.paper);
    let papers = list_rows_from_strings(PaperColor::ALL.map(PaperColor::name), paper);
    let dots: Vec<Box<dyn SceneList>> = (0..MAX_DOTS)
        .map(|i| -> Box<dyn SceneList> {
            bsn! {
                Node {
                    width: px(12),
                    height: px(12),
                    border: px(2),
                    border_radius: BorderRadius::all(px(6)),
                }
                StepDot({i})
            }
            .into()
        })
        .collect();
    commands.spawn_scene(root(Box::new(lessons), Box::new(dots), themes, papers));
}

/// The longest lesson has fewer steps.
const MAX_DOTS: usize = 16;

fn root(
    lessons: Box<dyn SceneList>,
    dots: Box<dyn SceneList>,
    themes: Box<dyn SceneList>,
    papers: Box<dyn SceneList>,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
        }
        TabGroup
        Pickable::IGNORE
        Children [
            @toolbar(lessons)
            --
            // The paper shows through.
            Node {
                flex_grow: 1.0,
            }
            Pickable::IGNORE
            --
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::End,
                justify_content: JustifyContent::Center,
                padding: px(12),
                column_gap: px(12),
            }
            Pickable::IGNORE
            Children [
                @instructions(dots)
                --
                @crease_pattern()
            ]
            --
            @help_overlay()
            --
            @settings_overlay(themes, papers)
        ]
    }
}

fn toolbar(lessons: Box<dyn SceneList>) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            column_gap: px(8),
            row_gap: px(6),
            padding: px(8),
        }
        ThemeBackgroundColor(feathers_tokens::PANE_HEADER_BG)
        Children [
            @label("Bevygami")
            --
            @FeathersMenu
            Children [
                @FeathersMenuButton {
                    @caption: bsn! { @caption("Lessons") }
                }
                --
                @FeathersMenuPopup
                Children [ {lessons} ]
            ]
            --
            @FeathersButton {
                @caption: bsn! { @caption("‹ Back") }
            }
            BackButton
            on(|_: On<Activate>, mut commands: MessageWriter<Command>, mut focus: ResMut<InputFocus>| {
                commands.write(Command::Back);
                focus.clear();
            })
            --
            @label_dim("") StepLabel
            Node {
                min_width: px(96),
                justify_content: JustifyContent::Center,
            }
            --
            @FeathersButton {
                @caption: bsn! { @caption("Fold ›") }
                @variant: ButtonVariant::Primary,
            }
            FoldButton
            on(|_: On<Activate>, mut commands: MessageWriter<Command>, mut focus: ResMut<InputFocus>| {
                commands.write(Command::Fold);
                focus.clear();
            })
            --
            @FeathersButton {
                @caption: bsn! { @caption("Restart") }
            }
            on(|_: On<Activate>, mut commands: MessageWriter<Command>, mut focus: ResMut<InputFocus>| {
                commands.write(Command::Restart);
                focus.clear();
            })
            --
            @flex_spacer()
            --
            @FeathersButton {
                @caption: bsn! { @caption("Reset view") }
            }
            on(|_: On<Activate>, mut reset: MessageWriter<ResetView>, mut focus: ResMut<InputFocus>| {
                reset.write(ResetView);
                focus.clear();
            })
            --
            @FeathersButton {
                @caption: bsn! { @caption("Settings") }
            }
            on(|_: On<Activate>, mut panel: ResMut<Panel>| panel.toggle(Panel::Settings))
            --
            @FeathersButton {
                @caption: bsn! { @caption("?") }
            }
            on(|_: On<Activate>, mut panel: ResMut<Panel>| panel.toggle(Panel::Help))
        ]
    }
}

/// The lesson, the instruction of the step and a dot per step.
fn instructions(dots: Box<dyn SceneList>) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Start,
            row_gap: px(6),
            padding: UiRect::axes(px(16), px(12)),
            border: px(1),
            border_radius: BorderRadius::all(px(10)),
            max_width: px(640),
            flex_shrink: 1.0,
        }
        ThemeBackgroundColor(feathers_tokens::WINDOW_BG)
        ThemeBorderColor(feathers_tokens::BUTTON_BG)
        Children [
            @label_dim("") LessonTitle
            --
            @label("") Instruction
            TextFont { font_size: FontSize::Px(19.0) }
            --
            @label_dim("") About
            --
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(6),
                row_gap: px(6),
                margin: UiRect::top(px(4)),
            }
            StepDots
            Children [ {dots} ]
        ]
    }
}

fn crease_pattern() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(4),
            padding: px(8),
            border: px(1),
            border_radius: BorderRadius::all(px(10)),
        }
        ThemeBackgroundColor(feathers_tokens::WINDOW_BG)
        ThemeBorderColor(feathers_tokens::BUTTON_BG)
        CreasePattern
        Children [
            @label_dim("Crease pattern")
            --
            Node {
                width: px(cpview::SIZE as f32 * 0.8),
                height: px(cpview::SIZE as f32 * 0.8),
            }
            ImageNode
            PatternImage
            --
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
            }
            Children [
                Node { width: px(cpview::SWATCH_W as f32), height: px(cpview::SWATCH_H as f32) }
                ImageNode
                ValleySwatch
                --
                @label_dim("valley")
                --
                Node { width: px(cpview::SWATCH_W as f32), height: px(cpview::SWATCH_H as f32), margin: UiRect::left(px(6)) }
                ImageNode
                MountainSwatch
                --
                @label_dim("mountain")
            ]
        ]
    }
}

fn help_overlay() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            display: Display::None,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        GlobalZIndex(50)
        HelpOverlay
        Children [
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
            }
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6))
            on(close_panel)
            --
            @group()
            Node {
                max_width: px(720),
                margin: px(12),
            }
            Children [
                @group_header() Children [ @label("Help") ]
                --
                @group_body()
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    padding: px(12),
                }
                Children [
                    @label(HELP)
                    --
                    @FeathersButton {
                        @caption: bsn! { @caption("Close") }
                        @variant: ButtonVariant::Primary,
                    }
                    on(close_panel_button)
                ]
            ]
        ]
    }
}

/// A checkbox of the settings, with what it does.
fn setting_checkbox(setting: Setting, text: &'static str) -> impl Scene {
    bsn! {
        @FeathersCheckbox {
            @caption: bsn! { @caption(text) }
        }
        SettingControl({setting})
        on(on_setting_change)
    }
}

fn settings_overlay(themes: Box<dyn SceneList>, papers: Box<dyn SceneList>) -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            display: Display::None,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        GlobalZIndex(50)
        SettingsOverlay
        Children [
            // A click beside the panel closes it, the clicks in the panel do not reach it.
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
            }
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6))
            on(close_panel)
            --
            @group()
            Node {
                width: px(480),
                max_width: percent(94),
            }
            Children [
                @group_header() Children [ @label("Settings") ]
                --
                @group_body()
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    padding: px(12),
                }
                Children [
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(12),
                    }
                    Children [
                        @label("Theme") Node { min_width: px(110) }
                        --
                        @FeathersSelect {
                            @options: themes,
                        }
                        Node {
                            min_width: px(180),
                        }
                        on(on_theme_change)
                    ]
                    --
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(12),
                    }
                    Children [
                        @label("Paper") Node { min_width: px(110) }
                        --
                        @FeathersSelect {
                            @options: papers,
                        }
                        Node {
                            min_width: px(180),
                        }
                        on(on_paper_change)
                    ]
                    --
                    @setting_checkbox(Setting::Arrows, "Arrows: how the paper moves")
                    --
                    @setting_checkbox(Setting::Landing, "Landing outline: where the flap lands, dotted")
                    --
                    @setting_checkbox(Setting::CreasePattern, "Crease pattern: the sheet unfolded, in a corner")
                    --
                    @setting_checkbox(Setting::ReduceMotion, "Reduce motion: no breathing, faster folds")
                    --
                    @FeathersButton {
                        @caption: bsn! { @caption("Close") }
                        @variant: ButtonVariant::Primary,
                    }
                    on(close_panel_button)
                ]
            ]
        ]
    }
}

fn pick_lesson(
    activate: On<Activate>,
    items: Query<&LessonItem>,
    mut commands: MessageWriter<Command>,
    mut focus: ResMut<InputFocus>,
) {
    if let Ok(LessonItem(i)) = items.get(activate.entity) {
        commands.write(Command::Lesson(*i));
    }
    focus.clear();
}

fn close_panel(_: On<PointerClick>, mut panel: ResMut<Panel>) {
    *panel = Panel::None;
}

fn close_panel_button(_: On<Activate>, mut panel: ResMut<Panel>, mut focus: ResMut<InputFocus>) {
    *panel = Panel::None;
    focus.clear();
}

fn panel_keys(keyboard: Res<ButtonInput<KeyCode>>, mut panel: ResMut<Panel>) {
    if keyboard.just_pressed(KeyCode::Escape) {
        *panel = Panel::None;
    }
    if keyboard.just_pressed(KeyCode::F1) || keyboard.just_pressed(KeyCode::Slash) {
        panel.toggle(Panel::Help);
    }
}

/// The images of the crease pattern, drawn by [`crate::cpview`].
#[allow(clippy::type_complexity)]
fn init_images(
    cp: Res<CpImages>,
    mut images: Query<
        (&mut ImageNode, Has<PatternImage>, Has<ValleySwatch>),
        Or<(Added<PatternImage>, Added<ValleySwatch>, Added<MountainSwatch>)>,
    >,
) {
    for (mut node, pattern, valley) in &mut images {
        node.image = if pattern {
            cp.pattern.clone()
        } else if valley {
            cp.valley.clone()
        } else {
            cp.mountain.clone()
        };
    }
}

/// The checkboxes have no initial value prop.
fn init_settings(
    controls: Query<(Entity, &SettingControl), Added<SettingControl>>,
    settings: Res<Settings>,
    mut commands: Commands,
) {
    for (entity, SettingControl(setting)) in &controls {
        if setting.get(&settings) {
            commands.entity(entity).insert(Checked);
        }
    }
}

fn on_setting_change(
    change: On<ValueChange<bool>>,
    controls: Query<&SettingControl>,
    mut settings: ResMut<Settings>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    let Ok(SettingControl(setting)) = controls.get(change.source) else {
        return;
    };
    if change.value {
        commands.entity(change.source).insert(Checked);
    } else {
        commands.entity(change.source).remove::<Checked>();
    }
    match setting {
        Setting::Arrows => settings.arrows = change.value,
        Setting::Landing => settings.landing = change.value,
        Setting::CreasePattern => settings.crease_pattern = change.value,
        Setting::ReduceMotion => settings.reduce_motion = change.value,
    }
    settings.save();
    focus.clear();
}

fn on_theme_change(
    change: On<ValueChange<Entity>>,
    options: Query<&OptionIndex>,
    mut settings: ResMut<Settings>,
    mut mode: ResMut<ThemeMode>,
) {
    if let Ok(option) = options.get(change.value)
        && let Some(theme) = ThemeMode::ALL.get(option.0)
    {
        settings.theme = *theme;
        *mode = *theme;
        settings.save();
    }
}

fn on_paper_change(change: On<ValueChange<Entity>>, options: Query<&OptionIndex>, mut settings: ResMut<Settings>) {
    if let Ok(option) = options.get(change.value)
        && let Some(paper) = PaperColor::ALL.get(option.0)
    {
        settings.paper = *paper;
        settings.save();
    }
}

fn show_panels(
    panel: Res<Panel>,
    mut help: Query<&mut Node, (With<HelpOverlay>, Without<SettingsOverlay>)>,
    mut settings: Query<&mut Node, (With<SettingsOverlay>, Without<HelpOverlay>)>,
) {
    let display = |on: bool| if on { Display::Flex } else { Display::None };
    for mut node in &mut help {
        node.display = display(*panel == Panel::Help);
    }
    for mut node in &mut settings {
        node.display = display(*panel == Panel::Settings);
    }
}

fn show_pattern(settings: Res<Settings>, mut pattern: Query<&mut Node, With<CreasePattern>>) {
    if !settings.is_changed() {
        return;
    }
    for mut node in &mut pattern {
        node.display = if settings.crease_pattern {
            Display::Flex
        } else {
            Display::None
        };
    }
}

#[allow(clippy::type_complexity)]
fn update_texts(
    session: Res<Session>,
    mut texts: ParamSet<(
        Query<&mut Text, With<LessonTitle>>,
        Query<&mut Text, With<Instruction>>,
        Query<&mut Text, With<About>>,
        Query<&mut Text, With<StepLabel>>,
    )>,
    back: Query<Entity, With<BackButton>>,
    fold: Query<Entity, With<FoldButton>>,
    mut shown: Local<Option<(u32, usize)>>,
    mut commands: Commands,
) {
    if *shown == Some((session.generation, session.lesson)) {
        return;
    }
    *shown = Some((session.generation, session.lesson));
    let lesson = session.current();
    let n = session.plans.len();
    let instruction = match lesson.steps.get(session.step) {
        Some(step) => step.text.to_string(),
        None => format!("Done! {}", lesson.done),
    };
    let label = if session.finished() {
        format!("Done, {n} steps")
    } else {
        format!("Step {} of {n}", session.step + 1)
    };
    for mut text in &mut texts.p0() {
        text.0 = lesson.title.to_string();
    }
    for mut text in &mut texts.p1() {
        text.0 = instruction.clone();
    }
    for mut text in &mut texts.p2() {
        text.0 = lesson.about.to_string();
    }
    for mut text in &mut texts.p3() {
        text.0 = label.clone();
    }
    for entity in &back {
        if session.step == 0 {
            commands.entity(entity).insert(InteractionDisabled);
        } else {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
    }
    for entity in &fold {
        if session.finished() {
            commands.entity(entity).insert(InteractionDisabled);
        } else {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
    }
}

/// Done steps filled, the current one breathing, the ones to come hollow.
fn update_dots(
    time: Res<Time>,
    session: Res<Session>,
    settings: Res<Settings>,
    theme: Res<bevy::feathers::theme::UiTheme>,
    mut dots: Query<(&StepDot, &mut Node, &mut BackgroundColor, &mut BorderColor)>,
) {
    let accent = theme.color(&feathers_tokens::BUTTON_PRIMARY_BG);
    let dim = theme.color(&feathers_tokens::TEXT_DIM);
    let breath = crate::paper::breath(&time, &settings);
    for (StepDot(i), mut node, mut background, mut border) in &mut dots {
        node.display = if *i < session.plans.len() {
            Display::Flex
        } else {
            Display::None
        };
        let (fill, edge) = if *i < session.step {
            (accent, accent)
        } else if *i == session.step {
            (accent.with_alpha(0.25 + 0.5 * breath), accent)
        } else {
            (Color::NONE, dim.with_alpha(0.6))
        };
        background.0 = fill;
        *border = BorderColor::all(edge);
    }
}
