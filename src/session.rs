//! The lesson being folded: its planned steps, the step the player is on, and the progress of
//! the fold in motion.
use std::f32::consts::PI;

use bevy::prelude::*;

use crate::fold::{Paper, Planned, plan_all};
use crate::lessons::{self, Lesson};
use crate::settings::Settings;

/// What the buttons, the keys and the menus ask for.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Folds the current step, or ends the fold in motion.
    Fold,
    /// Unfolds the last step done.
    Back,
    Restart,
    Lesson(usize),
}

pub struct Animation {
    pub target: f32,
}

#[derive(Resource)]
pub struct Session {
    pub lessons: Vec<Lesson>,
    pub lesson: usize,
    pub plans: Vec<Planned>,
    /// The paper with every step folded.
    pub last: Paper,
    /// The steps done.
    pub step: usize,
    /// How far the current step is folded, 0 to 1 (and back to 0 for a fold and unfold).
    pub t: f32,
    pub animation: Option<Animation>,
    /// The player holds the paper.
    pub dragging: bool,
    /// Bumped when the faces must be spawned again.
    pub generation: u32,
}

impl Session {
    pub fn new(lesson: usize, step: usize) -> Session {
        let lessons = lessons::all();
        let mut session = Session {
            lessons,
            lesson: 0,
            plans: Vec::new(),
            last: Paper::square(crate::fold::Pose::IDENTITY),
            step: 0,
            t: 0.0,
            animation: None,
            dragging: false,
            generation: 0,
        };
        session.load(lesson, step);
        session
    }

    pub fn load(&mut self, lesson: usize, step: usize) {
        self.lesson = lesson.min(self.lessons.len() - 1);
        let lesson = &self.lessons[self.lesson];
        match plan_all(lesson.start(), &lesson.folds()) {
            Ok((plans, last)) => {
                self.plans = plans;
                self.last = last;
            }
            Err((i, err)) => {
                error!("{}: step {} does not fold: {err:?}", lesson.title, i + 1);
                self.plans = Vec::new();
                self.last = lesson.start();
            }
        }
        self.step = step.min(self.plans.len());
        self.t = 0.0;
        self.animation = None;
        self.dragging = false;
        self.generation += 1;
    }

    pub fn current(&self) -> &Lesson {
        &self.lessons[self.lesson]
    }

    pub fn finished(&self) -> bool {
        self.step >= self.plans.len()
    }

    /// The fold of the current step, none at the end.
    pub fn plan(&self) -> Option<&Planned> {
        self.plans.get(self.step)
    }

    /// The paper on screen: split along the current fold line, or the finished model.
    pub fn shown(&self) -> &Paper {
        self.plan().map(|p| &p.before).unwrap_or(&self.last)
    }

    /// The turn of the moving faces around the fold line, 0 to π.
    pub fn angle(&self) -> f32 {
        match self.plan() {
            Some(plan) if plan.fold.unfold => PI * (PI * self.t).sin() * 0.97,
            Some(_) => PI * self.t,
            None => 0.0,
        }
    }

    /// The paper rests: no fold in motion nor in the hand.
    pub fn at_rest(&self) -> bool {
        self.animation.is_none() && !self.dragging && self.t == 0.0
    }
}

pub struct SessionPlugin;

impl Plugin for SessionPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Command>()
            .add_systems(Update, (run_commands, animate).chain().in_set(SessionSystems));
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionSystems;

/// The seconds a fold takes.
const FOLD_SECONDS: f32 = 1.1;

fn run_commands(mut commands: MessageReader<Command>, mut session: ResMut<Session>, mut settings: ResMut<Settings>) {
    for command in commands.read() {
        match command {
            Command::Fold => {
                if session.finished() {
                    continue;
                }
                if session.animation.as_ref().is_some_and(|a| a.target == 1.0) {
                    // A second press ends the fold at once.
                    session.t = 1.0;
                } else {
                    session.animation = Some(Animation { target: 1.0 });
                }
            }
            Command::Back => {
                if session.animation.as_ref().is_some_and(|a| a.target == 1.0) || session.t > 0.0 {
                    // Back during a fold: it unfolds again.
                    session.animation = Some(Animation { target: 0.0 });
                } else if session.step > 0 {
                    session.step -= 1;
                    session.t = 1.0;
                    session.animation = Some(Animation { target: 0.0 });
                    session.generation += 1;
                }
            }
            Command::Restart => {
                let lesson = session.lesson;
                session.load(lesson, 0);
            }
            Command::Lesson(i) => session.load(*i, 0),
        }
        settings.lesson = session.lesson;
        settings.step = session.step;
        settings.save();
    }
}

fn animate(time: Res<Time>, mut session: ResMut<Session>, mut settings: ResMut<Settings>) {
    let Some(target) = session.animation.as_ref().map(|a| a.target) else {
        return;
    };
    let speed = if settings.reduce_motion { 3.0 } else { 1.0 } / FOLD_SECONDS;
    let t = session.t;
    session.t = t + (target - t).clamp(-speed * time.delta_secs(), speed * time.delta_secs());
    if (session.t - target).abs() > 1e-4 {
        return;
    }
    session.animation = None;
    session.t = 0.0;
    if target == 1.0 {
        session.step += 1;
        session.generation += 1;
        if session.finished() && !settings.done.contains(&session.lesson) {
            let lesson = session.lesson;
            settings.done.push(lesson);
        }
    }
    settings.step = session.step;
    settings.save();
}
