//! `src/lib/stores/toast.svelte.ts`: success, info and error toasts that dismiss themselves,
//! paused while the stack is hovered. Time is the app clock `poll` gets.

use std::rc::Rc;

use crate::app::App;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

#[derive(Clone)]
pub struct ToastAction {
    pub label: String,
    pub run: Rc<dyn Fn(&mut App)>,
}

#[derive(Clone)]
pub struct Toast {
    pub id: u64,
    pub kind: ToastKind,
    pub message: String,
    pub action: Option<ToastAction>,
    /// Seconds of display left; counts down only while not paused.
    pub remaining: f64,
}

#[derive(Default)]
pub struct Toasts {
    pub list: Vec<Toast>,
    next: u64,
    pub paused: bool,
    last_tick: Option<f64>,
}

impl Toasts {
    fn push(&mut self, kind: ToastKind, message: String, ttl: f64, action: Option<ToastAction>) -> u64 {
        self.next += 1;
        self.list.push(Toast { id: self.next, kind, message, action, remaining: ttl });
        self.next
    }

    pub fn success(&mut self, message: impl Into<String>) -> u64 {
        self.push(ToastKind::Success, message.into(), 3.5, None)
    }
    pub fn info(&mut self, message: impl Into<String>) -> u64 {
        self.push(ToastKind::Info, message.into(), 4.5, None)
    }
    pub fn error(&mut self, message: impl Into<String>) -> u64 {
        self.push(ToastKind::Error, message.into(), 12.0, None)
    }
    pub fn with_action(&mut self, kind: ToastKind, message: impl Into<String>, label: impl Into<String>, run: impl Fn(&mut App) + 'static) -> u64 {
        let ttl = if kind == ToastKind::Error { 12.0 } else { 8.0 };
        self.push(kind, message.into(), ttl, Some(ToastAction { label: label.into(), run: Rc::new(run) }))
    }

    pub fn dismiss(&mut self, id: u64) {
        self.list.retain(|toast| toast.id != id);
    }

    /// Advance the countdowns to `now`; true when one expired. Returns the next deadline too.
    pub(crate) fn tick(&mut self, now: f64) -> (bool, Option<f64>) {
        let dt = self.last_tick.map_or(0.0, |last| (now - last).max(0.0));
        self.last_tick = Some(now);
        if !self.paused {
            for toast in &mut self.list {
                toast.remaining -= dt;
            }
        }
        let before = self.list.len();
        self.list.retain(|toast| toast.remaining > 0.0);
        let next = if self.paused { None } else { self.list.iter().map(|toast| toast.remaining).reduce(f64::min) };
        (self.list.len() != before, next)
    }
}
