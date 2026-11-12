//! Windows owns the backup; no session flag or timestamp is treated as proof it exists.
use crate::reg;

const QUERY: &str = r"Get-ComputerRestorePoint | Where-Object { $_.Description -eq 'OptiClean baseline' -or $_.Description -eq 'OptiClean' } | Sort-Object SequenceNumber | Select-Object -First 1";

pub fn status() -> Result<String, String> {
    reg::ps(&format!(
        r"$p=@({QUERY}); if($p.Count){{ $p[0] | Select-Object Description,SequenceNumber,CreationTime | ConvertTo-Json -Compress }}else{{'null'}}"
    ))
}

fn ensure_script() -> String {
    // A machine-wide named mutex also serializes separate elevated app processes.
    format!(
        r"
$mutex=[System.Threading.Mutex]::new($false,'Global\OptiClean.RestorePoint');
$held=$false;
try {{
  try {{ $held=$mutex.WaitOne(180000) }} catch [System.Threading.AbandonedMutexException] {{ $held=$true }};
  if(!$held){{throw 'Another OptiClean instance is creating the baseline. Try again shortly.'}};
  $p=@({QUERY});
  if(!$p.Count){{
    Enable-ComputerRestore -Drive ($env:SystemDrive + '\');
    Checkpoint-Computer -Description 'OptiClean baseline' -RestorePointType MODIFY_SETTINGS -WarningAction Stop;
    $p=@({QUERY});
    if(!$p.Count){{throw 'Windows did not create a restore point. Check System Protection and the Windows daily restore-point limit.'}}
  }}
}} finally {{ if($held){{$mutex.ReleaseMutex()}}; $mutex.Dispose() }}"
    )
}

pub fn ensure() -> Result<(), String> {
    reg::ps(&ensure_script())?;
    Ok(())
}

pub fn open() -> Result<(), String> {
    // No unattended restore or reboot. The user reviews the baseline in Windows.
    reg::ps(r"Start-Process -FilePath ($env:SystemRoot + '\System32\rstrui.exe')")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // These functions shadow the Windows cmdlets. No real restore settings are touched.
    const MOCKS: &str = r"
$script:points=@(); $script:created=0;
function Get-ComputerRestorePoint { $script:points }
function Enable-ComputerRestore { param($Drive) }
function Checkpoint-Computer {
  param($Description,$RestorePointType,$WarningAction)
  $script:created++; $script:points=@([pscustomobject]@{Description=$Description;SequenceNumber=1})
}
";

    #[test]
    fn repeated_requests_reuse_the_verified_point() {
        let script = format!(
            "{MOCKS}\n{}\n{}\n$script:created",
            ensure_script(),
            ensure_script()
        );
        let script = script.replace(
            "Global\\OptiClean.RestorePoint",
            &format!("Local\\OptiClean.Test.Reuse.{}", std::process::id()),
        );
        assert_eq!(reg::ps(&script).unwrap(), "1");
    }

    #[test]
    fn absent_checkpoint_never_reports_success_and_releases_lock() {
        let script = format!(
            r"{MOCKS}
function Checkpoint-Computer {{ param($Description,$RestorePointType,$WarningAction) }}
$failed=$false; try {{ {} }} catch {{ $failed=$true }};
if(!$failed){{throw 'Missing checkpoint was accepted'}};
$mutex=[System.Threading.Mutex]::new($false,'Global\OptiClean.RestorePoint');
$held=$mutex.WaitOne(0); if($held){{$mutex.ReleaseMutex()}}; $mutex.Dispose(); $held",
            ensure_script()
        );
        let script = script.replace(
            "Global\\OptiClean.RestorePoint",
            &format!("Local\\OptiClean.Test.Failure.{}", std::process::id()),
        );
        assert_eq!(reg::ps(&script).unwrap(), "True");
    }
}
