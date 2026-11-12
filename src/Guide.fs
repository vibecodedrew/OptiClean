module Guide

open System
open System.Text.RegularExpressions

type Item = { name: string; set: string; why: string }
type Section = { title: string; intro: string; items: Item list }
type Profile =
    { cpu: string; gpu: string; board: string; bios: string; psu: string; watts: string
      cooler: string; cpuTemp: string; gpuTemp: string
      desktop: bool; headroom: bool; stable: bool; support: bool }

let empty =
    { cpu = ""; gpu = ""; board = ""; bios = ""; psu = ""; watts = ""
      cooler = ""; cpuTemp = ""; gpuTemp = ""
      desktop = false; headroom = false; stable = false; support = false }

let matches (input: string, pattern: string) = Regex.IsMatch(input, pattern, RegexOptions.IgnoreCase)

let validName (s: string) =
    not (String.IsNullOrWhiteSpace s) && s.Trim().Length >= 3 &&
    not (matches(s, "^(unknown|n/?a|none|default string|to be filled.*|system product name)$"))
let number (s: string) =
    match Double.TryParse s with
    | true, n when not (Double.IsNaN n || Double.IsInfinity n) -> Some n
    | _ -> None

let missing p =
    [ for (label, value) in ["CPU", p.cpu; "GPU", p.gpu; "motherboard", p.board; "BIOS version", p.bios; "PSU make/model", p.psu; "CPU cooler", p.cooler] do
          if not (validName value) then yield label
      match number p.watts with | Some w when w >= 150. && w <= 2500. -> () | _ -> yield "PSU rated watts (150-2500)"
      match number p.cpuTemp with | Some t when t >= 10. && t <= 110. -> () | _ -> yield "measured CPU load temperature"
      match number p.gpuTemp with | Some t when t >= 10. && t <= 110. -> () | _ -> yield "measured GPU load temperature" ]

let ready p =
    List.isEmpty (missing p) && p.desktop && p.headroom && p.stable && p.support &&
    (number p.cpuTemp |> Option.exists (fun t -> t <= 80.)) &&
    (number p.gpuTemp |> Option.exists (fun t -> t <= 75.))

