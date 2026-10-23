//! Windows agent interface: fail closed until isolated child-process execution
//! (Windows Job Objects, timeouts and tool-denial verification) is implemented.
//! This deliberately does not silently execute commands with full user rights.

#[derive(Clone, Debug)]
pub struct Agent {
    pub id: String,
    pub name: &'static str,
}

pub fn default_agent() -> Option<Agent> {
    None
}

pub fn run(_agent: &Agent, _prompt: &str, _transcript: &str) -> Result<String, String> {
    Err("AI chapter generation is disabled on Windows until the agent runner has equivalent process isolation".to_owned())
}

pub fn cli(_args: &[String]) -> gtk::glib::ExitCode {
    eprintln!("AI agent execution on Windows is not yet available (secure runner pending)");
    gtk::glib::ExitCode::FAILURE
}
