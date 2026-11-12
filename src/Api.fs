module Api

open Fable.Core
open Fable.Core.JsInterop

// Field names match the JSON the Rust commands return.
type Tweak =
    { id: string
      category: string
      title: string
      summary: string
      details: string
      kind: string
      gain: string
      risk: string
      needs: string
      enabled: bool
      changes: string[] }

type StartupItem =
    { name: string
      command: string
      source: string
      enabled: bool }

type AppItem =
    { pkg: string
      name: string
      note: string
      caution: bool }

[<Import("invoke", "@tauri-apps/api/core")>]
let private invoke<'T> (cmd: string, args: obj) : JS.Promise<'T> = jsNative

let listTweaks () = invoke<Tweak[]> ("list_tweaks", null)
let tweakStatus (id: string) = invoke<bool> ("tweak_status", {| id = id |})
let setTweak (id: string, on: bool) = invoke<bool> ("set_tweak", {| id = id; on = on |})
let restorePoint () = invoke<unit> ("restore_point", null)
let listStartup () = invoke<StartupItem[]> ("list_startup", null)

let setStartup (source: string, name: string, enabled: bool) =
    invoke<unit> ("set_startup", {| source = source; name = name; enabled = enabled |})

type CleanItem =
    { id: string
      title: string
      details: string
      bytes: float
      sized: bool }

let listCleaners () = invoke<CleanItem[]> ("list_cleaners", null)
let runCleaner (id: string) = invoke<float> ("run_cleaner", {| id = id |})
let setColor (saturation: float, shadow: float) = invoke<unit> ("set_color", {| saturation = saturation; shadow = shadow |})

type ColorReceipt =
    { status: string; current: bool; saturation: float; shadow: float; error: string }
type ColorQueue =
    abstract enqueue: float[] * bool -> JS.Promise<ColorReceipt>
    abstract cancel: unit -> JS.Promise<unit>

[<Import("createColorQueue", "./color-live.js")>]
let private createColorQueue (apply: float[] -> JS.Promise<unit>) : ColorQueue = jsNative

let private colorQueue = createColorQueue (fun values -> setColor (values.[0], values.[1]))
let setColorLive (s: float, g: float) = colorQueue.enqueue([| s; g |], false)
let setColorPreset (s: float, g: float) = colorQueue.enqueue([| s; g |], true)
let resetColor () : JS.Promise<unit> =
    emitJsExpr (colorQueue, fun () -> invoke<unit> ("reset_color", null)) "$0.cancel().then($1)"
let clearLegacyColor () : JS.Promise<unit> =
    emitJsExpr (colorQueue, fun () -> invoke<unit> ("clear_legacy_color", null)) "$0.cancel().then($1)"
let recoveryStatus () = invoke<string> ("recovery_status", null)
let openRecovery () = invoke<unit> ("open_recovery", null)

type Hardware =
    { cpu: string; gpus: string[]; board: string; bios: string
      ramGb: int; ramSpeed: int; portable: bool; psu: string }
let hardware () = invoke<string> ("hardware", null)

let listApps () = invoke<AppItem[]> ("list_apps", null)
let removeApps (pkgs: string[]) = invoke<string[]> ("remove_apps", {| pkgs = pkgs |})