let sections p =
    let amd = matches(p.cpu, "ryzen")
    let intel = matches(p.cpu, "intel")
    let nvidia = matches(p.gpu, "(geforce|RTX|GTX)")
    let radeon = matches(p.gpu, "radeon")
    let portable = matches(p.gpu + " " + p.cpu + " " + p.board, "(laptop|mobile|notebook|max-q|[0-9](U|H|HX|HS)\\b)")
    let ryzenCo = amd && matches(p.cpu, "ryzen\\s+[3579]\\s+[579][0-9]{3}") &&
                  not (matches(p.cpu, "(X3D|Threadripper)")) &&
                  matches(p.board, "\\b(B[56][0-9]{2}|X[568][0-9]{2})")
    [ { title = "Your tuning baseline"
        intro = "This plan uses the specifications above. Windows normally cannot identify an ATX PSU or cooler; inspect the label without opening the PSU. Names and wattage cannot prove electrical headroom or stability. No firmware or clock settings are applied by this app."
        items = [
            { name = "Record and measure first"
              set = "Save your current BIOS profile and GPU profile. Benchmark the same replay or scene at stock, then change one setting at a time. Retain security, thermal and current protections."
              why = "A restore point cannot undo a BIOS or GPU overclock. Know the exact board's reset procedure from its manual; do not short unidentified pins." }
            { name = "Preserve the intended image"
              set = "Keep texture filtering at Quality, native render resolution, and dynamic resolution off when comparing results. Keep a warm shader cache."
              why = "High Performance texture filtering, shader-cache clearing and resolution reductions can sacrifice quality or add stutter. They are not free performance." } ] }
      { title = "BIOS and driver checks"
        intro = if amd then "AMD processor detected. Menus and supported features depend on the exact board and BIOS." elif intel then "Intel processor detected. Unlocked CPU and chipset support must both be verified." else "Processor support is unverified. Keep CPU clocks and voltages at factory defaults."
        items = [
            { name = "Memory profile: verify support"
              set = "Compare the memory kit's rated profile with the CPU memory specification and board QVL. If supported, try the kit's own XMP / EXPO / DOCP profile, then test memory. Do not invent timings or voltages."
              why = "A memory profile can improve CPU-bound performance, but is an overclock when beyond the CPU's specification. Revert after any memory error, WHEA event or boot problem." }
            { name = "Resizable BAR / Smart Access Memory"
              set = "Only if CPU, GPU, firmware and board support it: verify Windows already uses UEFI, then follow the board manual for Above 4G Decoding and Re-Size BAR. Verify in the GPU driver."
              why = "Gains vary by game. Do not switch a legacy installation to UEFI blindly; this can stop Windows booting. Secure Boot, TPM and BitLocker recovery access must be preserved." }
            { name = "Reduce render queue in the game"
              set = if nvidia then "Use NVIDIA Reflex in Fortnite / Call of Duty when offered. Start with On; benchmark Boost separately because it can increase power." elif radeon then "Use the supported Radeon Anti-Lag setting for the individual game; verify compatibility in the driver release notes." else "Choose the game's built-in low-latency option when supported; use per-game graphics preferences on hybrid systems."
              why = "Latency settings are game dependent. Keep texture filtering at Quality and avoid global maximum-power overrides." } ] }
      { title = if ready p && not portable then "Conservative manual OC starting point" else "OC recommendation: stay at stock for now"
        intro = "No overclock or undervolt is guaranteed safe or stable, and either may affect warranty. Temperatures below these planning thresholds are prerequisites, not proof that tuning is safe. Follow any lower vendor limit."
        items =
          if not (List.isEmpty (missing p)) then
              [ { name = "Complete the missing information"; set = String.concat ", " (missing p); why = "No clock, voltage or power recommendation is made from incomplete hardware information." } ]
          elif not (ready p) || portable then
              [ { name = "Keep factory clocks and voltage"
                  set = "Confirm a desktop with vendor-supported tuning, a healthy adequately sized PSU and connectors, cooling headroom, and error-free stock operation. If CPU exceeds 80 C or GPU exceeds 75 C during the stock baseline, address cooling first."
                  why = "Laptops, OEM power constraints, unknown compatibility or existing instability make a generic OC inappropriate." } ]
          else [
              { name = "CPU"
                set = if ryzenCo then "If the exact BIOS explicitly supports Curve Optimizer for this CPU: try a negative magnitude of 5 only, with stock PPT/TDC/EDC limits and boost override 0 MHz. Treat this as an experiment, not a validated setting. Otherwise keep stock boost." else "Keep factory automatic boost and stock power/current limits. No generic fixed multiplier or voltage is recommended for this CPU/board combination."
                why = "Never select motherboard/unlimited power limits. An undervolt can fail at idle or light load even if a heavy stress test passes." }
              { name = "GPU"
                set = if nvidia || radeon then "If the vendor tuning tool allows it on this exact card: start at +15 MHz core, +0 memory, stock voltage and stock power limit. Test before considering any further step. Do not enable Apply at startup until thoroughly validated." else "GPU tuning support is unverified. Keep factory clocks, voltage and power limits."
                why = "Even a tiny offset can be unstable and may yield no measurable gain. There is no universal maximum OC based on a model name or PSU wattage." }
              { name = "Validate and roll back"
                set = "Repeat your baseline game and a suitable CPU/GPU stress test for at least 30 minutes; test idle, sleep/wake and several games over subsequent days. Watch temperatures, WHEA errors, artifacts and driver resets. Revert immediately on any error or performance regression."
                why = "Keep whichever configuration has repeatably better frame times without errors. A short stress test alone never proves long-term stability." } ] } ]
