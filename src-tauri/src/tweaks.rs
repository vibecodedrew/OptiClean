//! Every tweak is data. Adding one means adding an entry here; nothing else changes.
//! `off` values are the Windows defaults, so Undo returns to stock rather than to a remembered value.

use crate::reg::{Action, Hive, Val, Val::*};
use serde::Serialize;

#[derive(Serialize)]
pub struct Tweak {
    pub id: &'static str,
    pub category: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub details: &'static str,
    /// What kind of gain: FPS, 1% lows, Latency, Responsiveness, Privacy, Fix, Look.
    pub kind: &'static str,
    pub gain: &'static str,
    /// "Safe", or a short warning shown in amber.
    pub risk: &'static str,
    /// "", "Reboot", "Sign out" or "Restart Explorer".
    pub needs: &'static str,
    #[serde(skip)]
    pub actions: &'static [Action],
}

const fn cu(path: &'static str, name: &'static str, on: Val, off: Val) -> Action {
    Action::Reg {
        hive: Hive::Hkcu,
        path,
        name,
        on,
        off,
    }
}
const fn lm(path: &'static str, name: &'static str, on: Val, off: Val) -> Action {
    Action::Reg {
        hive: Hive::Hklm,
        path,
        name,
        on,
        off,
    }
}

const PROFILE: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile";
const GAMES: &str =
    r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games";
const ADVANCED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const SEARCH: &str = r"Software\Microsoft\Windows\CurrentVersion\Search";
const SEARCH_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\SearchSettings";
const CDM: &str = r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager";
const DESKTOP: &str = r"Control Panel\Desktop";
const MOUSE: &str = r"Control Panel\Mouse";
const ACTIVITY: &str = r"SOFTWARE\Policies\Microsoft\Windows\System";

