module App

open Elmish
open Elmish.React
open Feliz
open Fable.Core.JsInterop
open Api

type Page =
    | Cat of string
    | CleanPage
    | StartupPage
    | AppsPage
    | ColorPage
    | GuidePage
    | RecoveryPage

type Model =
    { Page: Page
      Color: float
      Shadow: float
      AppliedColor: (float * float) option
      ColorRecovering: bool
      Hardware: Hardware option
      Profile: Guide.Profile
      Recovery: string
      Tweaks: Tweak[]
      Status: Map<string, bool> // missing = still reading
      Busy: Set<string>
      Open: Set<string>
      Notice: (bool * string) option
      Startup: StartupItem[]
      Apps: AppItem[] option
      Cleaners: CleanItem[] option
      Picked: Set<string>
      Removing: bool
      RestoreDone: bool // backend always verifies the persistent Windows baseline
      Pending: (string * bool) option // tweak waiting on the restore point
      RestoreErr: string option }

type Msg =
    | Loaded of Tweak[]
    | GotStatus of string * bool
    | Toggle of string * bool
    | RestoreOk
    | RestoreFailed of string
    | RetryRestore
    | CreateRestore
    | RecoveryLoaded of string
    | OpenRecovery
    | Detected of string
    | EditProfile of Guide.Profile
    | DismissNotice
    | Success of string
    | Cancel
    | Applied of string * bool * bool
    | ApplyFailed of string * string
    | Nav of Page
    | Expand of string
    | LoadStartup
    | StartupLoaded of StartupItem[]
    | SetStartup of StartupItem * bool
    | AppsLoaded of AppItem[]
    | Pick of string
    | RemovePicked
    | Removed of string[]
    | LoadCleaners
    | CleanersLoaded of CleanItem[]
    | RunCleaner of CleanItem
    | Cleaned of CleanItem * float
    | SetColor of float
    | SetShadow of float
    | ColorPreset of float * float
    | LiveColorDone of ColorReceipt
    | ResetColor
    | ClearLegacyColor
    | ColorOk of float * float
    | ColorRestored
    | ColorFailed of string
    | ColorLoaded of float[] option
    | Failed of string

let categories =
    [ "Gaming", "Frame rate, frame pacing and input delay in games."
      "System", "Desktop responsiveness and fewer interruptions. No in-game FPS change."
      "Network", "Latency consistency and bandwidth. Nothing here raises download speed."
      "Privacy & Search",
      "Less data sent to Microsoft and a quieter Start search. Enhanced file search is left as you set it in Settings > Privacy & security > Searching Windows."
      "Appearance", "Dark mode and small quality-of-life changes." ]

let call f arg ok =
    Cmd.OfPromise.either f arg ok (fun ex -> Failed(string ex))

let log ok text m =
    { m with Notice = Some(ok, text) }

let titleOf m id =
    m.Tweaks
    |> Array.tryFind (fun t -> t.id = id)
    |> Option.map (fun t -> t.title)
    |> Option.defaultValue id

let readStatus id =
    call tweakStatus id (fun s -> GotStatus(id, s))

let size (bytes: float) =
    if bytes >= 1073741824.0 then sprintf "%.1f GB" (bytes / 1073741824.0)
    else sprintf "%.0f MB" (bytes / 1048576.0)

let run (id, on) m =
    { m with Busy = m.Busy.Add id; Pending = None; RestoreErr = None },
    Cmd.OfPromise.either setTweak (id, on) (fun s -> Applied(id, on, s)) (fun ex -> ApplyFailed(id, string ex))

let scheduleColor immediate m =
    if not m.ColorRecovering then
        { m with Busy = m.Busy.Add "color" },
        call (if immediate then setColorPreset else setColorLive) (m.Color,m.Shadow) LiveColorDone
    else m, Cmd.none

let init () =
    { Page = Cat "Gaming"
      Color = 1.0
      Shadow = 1.0
      AppliedColor = None
      ColorRecovering = false
      Hardware = None
      Profile = Guide.empty
      Recovery = "Reading Windows System Protection..."
      Tweaks = [||]
      Status = Map.empty
      Busy = Set.empty
      Open = Set.empty
      Notice = None
      Startup = [||]
      Apps = None
      Cleaners = None
      Picked = Set.empty
      Removing = false
      RestoreDone = false
      Pending = None
      RestoreErr = None },
    Cmd.batch [
        call listTweaks () Loaded
        call hardware () Detected
        call recoveryStatus () RecoveryLoaded
        Cmd.OfPromise.either colorState () ColorLoaded (fun _ -> ColorLoaded None)
    ]

