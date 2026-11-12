use crate::reg::ps;
use serde::Serialize;

/// (package, display name, what it is, needed by some games). From ChrisTitusTech WinUtil's appx list.
const APPS: &[(&str, &str, &str, bool)] = &[
    ("Microsoft.Copilot", "Copilot", "Microsoft's AI assistant app", false),
    ("Microsoft.BingNews", "News", "News headlines and feeds", false),
    ("Microsoft.BingWeather", "Weather", "Forecasts and radar", false),
    ("Microsoft.BingSearch", "Bing Search", "Bing integration for Windows search", false),
    ("MicrosoftWindows.Client.WebExperience", "Widgets", "The news and weather board and its taskbar button. Windows blocks switching it off by policy, so removal is the way.", false),
    ("Microsoft.StartExperiencesApp", "Start Experiences", "Feeds the Widgets board with news and weather", false),
    ("Clipchamp.Clipchamp", "Clipchamp", "Video editor", false),
    ("Microsoft.MicrosoftOfficeHub", "Microsoft 365", "Launcher for Office apps; not Office itself", false),
    ("Microsoft.OutlookForWindows", "Outlook (new)", "Mail and calendar", false),
    ("MSTeams", "Microsoft Teams", "Chat and meetings", false),
    ("Microsoft.Todos", "To Do", "Tasks and reminders", false),
    ("Microsoft.PowerAutomateDesktop", "Power Automate", "Workflow automation", false),
    ("Microsoft.Windows.DevHome", "Dev Home", "Developer dashboard", false),
    ("Microsoft.WindowsFeedbackHub", "Feedback Hub", "Sends feedback to Microsoft", false),
    ("Microsoft.GetHelp", "Get Help", "Support and troubleshooting", false),
    ("MicrosoftCorporationII.QuickAssist", "Quick Assist", "Remote help and screen sharing", false),
    ("Microsoft.YourPhone", "Phone Link", "Phone calls, texts and photos on the PC; runs in the background", false),
    ("MicrosoftWindows.CrossDevice", "Cross Device", "Background link to paired phones", false),
    ("Microsoft.MicrosoftSolitaireCollection", "Solitaire Collection", "Card games", false),
    ("Microsoft.ZuneVideo", "Movies & TV", "Video player and store", false),
    ("Microsoft.ZuneMusic", "Media Player", "Music and video player", false),
    ("Microsoft.MicrosoftStickyNotes", "Sticky Notes", "Desktop notes", false),
    ("Microsoft.WindowsSoundRecorder", "Sound Recorder", "Audio recording", false),
    ("Microsoft.WindowsAlarms", "Clock", "Alarms and timers", false),
    ("Microsoft.WindowsCamera", "Camera", "Webcam capture", false),
    ("Microsoft.GamingApp", "Xbox app", "Game Pass and Microsoft Store games", true),
    ("Microsoft.XboxGamingOverlay", "Xbox Game Bar", "Win+G overlay; also provides Game Mode controls", true),
    ("Microsoft.XboxIdentityProvider", "Xbox Identity Provider", "Xbox sign-in used by many games", true),
    ("Microsoft.Xbox.TCUI", "Xbox TCUI", "Sign-in and friends UI inside games", true),
    ("Microsoft.XboxSpeechToTextOverlay", "Xbox Speech to Text", "Voice chat captions", true),
];

#[derive(Serialize)]
pub struct App {
    pkg: &'static str,
    name: &'static str,
    note: &'static str,
    caution: bool,
}

pub fn list() -> Result<Vec<App>, String> {
    let installed = ps("Get-AppxPackage | ForEach-Object Name")?;
    let installed: Vec<&str> = installed.lines().map(str::trim).collect();
    Ok(APPS
        .iter()
        .filter(|a| installed.contains(&a.0))
        .map(|&(pkg, name, note, caution)| App {
            pkg,
            name,
            note,
            caution,
        })
        .collect())
}

/// Removes each package in turn; returns one message per failure.
pub fn remove(pkgs: &[String]) -> Vec<String> {
    let mut errors = Vec::new();
    for p in pkgs {
        // Only names from our own list ever reach PowerShell.
        let Some(app) = APPS.iter().find(|a| a.0 == p) else {
            errors.push(format!("{p}: not a known app"));
            continue;
        };
        // Match the current-user listing; do not remove apps for unrelated accounts.
        let script = format!("Get-AppxPackage -Name '{}' | Remove-AppxPackage", app.0);
        if let Err(e) = ps(&script) {
            errors.push(format!("{}: {e}", app.1));
        }
    }
    errors
}