pub static TWEAKS: &[Tweak] = &[
    // ---------------- Gaming ----------------
    Tweak {
        id: "memory-integrity",
        category: "Gaming",
        title: "Turn off Memory Integrity (Core Isolation)",
        summary: "Removes the virtualization layer that sits between games and the processor.",
        details: "Memory Integrity runs Windows' driver checks inside a small hypervisor. That protection costs CPU time on every transition between a game, the kernel and the graphics driver. Turning it off removes that cost. The trade-off is real: Windows loses a defence against malicious or vulnerable kernel drivers. A few anti-cheat systems refuse to start with it off; if a game complains, undo this and reboot.",
        kind: "FPS",
        gain: "Hardware-dependent CPU overhead reduction, possibly no measurable gain. Disables a security boundary; keep enabled unless you explicitly accept the security cost.",
        risk: "Security trade-off",
        needs: "Reboot",
        actions: &[lm(r"SYSTEM\CurrentControlSet\Control\DeviceGuard\Scenarios\HypervisorEnforcedCodeIntegrity", "Enabled", Dword(0), Dword(1))],
    },
    Tweak {
        id: "hags",
        category: "Gaming",
        title: "Hardware-accelerated GPU scheduling",
        summary: "Lets the graphics card manage its own work queue instead of Windows.",
        details: "Moves GPU scheduling from the Windows graphics kernel onto the card's own scheduler, taking a small job off the CPU. NVIDIA's frame generation features require it. A small number of older games or capture tools stutter with it on; undo it if you notice that.",
        kind: "Latency",
        gain: "Small. Slightly lower CPU overhead and input delay; average FPS is normally within a percent or two either way.",
        risk: "Safe",
        needs: "Reboot",
        actions: &[lm(r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers", "HwSchMode", Dword(2), Dword(1))],
    },
    Tweak {
        id: "game-mode",
        category: "Gaming",
        title: "Game Mode",
        summary: "Tells Windows to hold back updates and background work while a game runs.",
        details: "While a game is in the foreground, Windows stops Windows Update from installing drivers or showing restart prompts and gives the game's threads preference over background processes. Windows 11 turns it on by default; applying this writes the setting explicitly so it stays on.",
        kind: "1% lows",
        gain: "No average FPS change. It prevents the occasional stutter caused by an update or background task landing mid-match.",
        risk: "Safe",
        needs: "",
        actions: &[
            cu(r"Software\Microsoft\GameBar", "AutoGameModeEnabled", Dword(1), Dword(0)),
            cu(r"Software\Microsoft\GameBar", "AllowAutoGameMode", Dword(1), Dword(0)),
        ],
    },
    Tweak {
        id: "game-dvr",
        category: "Gaming",
        title: "Turn off background recording (Game DVR)",
        summary: "Stops Windows silently recording gameplay in the background.",
        details: "Game DVR can keep a rolling video of your game so you can save the last 30 seconds. That uses the GPU's video encoder and some CPU the whole time you play. This turns the recording off; the Game Bar itself and Game Mode keep working. If you record with NVIDIA's overlay or OBS you lose nothing.",
        kind: "1% lows",
        gain: "Small but real if recording was active: a few percent, mostly in frame-time consistency. Nothing if it was already off.",
        risk: "Safe",
        needs: "",
        actions: &[
            cu(r"System\GameConfigStore", "GameDVR_Enabled", Dword(0), Dword(1)),
            cu(r"Software\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled", Dword(0), Dword(1)),
        ],
    },
    Tweak {
        id: "windowed-optimizations",
        category: "Gaming",
        title: "Optimizations for windowed games",
        summary: "Makes borderless-window games present frames as efficiently as exclusive fullscreen.",
        details: "Few people know this setting exists. Older DirectX 10 and 11 games in borderless or windowed mode hand each frame to the desktop compositor, which adds a frame of delay. This switches them to the modern flip presentation model, which bypasses that copy and allows variable refresh rate in a window. It is the same switch as Settings > System > Display > Graphics > Optimizations for windowed games.",
        kind: "Latency",
        gain: "Lower input delay in borderless DX10/11 games, roughly matching exclusive fullscreen, and G-SYNC works in a window. No change for DX12 or Vulkan games, which already present this way.",
        risk: "Safe",
        needs: "",
        actions: &[Action::Script {
            apply: r"$p='HKCU:\Software\Microsoft\DirectX\UserGpuPreferences'; if(!(Test-Path $p)){New-Item $p -Force | Out-Null}; $v=[string](Get-ItemProperty $p).DirectXUserGlobalSettings; Set-ItemProperty $p DirectXUserGlobalSettings (($v -replace 'SwapEffectUpgradeEnable=\d;','') + 'SwapEffectUpgradeEnable=1;')",
            undo: r"$p='HKCU:\Software\Microsoft\DirectX\UserGpuPreferences'; if(!(Test-Path $p)){New-Item $p -Force | Out-Null}; $v=[string](Get-ItemProperty $p).DirectXUserGlobalSettings; Set-ItemProperty $p DirectXUserGlobalSettings (($v -replace 'SwapEffectUpgradeEnable=\d;','') + 'SwapEffectUpgradeEnable=0;')",
            check: r"[string](Get-ItemProperty 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -ErrorAction SilentlyContinue).DirectXUserGlobalSettings -match 'SwapEffectUpgradeEnable=1'",
        }],
    },
    Tweak {
        id: "mmcss-games",
        category: "Gaming",
        title: "High priority for the Games scheduler class",
        summary: "Raises the priority Windows gives to games that register with its multimedia scheduler.",
        details: "Windows has a Multimedia Class Scheduler that boosts threads belonging to named task types such as Audio and Games. This raises the Games task from its default medium scheduling category to High and raises its CPU and disk priority. It only affects games that ask the scheduler for the Games class, which many do not.",
        kind: "1% lows",
        gain: "Small to none, depending on the game. Harmless, and it is the tweak most guides mean by 'game priority high'.",
        risk: "Safe",
        needs: "Reboot",
        actions: &[
            lm(GAMES, "GPU Priority", Dword(8), Dword(8)),
            lm(GAMES, "Priority", Dword(6), Dword(2)),
            lm(GAMES, "Scheduling Category", Str("High"), Str("Medium")),
            lm(GAMES, "SFIO Priority", Str("High"), Str("Normal")),
        ],
    },
    Tweak {
        id: "system-responsiveness",
        category: "Gaming",
        title: "Reserve less CPU for background tasks",
        summary: "Lowers the share of processor time Windows guarantees to low-priority work during games.",
        details: "When multimedia threads are running, Windows keeps 20% of CPU time available for lower-priority tasks so the desktop stays responsive. This lowers the reservation to 10%, the smallest value Windows honours. Setting it to 0, as some guides say, is treated as 10 anyway.",
        kind: "1% lows",
        gain: "Small. With 6 or more cores there is usually spare CPU, so this matters only in games that load every core.",
        risk: "Safe",
        needs: "Reboot",
        actions: &[lm(PROFILE, "SystemResponsiveness", Dword(10), Dword(20))],
    },
    Tweak {
        id: "power-throttling",
        category: "Gaming",
        title: "Turn off power throttling",
        summary: "Stops Windows slowing down background helpers such as voice chat and launchers.",
        details: "Not widely known. Windows puts background processes into an efficiency mode that pins them to low clock speeds to save power. On a desktop that saves almost nothing, but it can make things that run beside your game stutter: Discord, a browser on a second monitor, a game launcher's overlay, capture software. This disables that throttling system-wide.",
        kind: "Fix",
        gain: "No FPS change in the game itself. Helps when voice chat crackles or a stream or overlay hitches while you play.",
        risk: "Safe",
        needs: "Reboot",
        actions: &[lm(r"SYSTEM\CurrentControlSet\Control\Power\PowerThrottling", "PowerThrottlingOff", Dword(1), Delete)],
    },
    Tweak {
        id: "ultimate-performance",
        category: "Gaming",
        title: "Ultimate Performance power plan",
        summary: "Keeps all processor cores awake and at speed instead of parking them.",
        details: "Adds Microsoft's hidden Ultimate Performance plan and switches to it. Cores are never parked and the processor does not wait before ramping up. Undo switches back to Balanced and removes the plan. The cost is higher idle power and a warmer, slightly louder PC when you are not gaming. If the plan is back on Balanced after a restart, another program is resetting it: motherboard and laptop utilities such as MSI Center or Armoury Crate apply their own power profile every time Windows starts. Change the profile there, or stop that utility starting with Windows.",
        kind: "1% lows",
        gain: "Small. Modern processors already ramp quickly on Balanced, so expect a slight improvement in frame-time consistency, not average FPS. If idle temperatures bother you, undo it; you lose very little.",
        risk: "Safe",
        needs: "",
        actions: &[Action::Script {
            apply: "powercfg -duplicatescheme e9a42b02-d5df-448d-aa00-03f14749eb61 a1b2c3d4-0000-4000-8000-0f7100000001 | Out-Null; powercfg -setactive a1b2c3d4-0000-4000-8000-0f7100000001",
            undo: "powercfg -setactive 381b4222-f694-41f0-9685-ff5bb260df2e; powercfg -delete a1b2c3d4-0000-4000-8000-0f7100000001",
            check: "[string](powercfg -getactivescheme) -match 'a1b2c3d4-0000-4000-8000-0f7100000001'",
        }],
    },
    Tweak {
        id: "mouse-acceleration",
        category: "Gaming",
        title: "Turn off mouse acceleration",
        summary: "Makes the cursor move the same distance for the same hand movement, at any speed.",
        details: "With 'Enhance pointer precision' on, a fast flick moves the cursor further than a slow one over the same distance on the mouse pad. That makes aim hard to build muscle memory for. Most shooters use raw input and ignore this, but menus, some older games and the desktop do not.",
        kind: "Latency",
        gain: "No FPS change. Consistent aim in games that do not use raw input.",
        risk: "Safe",
        needs: "Sign out",
        actions: &[
            cu(MOUSE, "MouseSpeed", Str("0"), Str("1")),
            cu(MOUSE, "MouseThreshold1", Str("0"), Str("6")),
            cu(MOUSE, "MouseThreshold2", Str("0"), Str("10")),
        ],
    },
    Tweak {
        id: "mpo",
        category: "Gaming",
        title: "Turn off Multiplane Overlay",
        summary: "A fix for flicker, black flashes or stutter in windowed games on some NVIDIA setups.",
        details: "Multiplane Overlay lets the graphics card composite a game window and the desktop as separate layers. It normally helps, but on some NVIDIA driver and monitor combinations it causes flickering, brief black screens, or stutter when a browser or Discord is open beside a game. NVIDIA documents this registry switch as the workaround. Leave this off unless you have those symptoms.",
        kind: "Fix",
        gain: "Not a performance boost. Apply only if you see flicker or stutter in borderless games with other windows open; otherwise it can slightly raise latency.",
        risk: "Only if you have the problem",
        needs: "Reboot",
        actions: &[lm(r"SOFTWARE\Microsoft\Windows\Dwm", "OverlayTestMode", Dword(5), Delete)],
    },
    Tweak {
        id: "timer-resolution",
        category: "Gaming",
        title: "Global timer resolution requests",
        summary: "Restores the older Windows behaviour where one app's high-precision timer applies system-wide.",
        details: "Not widely known. Since Windows 10 version 2004, a program that asks for a 1 ms system timer only gets it for itself, and Windows 11 also ignores the request when the program's window is hidden. Some games, frame limiters and capture tools relied on the old behaviour, where any program's request sharpened timing for everything. This registry switch, documented by Microsoft for Windows 11, brings that back.",
        kind: "1% lows",
        gain: "Situational. Can smooth frame pacing in older games and with external frame limiters such as RTSS. No change in most modern games, which request their own timer. Slightly higher idle power.",
        risk: "Compatibility and power trade-off",
        needs: "Reboot",
        actions: &[lm(r"SYSTEM\CurrentControlSet\Control\Session Manager\kernel", "GlobalTimerResolutionRequests", Dword(1), Delete)],
    },
    Tweak {
        id: "accessibility-shortcuts",
        category: "Gaming",
        title: "Turn off Sticky Keys shortcuts",
        summary: "Stops the Sticky Keys, Filter Keys and Toggle Keys pop-ups from interrupting games.",
        details: "Pressing Shift five times, or holding it for eight seconds, opens an accessibility prompt that pulls you out of a fullscreen game. This turns off the keyboard shortcuts that trigger those prompts. The features themselves stay available in Settings.",
        kind: "Fix",
        gain: "No FPS change. Ends the pop-up that minimises your game when you mash Shift.",
        risk: "Safe",
        needs: "Sign out",
        actions: &[
            cu(r"Control Panel\Accessibility\StickyKeys", "Flags", Str("506"), Str("510")),
            cu(r"Control Panel\Accessibility\Keyboard Response", "Flags", Str("122"), Str("126")),
            cu(r"Control Panel\Accessibility\ToggleKeys", "Flags", Str("58"), Str("62")),
        ],
    },
    Tweak {
        id: "vrr-windowed",
        category: "Gaming",
        title: "Variable refresh rate for games without support",
        summary: "Lets G-SYNC or FreeSync work in fullscreen DirectX 11 games that never added it.",
        details: "Few people know this setting exists. Some DirectX 11 games do not support variable refresh rate on their own, so they tear or need V-Sync. This turns on the Windows switch that adds it for them. It is the same switch as Settings > System > Display > Graphics > Variable refresh rate. It does nothing without a G-SYNC or FreeSync monitor. Undo removes the override and returns to the Windows default.",
        kind: "Latency",
        gain: "Tear-free play without V-Sync delay in the DX11 games it applies to. No change in games that already support variable refresh rate.",
        risk: "Safe",
        needs: "",
        actions: &[Action::Script {
            apply: r"$p='HKCU:\Software\Microsoft\DirectX\UserGpuPreferences'; if(!(Test-Path $p)){New-Item $p -Force | Out-Null}; $v=[string](Get-ItemProperty $p).DirectXUserGlobalSettings; Set-ItemProperty $p DirectXUserGlobalSettings (($v -replace 'VRROptimizeEnable=\d;','') + 'VRROptimizeEnable=1;')",
            undo: r"$p='HKCU:\Software\Microsoft\DirectX\UserGpuPreferences'; if(!(Test-Path $p)){New-Item $p -Force | Out-Null}; $v=[string](Get-ItemProperty $p).DirectXUserGlobalSettings; Set-ItemProperty $p DirectXUserGlobalSettings ($v -replace 'VRROptimizeEnable=\d;','')",
            check: r"[string](Get-ItemProperty 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -ErrorAction SilentlyContinue).DirectXUserGlobalSettings -match 'VRROptimizeEnable=1'",
        }],
    },
    Tweak {
        id: "fault-tolerant-heap",
        category: "Gaming",
        title: "Turn off the Fault Tolerant Heap",
        summary: "Stops Windows quietly slowing down a game after it has crashed a few times.",
        details: "Almost unknown outside game development. When a program crashes several times, Windows marks it and from then on runs its memory allocations through a slower, more forgiving path to hide the bug. The game keeps that penalty and nothing tells you. This turns the feature off, so games always run on the normal fast path. The cost: a buggy program that the feature was keeping alive may crash again.",
        kind: "1% lows",
        gain: "Nothing for games that have never crashed. For a game Windows had marked, it restores the performance that was lost.",
        risk: "May expose app crashes",
        needs: "Reboot",
        actions: &[lm(r"SOFTWARE\Microsoft\FTH", "Enabled", Dword(0), Dword(1))],
    },
    Tweak {
        id: "usb-suspend",
        category: "Gaming",
        title: "Turn off USB selective suspend",
        summary: "Stops Windows putting your mouse, keyboard, headset and controller to sleep.",
        details: "Windows powers down USB devices it thinks are idle. Waking one takes a moment, which shows up as a missed first input, a wireless headset cutting out, or a controller disconnecting. This turns the feature off for the power plan in use now; if you switch power plans later, apply it again. Costs a fraction of a watt on a desktop.",
        kind: "Fix",
        gain: "No FPS change. Removes input hiccups and device dropouts caused by USB power saving.",
        risk: "Safe",
        needs: "",
        actions: &[Action::Script {
            apply: "powercfg -setacvalueindex SCHEME_CURRENT 2a737441-1930-4402-8d77-b2bebba308a3 48e6b7a6-50f5-4782-a5d4-53bb8f07e226 0; powercfg -setactive SCHEME_CURRENT",
            undo: "powercfg -setacvalueindex SCHEME_CURRENT 2a737441-1930-4402-8d77-b2bebba308a3 48e6b7a6-50f5-4782-a5d4-53bb8f07e226 1; powercfg -setactive SCHEME_CURRENT",
            // The first line holding a hex value is the plugged-in setting, in any display language.
            check: "[string]@(powercfg -query SCHEME_CURRENT 2a737441-1930-4402-8d77-b2bebba308a3 48e6b7a6-50f5-4782-a5d4-53bb8f07e226 | Select-String '0x')[0] -match '0x0+$'",
        }],
    },
    Tweak {
        id: "pcie-aspm",
        category: "Gaming",
        title: "Turn off PCI Express link power saving",
        summary: "Keeps the link to the graphics card and NVMe drive at full power.",
        details: "On the Balanced plan Windows lets the PCI Express link to the graphics card, NVMe drive and network card drop into a low-power state when traffic is light, and waking it adds a small delay. This turns that off for the power plan in use now. The High and Ultimate Performance plans already have it off, so it will show as applied on those.",
        kind: "1% lows",
        gain: "Small. Can remove occasional hitches when the graphics card or drive wakes from an idle moment. A few watts more at idle.",
        risk: "Safe",
        needs: "",
        actions: &[Action::Script {
            apply: "powercfg -setacvalueindex SCHEME_CURRENT 501a4d13-42af-4429-9fd1-a8218c268e20 ee12f906-d277-404b-b6da-e5fa1a576df5 0; powercfg -setactive SCHEME_CURRENT",
            undo: "powercfg -setacvalueindex SCHEME_CURRENT 501a4d13-42af-4429-9fd1-a8218c268e20 ee12f906-d277-404b-b6da-e5fa1a576df5 1; powercfg -setactive SCHEME_CURRENT",
            check: "[string]@(powercfg -query SCHEME_CURRENT 501a4d13-42af-4429-9fd1-a8218c268e20 ee12f906-d277-404b-b6da-e5fa1a576df5 | Select-String '0x')[0] -match '0x0+$'",
        }],
    },
    // Documented, opt-in resource policies. Already-default settings need no change.
    Tweak {
        id: "search-backoff", category: "System", title: "Let indexing yield to active work",
        summary: "Keeps Search indexing, but lets it back off when Windows is busy.",
        details: "Disables the policy that forces indexing to ignore activity and resource pressure. Enhanced search stays available. This is already the unmanaged Windows default. Source: https://learn.microsoft.com/en-us/windows/client-management/mdm/policy-csp-search#disablebackoff",
        kind: "Responsiveness", gain: "Less indexing contention if backoff was disabled by another tool. No gain if it was already enabled.",
        risk: "Safe", needs: "Reboot",
        actions: &[lm(r"SOFTWARE\Policies\Microsoft\Windows\Windows Search", "DisableBackoff", Dword(0), Delete)],
    },
    Tweak {
        id: "edge-sleeping-tabs", category: "System", title: "Sleep inactive Edge tabs",
        summary: "Allows Edge to release resources from idle tabs while it stays open.",
        details: "Enables Edge's documented sleeping-tab policy. Uses the current timeout, exclusions and media heuristics, rather than forcing aggressive timeouts. Inactive pages can take a moment to resume. Source: https://learn.microsoft.com/en-us/deployedge/microsoft-edge-browser-policies/sleepingtabsenabled",
        kind: "Responsiveness", gain: "Less browser memory and background CPU for idle tabs. No change if already enabled; no guaranteed FPS gain.",
        risk: "Safe", needs: "Restart Edge",
        actions: &[cu(r"Software\Policies\Microsoft\Edge", "SleepingTabsEnabled", Dword(1), Delete)],
    },
    Tweak {
        id: "explorer-network-thumbnails", category: "System", title: "Skip thumbnail cache writes on network shares",
        summary: "Avoids writing hidden thumbs.db files to remote folders.",
        details: "Disables Explorer's network thumbnail disk cache. Local thumbnails and game texture quality are unaffected. Network folders may regenerate thumbnails on revisit. Source: https://learn.microsoft.com/en-us/windows/client-management/mdm/policy-csp-admx-thumbnails#disablethumbsdbonnetworkfolders",
        kind: "Responsiveness", gain: "Fewer small network writes while browsing media shares. No direct FPS improvement.",
        risk: "Safe", needs: "Restart Explorer",
        actions: &[cu(r"Software\Policies\Microsoft\Windows\Explorer", "DisableThumbsDBOnNetworkFolders", Dword(1), Delete)],
    },
    // ---------------- System ----------------
    Tweak {
        id: "menu-delay",
        category: "System",
        title: "Remove the menu delay",
        summary: "Makes submenus open the moment you point at them.",
        details: "Windows waits 400 milliseconds before opening a submenu, a holdover from slow computers. This sets the wait to zero.",
        kind: "Responsiveness",
        gain: "Menus feel instant. No change in games.",
        risk: "Safe",
        needs: "Sign out",
        actions: &[cu(DESKTOP, "MenuShowDelay", Str("0"), Str("400"))],
    },
    Tweak {
        id: "hibernation",
        category: "System",
        title: "Turn off hibernation",
        summary: "Frees the hibernation file; space saved depends on installed RAM.",
        details: "Windows reserves a hidden file, hiberfil.sys, sized at about 40% of your memory so it can hibernate. Space saved depends on RAM capacity and hibernation configuration. A desktop rarely needs hibernate; sleep keeps working. This also switches Fast Startup off, since it depends on the same file.",
        kind: "Fix",
        gain: "Disk space only; the amount varies by system. No FPS change.",
        risk: "Safe",
        needs: "",
        actions: &[Action::Script {
            apply: "powercfg -h off",
            undo: "powercfg -h on",
            check: r"(Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Power').HibernateEnabled -eq 0",
        }],
    },
    Tweak {
        id: "edge-background",
        category: "System",
        title: "Stop Edge running in the background",
        summary: "Prevents Microsoft Edge from preloading at sign-in and staying alive after you close it.",
        details: "Edge starts several hidden processes when you sign in (Startup boost) so it opens faster, and keeps extensions running after the last window closes. This turns both off by policy. Edge still works normally; it just opens a moment slower the first time. Edge will show 'managed by your organization' in its settings.",
        kind: "Responsiveness",
        gain: "A few hundred megabytes of memory and several background processes gone, whether or not you use Edge. Most useful with constrained memory; no guaranteed FPS gain.",
        risk: "Safe",
        needs: "Sign out",
        actions: &[
            lm(r"SOFTWARE\Policies\Microsoft\Edge", "StartupBoostEnabled", Dword(0), Delete),
            lm(r"SOFTWARE\Policies\Microsoft\Edge", "BackgroundModeEnabled", Dword(0), Delete),
        ],
    },
    Tweak {
        id: "tips",
        category: "System",
        title: "Turn off tips and suggestions",
        summary: "No 'finish setting up your PC' screens, Start suggestions or tip notifications.",
        details: "Turns off the suggestions Windows shows in Start and Settings, the tips and tricks notifications, the welcome screen after updates, and the recurring prompt to finish setting up your device.",
        kind: "Fix",
        gain: "Fewer interruptions and nags. No FPS change.",
        risk: "Safe",
        needs: "",
        actions: &[
            cu(CDM, "SubscribedContent-338389Enabled", Dword(0), Dword(1)),
            cu(CDM, "SubscribedContent-310093Enabled", Dword(0), Dword(1)),
            cu(CDM, "SystemPaneSuggestionsEnabled", Dword(0), Dword(1)),
            cu(CDM, "SoftLandingEnabled", Dword(0), Dword(1)),
            cu(r"Software\Microsoft\Windows\CurrentVersion\UserProfileEngagement", "ScoobeSystemSettingEnabled", Dword(0), Dword(1)),
        ],
    },
    Tweak {
        id: "visual-effects",
        category: "System",
        title: "Visual effects: best performance",
        summary: "Turns off window, menu and taskbar animations.",
        details: "Disables the animations Windows plays when windows open, minimise and snap, plus taskbar animations, list shadows and Aero Peek. Font smoothing is kept on so text still looks right. Windows and menus appear instantly instead of sliding in. The menu delay has its own switch.",
        kind: "Responsiveness",
        gain: "The desktop feels noticeably quicker. No change inside games.",
        risk: "Safe",
        needs: "Sign out",
        actions: &[
            cu(r"Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects", "VisualFXSetting", Dword(3), Dword(0)),
            cu(DESKTOP, "UserPreferencesMask", Bin(&[0x90, 0x12, 0x03, 0x80, 0x10, 0, 0, 0]), Delete),
            cu(DESKTOP, "DragFullWindows", Str("0"), Str("1")),
            // Windows' own "best performance" preset also switches font smoothing off; keep it on.
            cu(DESKTOP, "FontSmoothing", Str("2"), Str("2")),
            cu(r"Control Panel\Desktop\WindowMetrics", "MinAnimate", Str("0"), Str("1")),
            cu(ADVANCED, "TaskbarAnimations", Dword(0), Dword(1)),
            cu(ADVANCED, "ListviewAlphaSelect", Dword(0), Dword(1)),
            cu(ADVANCED, "ListviewShadow", Dword(0), Dword(1)),
            cu(r"Software\Microsoft\Windows\DWM", "EnableAeroPeek", Dword(0), Dword(1)),
        ],
    },
    Tweak {
        id: "transparency",
        category: "System",
        title: "Turn off transparency effects",
        summary: "Makes the taskbar, Start menu and window backgrounds solid.",
        details: "The blurred, see-through look of the taskbar and menus is drawn by the graphics card continuously. Turning it off removes that work.",
        kind: "Responsiveness",
        gain: "Negligible on any dedicated graphics card. Mostly a matter of taste; some people find solid surfaces easier to read.",
        risk: "Safe",
        needs: "",
        actions: &[cu(PERSONALIZE, "EnableTransparency", Dword(0), Dword(1))],
    },
    Tweak {
        id: "notifications",
        category: "System",
        title: "Turn off notifications",
        summary: "No pop-ups from apps or Windows, and no notification centre.",
        details: "Disables toast notifications and the notification centre for your account. A notification appearing over a fullscreen game can pull focus, drop the game out of fullscreen, or cost a frame-time spike. Note that this also hides the calendar flyout when you click the clock.",
        kind: "Fix",
        gain: "No FPS change. Removes interruptions and the focus-stealing they cause mid-game.",
        risk: "Safe",
        needs: "Sign out",
        actions: &[
            cu(r"Software\Microsoft\Windows\CurrentVersion\PushNotifications", "ToastEnabled", Dword(0), Dword(1)),
            cu(r"Software\Policies\Microsoft\Windows\Explorer", "DisableNotificationCenter", Dword(1), Delete),
        ],
    },
    Tweak {
        id: "startup-delay",
        category: "System",
        title: "Remove the startup app delay",
        summary: "Lets startup programs launch immediately after sign-in.",
        details: "Not widely known. Windows deliberately waits several seconds after you sign in before launching your startup programs, a leftover from the hard-drive era. On an SSD the wait is pure delay. This sets it to zero.",
        kind: "Responsiveness",
        gain: "The desktop is fully ready several seconds sooner after sign-in. No change in games.",
        risk: "Safe",
        needs: "",
        actions: &[cu(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Serialize", "StartupDelayInMSec", Dword(0), Delete)],
    },
    Tweak {
        id: "fast-startup",
        category: "System",
        title: "Turn off Fast Startup",
        summary: "Makes Shut down a real shutdown instead of a partial hibernate.",
        details: "With Fast Startup, 'Shut down' saves the Windows kernel and drivers to disk and reloads them next boot. Drivers therefore never start fresh, which is a common cause of problems that 'go away after a restart': audio glitches, USB devices misbehaving, a GPU driver update not fully applying. On an SSD the time saved is a couple of seconds.",
        kind: "Fix",
        gain: "No FPS change. Every boot starts clean, and several settings in this app that need a reboot will then apply after a normal shutdown too.",
        risk: "Safe",
        needs: "",
        actions: &[lm(r"SYSTEM\CurrentControlSet\Control\Session Manager\Power", "HiberbootEnabled", Dword(0), Dword(1))],
    },
    Tweak {
        id: "background-apps",
        category: "System",
        title: "Stop Store apps running in the background",
        summary: "Prevents Microsoft Store apps from running when you have not opened them.",
        details: "Store apps such as Phone Link, Weather and Xbox can wake in the background to sync and update tiles. This blocks that for your account. Desktop programs such as Discord and Steam are not affected. Mail and calendar Store apps will stop notifying you until opened.",
        kind: "Responsiveness",
        gain: "Small. A handful fewer processes waking up during play.",
        risk: "Safe",
        needs: "Sign out",
        actions: &[cu(r"Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications", "GlobalUserDisabled", Dword(1), Dword(0))],
    },
    // ---------------- Network ----------------
    Tweak {
        id: "network-throttling",
        category: "Network",
        title: "Turn off network throttling",
        summary: "Removes the cap Windows puts on network packets while audio or video plays.",
        details: "While any multimedia is playing, including game audio, Windows limits non-multimedia network traffic to about 10 packets per millisecond so that audio does not glitch. On a modern CPU that protection is unnecessary, and it can delay game packets. This removes the limit.",
        kind: "Latency",
        gain: "Evidence is anecdotal. It can smooth occasional ping spikes in online games; it does not lower your base ping or raise download speed.",
        risk: "Safe",
        needs: "Reboot",
        actions: &[lm(PROFILE, "NetworkThrottlingIndex", Dword(0xffff_ffff), Dword(10))],
    },
    Tweak {
        id: "nagle",
        category: "Network",
        title: "Turn off Nagle's algorithm",
        summary: "Sends small network packets immediately instead of batching them.",
        details: "Nagle's algorithm holds back small TCP packets briefly to combine them, and Windows also delays acknowledgements. Together these can add tens of milliseconds to games that use TCP. This turns both off on your active network adapters. Most shooters use UDP and are unaffected either way.",
        kind: "Latency",
        gain: "Noticeable in TCP-based games, which includes many MMOs. No effect in UDP games such as most competitive shooters.",
        risk: "Safe",
        needs: "Reboot",
        actions: &[Action::Script {
            apply: r"Get-NetAdapter -Physical | Where-Object Status -eq 'Up' | ForEach-Object { $p = 'HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces\' + $_.InterfaceGuid; Set-ItemProperty $p TcpAckFrequency 1 -Type DWord; Set-ItemProperty $p TCPNoDelay 1 -Type DWord }",
            undo: r"Get-NetAdapter -Physical | ForEach-Object { $p = 'HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces\' + $_.InterfaceGuid; Remove-ItemProperty $p TcpAckFrequency,TCPNoDelay -ErrorAction SilentlyContinue }",
            check: r"$a = @(Get-NetAdapter -Physical | Where-Object Status -eq 'Up'); $ok = $a.Count -gt 0; foreach ($n in $a) { $v = Get-ItemProperty ('HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces\' + $n.InterfaceGuid); if ($v.TcpAckFrequency -ne 1 -or $v.TCPNoDelay -ne 1) { $ok = $false } }; $ok",
        }],
    },
    Tweak {
        id: "delivery-optimization",
        category: "Network",
        title: "No update sharing with other devices",
        summary: "Stops Windows uploading and downloading updates to and from other PCs.",
        details: "Delivery Optimization lets your PC send pieces of Windows updates to other computers on your network and the internet, and fetch them the same way. This restricts updates to Microsoft's servers only, which also makes the bandwidth options under Advanced options irrelevant. The Settings page will show 'managed by your organization' while this is applied.",
        kind: "Latency",
        gain: "Prevents surprise upload traffic that raises ping while you play. No effect when no update is in progress.",
        risk: "Safe",
        needs: "",
        actions: &[lm(r"SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization", "DODownloadMode", Dword(0), Delete)],
    },
    // ---------------- Privacy & Search ----------------
    Tweak {
        id: "telemetry",
        category: "Privacy & Search",
        title: "Reduce telemetry",
        summary: "Turns off optional diagnostic data, the advertising ID and the telemetry service.",
        details: "Sets diagnostic data to the minimum Windows allows, disables the advertising ID, tailored experiences and feedback prompts, and disables the Connected User Experiences and Telemetry service. Windows Home still sends required diagnostic data; that cannot be turned off.",
        kind: "Privacy",
        gain: "Privacy, and one fewer service using disk and network in the background. No measurable FPS change.",
        risk: "Safe",
        needs: "Reboot",
        actions: &[
            lm(r"SOFTWARE\Policies\Microsoft\Windows\DataCollection", "AllowTelemetry", Dword(0), Delete),
            lm(r"SYSTEM\CurrentControlSet\Services\DiagTrack", "Start", Dword(4), Dword(2)),
            cu(r"Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo", "Enabled", Dword(0), Delete),
            cu(r"Software\Microsoft\Windows\CurrentVersion\Privacy", "TailoredExperiencesWithDiagnosticDataEnabled", Dword(0), Delete),
            cu(r"Software\Microsoft\Siuf\Rules", "NumberOfSIUFInPeriod", Dword(0), Delete),
            cu(ADVANCED, "Start_TrackProgs", Dword(0), Delete),
        ],
    },
    Tweak {
        id: "consumer-features",
        category: "Privacy & Search",
        title: "Block promoted app installs",
        summary: "Stops Windows installing suggested apps and games on its own.",
        details: "Windows periodically installs or pins promoted apps to the Start menu without asking. This policy turns that off. On Windows Home the policy is honoured less strictly than on Pro, so an occasional suggestion may still appear.",
        kind: "Privacy",
        gain: "Keeps unwanted apps, and their background tasks, off the PC. No FPS change.",
        risk: "Safe",
        needs: "",
        actions: &[lm(r"SOFTWARE\Policies\Microsoft\Windows\CloudContent", "DisableWindowsConsumerFeatures", Dword(1), Delete)],
    },
    Tweak {
        id: "activity-history",
        category: "Privacy & Search",
        title: "Turn off activity history",
        summary: "Stops Windows recording which apps and files you open.",
        details: "Windows keeps a timeline of the apps, files and sites you use and can upload it to your Microsoft account. This stops both the recording and the upload.",
        kind: "Privacy",
        gain: "Privacy only.",
        risk: "Safe",
        needs: "",
        actions: &[
            lm(ACTIVITY, "EnableActivityFeed", Dword(0), Delete),
            lm(ACTIVITY, "PublishUserActivities", Dword(0), Delete),
            lm(ACTIVITY, "UploadUserActivities", Dword(0), Delete),
        ],
    },
    // No Widgets tweak: Windows' UCPD driver denies writes to Policies\Microsoft\Dsh even to
    // administrators. Widgets is on the Remove apps page instead.
    Tweak {
        id: "copilot-recall",
        category: "Privacy & Search",
        title: "Turn off Copilot and Recall",
        summary: "Disables the built-in AI assistant and screen-snapshot history by policy.",
        details: "Sets the Windows policies that turn off the Copilot integration and prevent Recall from saving snapshots of your screen. On recent builds Copilot is also a separate app; remove that on the Remove apps page.",
        kind: "Privacy",
        gain: "Privacy, and no AI components running in the background. No FPS change.",
        risk: "Safe",
        needs: "Sign out",
        actions: &[
            cu(r"Software\Policies\Microsoft\Windows\WindowsCopilot", "TurnOffWindowsCopilot", Dword(1), Delete),
            lm(r"SOFTWARE\Policies\Microsoft\Windows\WindowsAI", "DisableAIDataAnalysis", Dword(1), Delete),
        ],
    },
    Tweak {
        id: "bing-search",
        category: "Privacy & Search",
        title: "Remove web results from Start search",
        summary: "Start menu search shows only your apps, files and settings.",
        details: "By default everything you type into Start is sent to Bing and web results are mixed into the list. This turns that off, so searches stay on your PC and results appear faster.",
        kind: "Responsiveness",
        gain: "Start search responds faster and nothing you type leaves the PC. No FPS change.",
        risk: "Safe",
        needs: "Sign out",
        actions: &[
            cu(SEARCH, "BingSearchEnabled", Dword(0), Dword(1)),
            cu(r"Software\Policies\Microsoft\Windows\Explorer", "DisableSearchBoxSuggestions", Dword(1), Delete),
        ],
    },
    Tweak {
        id: "search-highlights",
        category: "Privacy & Search",
        title: "Turn off search highlights",
        summary: "Removes the daily illustrations and trending content from the search box.",
        details: "Search highlights download and display promotional content in the search panel each day. This turns them off.",
        kind: "Privacy",
        gain: "A cleaner search panel and one fewer background download.",
        risk: "Safe",
        needs: "",
        actions: &[cu(SEARCH_SETTINGS, "IsDynamicSearchBoxEnabled", Dword(0), Dword(1))],
    },
    Tweak {
        id: "cloud-search",
        category: "Privacy & Search",
        title: "Turn off cloud content search",
        summary: "Stops Windows search looking in OneDrive, Outlook and other online accounts.",
        details: "Windows search can include results from your Microsoft account and work or school accounts. This limits search to this PC.",
        kind: "Privacy",
        gain: "Search stays local and returns results a little faster.",
        risk: "Safe",
        needs: "",
        actions: &[
            cu(SEARCH_SETTINGS, "IsMSACloudSearchEnabled", Dword(0), Dword(1)),
            cu(SEARCH_SETTINGS, "IsAADCloudSearchEnabled", Dword(0), Dword(1)),
        ],
    },
    Tweak {
        id: "search-history",
        category: "Privacy & Search",
        title: "Turn off search history",
        summary: "Windows no longer stores what you search for on this device.",
        details: "Stops Windows saving your local search history to improve suggestions.",
        kind: "Privacy",
        gain: "Privacy only.",
        risk: "Safe",
        needs: "",
        actions: &[cu(SEARCH_SETTINGS, "IsDeviceSearchHistoryEnabled", Dword(0), Dword(1))],
    },
    // ---------------- Appearance ----------------
    Tweak {
        id: "dark-apps",
        category: "Appearance",
        title: "Dark mode for apps",
        summary: "File Explorer, Settings and supporting apps use a dark background.",
        details: "Switches the app colour mode to dark. Apps that follow the Windows setting change immediately.",
        kind: "Look",
        gain: "Appearance only.",
        risk: "Safe",
        needs: "",
        actions: &[cu(PERSONALIZE, "AppsUseLightTheme", Dword(0), Dword(1))],
    },
    Tweak {
        id: "dark-system",
        category: "Appearance",
        title: "Dark mode for Windows",
        summary: "The taskbar, Start menu and system flyouts use a dark background.",
        details: "Switches the Windows colour mode to dark, separately from apps, so you can mix the two if you prefer.",
        kind: "Look",
        gain: "Appearance only.",
        risk: "Safe",
        needs: "",
        actions: &[cu(PERSONALIZE, "SystemUsesLightTheme", Dword(0), Dword(1))],
    },
    Tweak {
        id: "accent-color",
        category: "Appearance",
        title: "Accent colour on taskbar and title bars",
        summary: "Tints the taskbar, Start menu and window title bars with your accent colour.",
        details: "Shows your chosen accent colour on Start, the taskbar and window title bars instead of plain dark grey. The taskbar tint only shows when dark mode for Windows is on. Pick the colour itself in Settings > Personalization > Colors.",
        kind: "Look",
        gain: "Appearance only.",
        risk: "Safe",
        needs: "",
        actions: &[
            cu(PERSONALIZE, "ColorPrevalence", Dword(1), Dword(0)),
            cu(r"Software\Microsoft\Windows\DWM", "ColorPrevalence", Dword(1), Dword(0)),
        ],
    },
    Tweak {
        id: "file-extensions",
        category: "Appearance",
        title: "Show file extensions",
        summary: "File names show their type, such as .exe or .txt.",
        details: "Windows hides file extensions by default, which makes it easy to mistake a program for a document. Showing them is safer and makes config files easier to find.",
        kind: "Look",
        gain: "Clarity and a little safety. No performance change.",
        risk: "Safe",
        needs: "Restart Explorer",
        actions: &[cu(ADVANCED, "HideFileExt", Dword(0), Dword(1))],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Applies and undoes every tweak on this PC, then puts each one back exactly as it was.
    #[test]
    #[ignore = "changes real settings briefly; run elevated: cargo test -- --ignored --nocapture"]
    fn every_tweak_applies_and_reverts_live() {
        use crate::reg::{apply, is_on, restore, snapshot};
        let mut failures = Vec::new();
        for t in TWEAKS {
            let was_on = is_on(t.actions);
            let saved: Vec<_> = t.actions.iter().map(snapshot).collect();
            let result = (|| {
                apply(t.actions, true)?;
                if !is_on(t.actions) {
                    return Err("reads as not applied after apply".to_string());
                }
                apply(t.actions, false)?;
                if is_on(t.actions) {
                    return Err("still reads as applied after undo".to_string());
                }
                Ok(())
            })();
            if was_on {
                let _ = apply(t.actions, true); // scripts have no snapshot; re-apply instead
            }
            t.actions.iter().zip(saved).for_each(|(a, s)| restore(a, s));
            match result {
                Ok(()) => println!("ok    {}", t.id),
                Err(e) => {
                    println!("FAIL  {}: {e}", t.id);
                    failures.push(t.id);
                }
            }
        }
        assert!(failures.is_empty(), "failed: {failures:?}");
    }

    /// Two tweaks writing one value fight: turning either on or off makes the other read as
    /// "not applied", which looks like a tweak that will not stick.
    #[test]
    fn no_two_tweaks_write_the_same_value() {
        let mut owners = std::collections::HashMap::new();
        for t in TWEAKS {
            for a in t.actions {
                if let Action::Reg { hive, path, name, .. } = a {
                    let key = (matches!(hive, Hive::Hklm), path.to_lowercase(), name.to_lowercase());
                    if let Some(other) = owners.insert(key, t.id) {
                        assert_eq!(other, t.id, "{other} and {} both write {path}\\{name}", t.id);
                    }
                }
            }
        }
    }

    /// Windows' UCPD driver refuses these keys to every program, administrators included, so a
    /// tweak that writes them can only ever fail with "Access is denied".
    #[test]
    fn no_tweak_writes_a_key_windows_protects() {
        for t in TWEAKS {
            for a in t.actions {
                if let Action::Reg { path, name, .. } = a {
                    let target = format!("{path}\\{name}").to_lowercase();
                    for blocked in [r"policies\microsoft\dsh", r"explorer\advanced\taskbarda"] {
                        assert!(!target.contains(blocked), "{} writes protected {path}\\{name}", t.id);
                    }
                }
            }
        }
    }

    #[test]
    fn table_is_well_formed() {
        let mut ids = HashSet::new();
        for t in TWEAKS {
            assert!(ids.insert(t.id), "duplicate id {}", t.id);
            for text in [
                t.id, t.category, t.title, t.summary, t.details, t.kind, t.gain, t.risk,
            ] {
                assert!(!text.is_empty(), "{} has empty text", t.id);
            }
            assert!(!t.actions.is_empty(), "{} does nothing", t.id);
            for a in t.actions {
                if let Action::Script { apply, undo, check } = a {
                    for s in [apply, undo, check] {
                        assert!(!s.is_empty(), "{} has an empty script", t.id);
                        // reg::ps passes the script as one argument; a double quote would break it
                        assert!(!s.contains('"'), "{} script contains a double quote", t.id);
                    }
                }
            }
        }
    }
}
