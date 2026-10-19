use std::sync::atomic::{AtomicI32, Ordering};

use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};

/// Last signal caught by the shell (0 if none).
pub static SIGNAL: AtomicI32 = AtomicI32::new(0);

extern "C" fn on_signal(sig: i32) {
    SIGNAL.store(sig, Ordering::SeqCst);
}

/// How signal delivery should behave for the current shell state.
#[allow(dead_code)]
pub enum Mode {
    Prompt,
    #[allow(dead_code)]
    Running,
    #[allow(dead_code)]
    Heredoc,
    #[allow(dead_code)]
    Child,
}

/// Install the signal handlers that match `mode`.
pub fn set_mode(mode: Mode) {
    let (sigint, sigquit) = match mode {
        Mode::Prompt => (SigHandler::Handler(on_signal), SigHandler::SigIgn),
        Mode::Running => (SigHandler::SigIgn, SigHandler::SigIgn),
        Mode::Heredoc => (SigHandler::Handler(on_signal), SigHandler::SigIgn),
        Mode::Child => (SigHandler::SigDfl, SigHandler::SigDfl),
    };
    let act_int = SigAction::new(sigint, SaFlags::empty(), SigSet::empty());
    let act_quit = SigAction::new(sigquit, SaFlags::empty(), SigSet::empty());
    unsafe {
        // SAFETY: the handler only stores into an atomic; all other arguments
        // are valid sigaction structures.
        let _ = sigaction(Signal::SIGINT, &act_int);
        let _ = sigaction(Signal::SIGQUIT, &act_quit);
    }
}