let update msg m =
    match msg with
    | Loaded ts -> { m with Tweaks = ts; Status = ts |> Array.map (fun t -> t.id, t.enabled) |> Map.ofArray }, Cmd.none
    | GotStatus(id, s) -> { m with Status = m.Status.Add(id, s) }, Cmd.none
    | Toggle(id, on) when m.RestoreDone -> run (id, on) m
    | Toggle(_, _) when m.Pending.IsSome || m.Busy.Contains "restore" -> m, Cmd.none
    | Toggle(id, on) ->
        { m with Pending = Some(id, on); Busy = m.Busy.Add id } |> log true "Verifying the restore baseline first. This can take a minute.",
        Cmd.OfPromise.either restorePoint () (fun _ -> RestoreOk) (fun ex -> RestoreFailed(string ex))
    | RestoreOk ->
        let m = { m with RestoreDone = true; Busy = m.Busy.Remove "restore" } |> log true "Baseline verified. Windows will reuse this restore point."
        match m.Pending with
        | Some p -> run p m
        | None -> m, call recoveryStatus () RecoveryLoaded
    | RestoreFailed e -> { m with RestoreErr = Some e; Busy = m.Busy.Remove "restore" } |> log false ("Restore point failed: " + e), Cmd.none
    | RetryRestore ->
        { m with RestoreErr = None }, Cmd.OfPromise.either restorePoint () (fun _ -> RestoreOk) (fun ex -> RestoreFailed(string ex))
    | CreateRestore when m.Busy.Contains "restore" || m.Pending.IsSome -> m, Cmd.none
    | CreateRestore ->
        { m with Busy = m.Busy.Add "restore" } |> log true "Verifying the baseline. This may take a minute.",
        Cmd.OfPromise.either restorePoint () (fun _ -> RestoreOk) (fun ex -> RestoreFailed(string ex))
    | RecoveryLoaded text ->
        let detail =
            try
                if text = "null" then None
                else
                    let p: obj = emitJsExpr text "JSON.parse($0)"
                    Some(sprintf "%s / sequence %s / %s" (string p?Description) (string p?SequenceNumber) (string p?CreationTime))
            with _ -> Some "Windows returned an unreadable restore point. Check System Protection."
        { m with Recovery = defaultArg detail "No OptiClean baseline exists yet. Create it before changing settings."; RestoreDone = detail.IsSome }, Cmd.none
    | OpenRecovery -> m, call openRecovery () (fun _ -> Success "System Restore opened. Select the OptiClean baseline and review affected programs before continuing.")
    | Detected text ->
        try
            let h: Hardware = emitJsExpr text "JSON.parse($0)"
            let p = m.Profile
            let fill old detected = if old = "" then detected else old
            let gpu = h.gpus |> Array.tryFind (fun g -> System.Text.RegularExpressions.Regex.IsMatch(g,"(GeForce|RTX|GTX|Radeon|Arc)",System.Text.RegularExpressions.RegexOptions.IgnoreCase)) |> Option.orElse (Array.tryHead h.gpus) |> Option.defaultValue ""
            { m with Hardware = Some h; Profile = { p with cpu = fill p.cpu h.cpu; gpu = fill p.gpu gpu; board = fill p.board h.board; bios = fill p.bios h.bios; psu = fill p.psu h.psu } }, Cmd.none
        with _ -> log false "Hardware detection was incomplete. Enter your specifications in BIOS & GPU." m, Cmd.none
    | EditProfile p -> { m with Profile = p }, Cmd.none
    | DismissNotice -> { m with Notice = None }, Cmd.none
    | Success text -> log true text m, Cmd.none
    | Cancel ->
        let busy = m.Pending |> Option.map (fun (id, _) -> m.Busy.Remove id) |> Option.defaultValue m.Busy
        { m with Pending = None; RestoreErr = None; Busy = busy }, Cmd.none
    | Applied(id, wanted, actual) ->
        let m = { m with Busy = m.Busy.Remove id; Status = m.Status.Add(id, actual) }
        let name = titleOf m id
        if wanted = actual then
            // Say what is still needed, so a change that waits for a sign-out does not look like it failed.
            let needs = m.Tweaks |> Array.tryFind (fun t -> t.id = id) |> Option.map (fun t -> t.needs) |> Option.defaultValue ""
            let finish = if needs = "" then "" else ". " + needs + " to finish"
            log true (name + (if actual then ": applied" else ": undone") + finish) m, Cmd.none
        else
            log false (name + ": Windows did not keep the change. Another program or a Windows policy may be holding this setting") m, Cmd.none
    | ApplyFailed(id, e) -> { m with Busy = m.Busy.Remove id } |> log false (titleOf m id + ": " + e), readStatus id
    | Nav p ->
        { m with Page = p; Notice = None },
        match p with
        | StartupPage -> Cmd.ofMsg LoadStartup
        | CleanPage -> Cmd.ofMsg LoadCleaners
        | AppsPage when m.Apps.IsNone -> call listApps () AppsLoaded
        | RecoveryPage -> call recoveryStatus () RecoveryLoaded
        | _ -> Cmd.none
    | Expand id -> { m with Open = (if m.Open.Contains id then m.Open.Remove id else m.Open.Add id) }, Cmd.none
    | LoadStartup -> m, call listStartup () StartupLoaded
    | StartupLoaded items -> { m with Startup = items }, Cmd.none
    | SetStartup(item, on) ->
        log true (item.name + ": updating startup setting...") m,
        call setStartup (item.source, item.name, on) (fun _ -> LoadStartup)
    | AppsLoaded apps -> { m with Apps = Some apps; Removing = false }, Cmd.none
    | Pick pkg -> { m with Picked = (if m.Picked.Contains pkg then m.Picked.Remove pkg else m.Picked.Add pkg) }, Cmd.none
    | RemovePicked -> { m with Removing = true }, call removeApps (Set.toArray m.Picked) Removed
    | Removed errors ->
        let m = { m with Picked = Set.empty }
        let m =
            if errors.Length = 0 then
                log true "Selected apps removed." m
            else
                errors |> Array.fold (fun m e -> log false e m) m
        m, call listApps () AppsLoaded
    | LoadCleaners -> m, call listCleaners () CleanersLoaded
    | CleanersLoaded items -> { m with Cleaners = Some items }, Cmd.none
    | RunCleaner c ->
        let key = "clean:" + c.id
        { m with Busy = m.Busy.Add key } |> log true (c.title + ": cleaning..."),
        Cmd.OfPromise.either runCleaner c.id (fun freed -> Cleaned(c, freed)) (fun ex -> ApplyFailed(key, c.title + ": " + string ex))
    | Cleaned(c, freed) ->
        { m with Busy = m.Busy.Remove("clean:" + c.id) }
        |> log true (c.title + (if c.sized then ": freed " + size freed else ": done")),
        Cmd.ofMsg LoadCleaners
    | SetColor _ when m.ColorRecovering -> m, Cmd.none
    | SetColor s -> scheduleColor false { m with Color = s }
    | SetShadow _ when m.ColorRecovering -> m, Cmd.none
    | SetShadow value -> scheduleColor false { m with Shadow = value }
    | ColorPreset(_, _) when m.ColorRecovering -> m, Cmd.none
    | ColorPreset(s,g) -> scheduleColor true { m with Color = s; Shadow = g }
    | LiveColorDone r when m.ColorRecovering || not r.current || (r.saturation,r.shadow) <> (m.Color,m.Shadow) -> m, Cmd.none
    | LiveColorDone r when r.status = "applied" ->
        { m with Busy = m.Busy.Remove "color"; AppliedColor = Some(r.saturation,r.shadow) }, Cmd.none
    | LiveColorDone r when r.status = "failed" ->
        { m with Busy = m.Busy.Remove "color" } |> log false r.error, Cmd.none
    | LiveColorDone _ -> m, Cmd.none
    | ResetColor when m.Busy.Contains "color" -> m, Cmd.none
    | ResetColor ->
        { m with Busy = m.Busy.Add "color"; ColorRecovering = true },
        Cmd.OfPromise.either resetColor () (fun _ -> ColorRestored) (fun ex -> ColorFailed(string ex))
    | ClearLegacyColor when m.Busy.Contains "color" -> m, Cmd.none
    | ClearLegacyColor ->
        if emitJsExpr<bool> () "window.confirm('Clear the old saturation, shadow and contrast filter? This also resets custom gamma/calibration. Use only to recover from the older app.')" then
            { m with Busy = m.Busy.Add "color"; ColorRecovering = true },
            Cmd.OfPromise.either clearLegacyColor () (fun _ -> ColorOk(1.0,1.0)) (fun ex -> ColorFailed(string ex))
        else m, Cmd.none
    | ColorOk(s,g) -> { m with Busy = m.Busy.Remove "color"; Color = s; Shadow = g; AppliedColor = Some(s,g); ColorRecovering = false } |> log true "Driver colors updated. You can close OptiClean while playing.", Cmd.none
    | ColorRestored -> { m with Busy = m.Busy.Remove "color"; Color = 1.0; Shadow = 1.0; AppliedColor = None; ColorRecovering = false } |> log true "Previous matrix, native vibrance and saved tone curve restored.", Cmd.none
    | ColorFailed e -> { m with Busy = m.Busy.Remove "color"; ColorRecovering = false } |> log false e, Cmd.none
    | ColorLoaded(Some v) when v.Length = 2 -> { m with Color = v.[0]; Shadow = v.[1]; AppliedColor = Some(v.[0], v.[1]) }, Cmd.none
    | ColorLoaded _ -> m, Cmd.none
    | Failed e -> { m with Removing = false } |> log false e, Cmd.none

// ---------- view ----------

let intro (text: string) (extra: ReactElement list) =
    Html.div [ prop.className "intro"; prop.children [ Html.p text; yield! extra ] ]

let slider (title: string) (sub: string) (shown: string) (lo: float) (hi: float) (value: float) (disabled: bool) (onChange: float -> unit) =
    Html.div [
        prop.className "row"
        prop.children [
            Html.div [
                prop.className "row-main"
                prop.children [
                    Html.span [ prop.className "row-title"; prop.text title ]
                    Html.span [ prop.className "row-sub wrap"; prop.text sub ]
                ]
            ]
            Html.input [ prop.type'.range; prop.disabled disabled; prop.ariaLabel title; prop.min lo; prop.max hi; prop.step 0.01; prop.value value; prop.onChange onChange ]
            Html.span [ prop.className "tally num"; prop.text shown ]
        ]
    ]

let colorPage m dispatch =
    let preset (label: string) s g =
        Html.button [
            prop.className ("btn" + (if m.Color = s && m.Shadow = g then " selected" else ""))
            prop.disabled m.ColorRecovering
            prop.onClick (fun _ -> dispatch (ColorPreset(s,g)))
            prop.text label
            if label = "Xtra" then prop.title "175% saturation. The most saturated colors can still clip slightly."
        ]
    [ Html.div [ prop.className "color-controls"; prop.children [
          Html.div [ prop.className "list"; prop.children [
              slider "Extended vibrance" "Neutralizes NVIDIA Digital Vibrance first, then applies this absolute color boost." (sprintf "%.0f%%" (m.Color * 100.0)) 1.0 2.75 m.Color m.ColorRecovering (fun v -> dispatch (SetColor v))
              slider "Shadow boost" "Lifts dark detail, fades toward highlights, and uses your saved tone curve. 1.00 keeps that curve." (sprintf "%.2f" m.Shadow) 1.0 1.8 m.Shadow m.ColorRecovering (fun v -> dispatch (SetShadow v))
          ] ]
          Html.div [ prop.className "preset-bar"; prop.children [ preset "Original" 1.0 1.0; preset "Competitive" 1.35 1.2; preset "Vivid" 1.55 1.1; preset "Xtra" 1.75 1.0 ] ]
      ] ] ]

let switch (isOn: bool) (busy: bool) (label: string) (onClick: unit -> unit) =
    Html.button [
        prop.className ("switch" + (if isOn then " on" else "") + (if busy then " busy" else ""))
        prop.role "switch"
        prop.ariaChecked isOn
        prop.ariaLabel label
        prop.disabled busy
        prop.onClick (fun _ -> onClick ())
        prop.children [ Html.span [ prop.className "knob" ] ]
    ]

let tweakRow m dispatch (t: Tweak) =
    let status = m.Status.TryFind t.id
    let busy = m.Busy.Contains t.id || m.Pending.IsSome || status.IsNone
    let isOpen = m.Open.Contains t.id

    Html.div [
        prop.key t.id
        prop.className ("item" + (if status = Some true then " on" else "") + (if isOpen then " open" else ""))
        prop.children [
            Html.div [
                prop.className "item-head"
                prop.children [
                    switch (status = Some true) busy t.title (fun () -> dispatch (Toggle(t.id, status <> Some true)))
                    Html.button [
                        prop.className "item-main"
                        prop.ariaExpanded isOpen
                        prop.onClick (fun _ -> dispatch (Expand t.id))
                        prop.children [
                            Html.span [ prop.className "item-title"; prop.text t.title ]
                            Html.span [
                                prop.className "item-meta"
                                prop.children [
                                    Html.span [ prop.className "chip"; prop.text t.kind ]
                                    if t.needs <> "" then
                                        Html.span [ prop.className "chip needs"; prop.text t.needs ]
                                    if t.risk <> "Safe" then
                                        Html.span [ prop.className "warn"; prop.text t.risk ]
                                ]
                            ]
                        ]
                    ]
                ]
            ]
            if isOpen then
                Html.div [
                    prop.className "item-body"
                    prop.children [
                        Html.p [ prop.className "lead"; prop.text t.summary ]
                        Html.h4 "What it does"
                        Html.p t.details
                        Html.h4 "Expected gain on your PC"
                        Html.p t.gain
                        Html.h4 "What changes"
                        Html.ul [ for c in t.changes do Html.li [ Html.code c ] ]
                    ]
                ]
        ]
    ]

let tweakPage m dispatch cat =
    let tweaks = m.Tweaks |> Array.filter (fun t -> t.category = cat)
    let applied = tweaks |> Array.filter (fun t -> m.Status.TryFind t.id = Some true) |> Array.length
    let sub = categories |> List.find (fun (c, _) -> c = cat) |> snd

    [ intro sub [ Html.span [ prop.className "tally"; prop.text (sprintf "%d of %d applied" applied tweaks.Length) ] ]
      Html.div [ prop.className "grid"; prop.children [ for t in tweaks do tweakRow m dispatch t ] ]
      Html.p [ prop.className "hint"; prop.text "Click a name to see what it does and what to expect." ] ]

let cleanPage m dispatch =
    [ intro "Free disk space. Nothing here changes settings, and none of it raises FPS; a full system drive does slow Windows down." []
      Html.div [
          prop.className "list"
          prop.children [
              match m.Cleaners with
              | None -> Html.p [ prop.className "empty"; prop.text "Measuring..." ]
              | Some items ->
                  for c in items do
                      let busy = m.Busy.Contains("clean:" + c.id)
                      Html.div [
                          prop.key c.id
                          prop.className "row"
                          prop.children [
                              Html.div [
                                  prop.className "row-main"
                                  prop.children [
                                      Html.span [ prop.className "row-title"; prop.text c.title ]
                                      Html.span [ prop.className "row-sub wrap"; prop.text c.details ]
                                  ]
                              ]
                              if c.sized then
                                  Html.span [ prop.className "tally"; prop.text (size c.bytes) ]
                              Html.button [
                                  prop.className "btn"
                                  prop.disabled busy
                                  prop.onClick (fun _ -> dispatch (RunCleaner c))
                                  prop.text (if busy then "Cleaning..." else "Clean")
                              ]
                          ]
                      ]
          ]
      ] ]

let startupPage m dispatch =
    [ intro "Programs that launch when you sign in. Turning one off uses the same switch as Task Manager; nothing is deleted." []
      Html.div [
          prop.className "list"
          prop.children [
              if m.Startup.Length = 0 then
                  Html.p [ prop.className "empty"; prop.text "No startup entries found." ]
              for s in m.Startup do
                  Html.div [
                      prop.key (s.source + s.name)
                      prop.className "row"
                      prop.children [
                          switch s.enabled false s.name (fun () -> dispatch (SetStartup(s, not s.enabled)))
                          Html.div [
                              prop.className "row-main"
                              prop.children [
                                  Html.span [ prop.className "row-title"; prop.text s.name ]
                                  Html.span [ prop.className "row-sub"; prop.title s.command; prop.text s.command ]
                              ]
                          ]
                          Html.span [ prop.className "tally"; prop.text s.source ]
                      ]
                  ]
          ]
      ] ]

let appsPage m dispatch =
    [ intro "Preinstalled Windows apps found on this PC. Removal cannot be undone from here; reinstall from the Microsoft Store if you change your mind." [
          Html.button [
              prop.className "btn danger"
              prop.disabled (m.Picked.IsEmpty || m.Removing)
              prop.onClick (fun _ -> dispatch RemovePicked)
              prop.text (if m.Removing then "Removing..." else sprintf "Remove %d selected" m.Picked.Count)
          ]
      ]
      Html.div [
          prop.className "grid"
          prop.children [
              match m.Apps with
              | None -> Html.p [ prop.className "empty"; prop.text "Reading installed apps..." ]
              | Some apps ->
                  if apps.Length = 0 then
                      Html.p [ prop.className "empty"; prop.text "None of the removable apps are installed." ]
                  for a in apps do
                      let picked = m.Picked.Contains a.pkg
                      Html.button [
                          prop.key a.pkg
                          prop.className ("row pick" + (if picked then " picked" else ""))
                          prop.role "checkbox"
                          prop.ariaChecked picked
                          prop.disabled m.Removing
                          prop.onClick (fun _ -> dispatch (Pick a.pkg))
                          prop.children [
                              Html.span [ prop.className "box" ]
                              Html.span [
                                  prop.className "row-main"
                                  prop.children [
                                      Html.span [ prop.className "row-title"; prop.text a.name ]
                                      Html.span [ prop.className "row-sub"; prop.text a.note ]
                                      if a.caution then
                                          Html.span [ prop.className "row-sub warn"; prop.text "Needed by some games" ]
                                  ]
                              ]
                          ]
                      ]
          ]
      ] ]

let guidePage m dispatch =
    let p = m.Profile
    let field (label: string) value change =
        Html.label [ prop.className "field"; prop.children [ Html.span label; Html.input [ prop.type'.text; prop.value (value: string); prop.maxLength 180; prop.onChange (fun (v: string) -> dispatch (EditProfile(change v))) ] ] ]
    let check (label: string) value change =
        Html.label [ prop.className "check-line"; prop.children [ Html.input [ prop.type'.checkbox; prop.isChecked value; prop.onChange (fun (v: bool) -> dispatch (EditProfile(change v))) ]; Html.span label ] ]
    [ intro "A plan for this PC. Detected fields remain editable; enter anything Windows could not identify. This form stays in memory and is never uploaded." []
      Html.div [ prop.className "hardware-form"; prop.children [
          field "CPU model" p.cpu (fun v -> { p with cpu = v })
          field "GPU model (card being tuned)" p.gpu (fun v -> { p with gpu = v })
          field "Motherboard make / model" p.board (fun v -> { p with board = v })
          field "BIOS version" p.bios (fun v -> { p with bios = v })
          field "PSU make / exact model (read its label)" p.psu (fun v -> { p with psu = v })
          field "PSU rated watts" p.watts (fun v -> { p with watts = v })
          field "CPU cooler model" p.cooler (fun v -> { p with cooler = v })
          field "CPU stock load temperature (C)" p.cpuTemp (fun v -> { p with cpuTemp = v })
          field "GPU stock load temperature (C)" p.gpuTemp (fun v -> { p with gpuTemp = v })
      ] ]
      match m.Hardware with
      | Some h -> Html.p [ prop.className "hint"; prop.text ("Detected GPUs: " + String.concat " / " h.gpus + (if h.portable then ". Portable system detected: keep stock settings." else "")) ]
      | None -> Html.p [ prop.className "hint"; prop.text "Detection unavailable or still running. Enter your hardware manually." ]
      Html.div [ prop.className "checks"; prop.children [
          check "This is a desktop with user-serviceable cooling and power (not a laptop)." p.desktop (fun v -> { p with desktop = v })
          check "I verified PSU health, required connectors and headroom against CPU/GPU vendor requirements." p.headroom (fun v -> { p with headroom = v })
          check "Stable at stock: no WHEA errors, artifacts or driver resets." p.stable (fun v -> { p with stable = v })
          check "The exact CPU/GPU/board manuals permit the tuning feature I plan to use." p.support (fun v -> { p with support = v })
      ] ]
      Html.div [ prop.className "guide"; prop.children [
          for section in Guide.sections p do
              Html.section [ prop.key section.title; prop.children [
                  Html.h2 section.title; Html.p section.intro
                  for item in section.items do
                      Html.div [ prop.className "guide-item"; prop.children [ Html.h3 item.name; Html.p [ prop.className "set"; prop.text item.set ]; Html.p item.why ] ]
              ] ]
      ] ] ]

let recoveryPage m dispatch =
    let verifying = m.Busy.Contains "restore"
    [ intro "One shared baseline for OptiClean changes, verified in Windows and reused across app launches. If Windows expires or deletes it, the next change needs a new baseline." []
      Html.div [ prop.className "list"; prop.children [
          Html.div [ prop.className "row"; prop.children [
              Html.div [ prop.className "row-main"; prop.children [
                  Html.span [ prop.className "row-title"; prop.text (if m.RestoreDone then "Baseline restore point" else "No baseline") ]
                  Html.span [ prop.className "row-sub wrap"; prop.text m.Recovery ] ] ]
              if not m.RestoreDone || verifying then
                  Html.button [ prop.className "btn"; prop.disabled (verifying || m.Pending.IsSome); prop.onClick (fun _ -> dispatch CreateRestore); prop.text (if verifying then "Verifying..." else "Create baseline") ]
              Html.button [ prop.className "btn primary"; prop.onClick (fun _ -> dispatch OpenRecovery); prop.text "Open System Restore" ] ] ] ] ]
      Html.p [ prop.className "hint"; prop.text "In Windows, choose OptiClean baseline (or the legacy OptiClean point), review affected programs and confirm there. The app does not restore or reboot automatically." ]
      Html.p [ prop.className "hint"; prop.text "System Restore protects system settings and installed components. It does not back up personal files, recover cleaned files, or undo firmware/GPU clocks. Per-tweak Undo uses documented defaults, which may differ from your custom settings. Windows' normal restore-point frequency policy is respected." ] ]

let tab m dispatch (page: Page) (label: string) =
    Html.button [
        prop.className ("tab" + (if m.Page = page then " active" else ""))
        prop.role "tab"
        prop.ariaSelected ((m.Page = page))
        prop.onClick (fun _ -> dispatch (Nav page))
        prop.children [
            Html.span label
            // Tweak categories show how many of their switches are on.
            match page with
            | Cat c ->
                let mine = m.Tweaks |> Array.filter (fun t -> t.category = c)
                let on = mine |> Array.filter (fun t -> m.Status.TryFind t.id = Some true) |> Array.length
                if mine.Length > 0 then
                    Html.span [ prop.className "badge"; prop.text (sprintf "%d/%d" on mine.Length) ]
            | _ -> ()
        ]
    ]

let view m dispatch =
    let applied = m.Status |> Map.filter (fun _ on -> on) |> Map.count
    let pageTitle = match m.Page with | Cat c -> c | CleanPage -> "Cleanup" | StartupPage -> "Startup apps" | AppsPage -> "Remove apps" | ColorPage -> "Color studio" | GuidePage -> "BIOS & GPU advisor" | RecoveryPage -> "Recovery"

    Html.div [
        prop.className "app"
        prop.children [
            Html.div [
                prop.className "titlebar"
                prop.custom ("data-tauri-drag-region", "")
                prop.children [
                    Html.span [ prop.className "wordmark"; prop.text "OptiClean" ]
                    Html.button [ prop.className "win min"; prop.ariaLabel "Minimize"; prop.onClick (fun _ -> minimizeWindow ()) ]
                    Html.button [ prop.className "win close"; prop.ariaLabel "Close"; prop.onClick (fun _ -> closeWindow ()) ]
                ]
            ]
            Html.header [
                prop.className "top"
                prop.children [
                    Html.span [ prop.className "logo" ]
                    Html.div [
                        prop.className "brand"
                        prop.children [
                            Html.h1 "OptiClean"
                            Html.div [
                                prop.className "brand-sub"
                                prop.children [
                                    Html.span (sprintf "%d of %d tweaks applied" applied m.Tweaks.Length)
                                    Html.div [
                                        prop.className "meter"
                                        prop.ariaHidden true
                                        prop.children [ Html.span [ prop.style [ style.width (length.percent (if m.Tweaks.Length = 0 then 0 else applied * 100 / m.Tweaks.Length)) ] ] ]
                                    ]
                                ]
                            ]
                        ]
                    ]
                    match m.Hardware with
                    | Some h ->
                        let spec (label: string) (value: string) =
                            Html.div [ prop.className "spec"; prop.children [
                                Html.span [ prop.className "spec-label"; prop.text label ]
                                Html.span [ prop.className "spec-value"; prop.title value; prop.text value ] ] ]
                        Html.div [ prop.className "specs"; prop.children [
                            spec "CPU" h.cpu
                            spec "GPU" (String.concat " / " h.gpus)
                            spec "Memory" (sprintf "%d GB / %d MT/s" h.ramGb h.ramSpeed)
                            spec "Board" h.board ] ]
                    | None -> Html.div [ prop.className "specs pending"; prop.text "Detecting your hardware..." ]
                ]
            ]
            Html.nav [
                prop.className "tabs"
                prop.role "tablist"
                prop.children [
                    for (c, _) in categories do
                        tab m dispatch (Cat c) c
                    Html.span [ prop.className "tab-gap"; prop.ariaHidden true ]
                    tab m dispatch CleanPage "Cleanup"
                    tab m dispatch StartupPage "Startup"
                    tab m dispatch AppsPage "Apps"
                    tab m dispatch ColorPage "Colour"
                    tab m dispatch GuidePage "BIOS & GPU"
                    tab m dispatch RecoveryPage "Recovery"
                ]
            ]
            Html.main [
                prop.key pageTitle
                prop.children [
                    Html.div [ prop.className "page-heading"; prop.children [ Html.div [ prop.children [ Html.span [ prop.className "eyebrow"; prop.text "OPTICLEAN / YOUR SYSTEM, REFINED" ]; Html.h2 pageTitle ] ] ] ]
                    match m.Notice with
                    | Some(ok, text) ->
                        Html.div [ prop.className ("notice" + (if ok then "" else " error")); prop.role "status"; prop.children [ Html.span text; Html.button [ prop.className "dismiss"; prop.ariaLabel "Dismiss message"; prop.onClick (fun _ -> dispatch DismissNotice); prop.text "x" ] ] ]
                    | None -> ()
                    yield!
                        match m.Page with
                        | Cat c -> tweakPage m dispatch c
                        | CleanPage -> cleanPage m dispatch
                        | StartupPage -> startupPage m dispatch
                        | AppsPage -> appsPage m dispatch
                        | ColorPage -> colorPage m dispatch
                        | GuidePage -> guidePage m dispatch
                        | RecoveryPage -> recoveryPage m dispatch
                ]
            ]
            match m.RestoreErr with
            | None -> Html.none
            | Some e ->
                Html.div [
                    prop.className "overlay"
                    prop.children [
                        Html.div [
                            prop.className "modal"
                            prop.role "dialog"
                            prop.children [
                                Html.h2 "Could not create a restore point"
                                Html.p e
                                Html.p "No pending tweak was applied. Enable System Protection or wait for the Windows daily restore-point limit, then retry."
                                Html.div [
                                    prop.className "modal-actions"
                                    prop.children [
                                        Html.button [ prop.className "btn"; prop.onClick (fun _ -> dispatch Cancel); prop.text "Cancel" ]
                                        Html.button [ prop.className "btn primary"; prop.onClick (fun _ -> dispatch RetryRestore); prop.text "Retry" ]
                                    ]
                                ]
                            ]
                        ]
                    ]
                ]
        ]
    ]

Program.mkProgram init update view
|> Program.withReactSynchronous "root"
|> Program.run
