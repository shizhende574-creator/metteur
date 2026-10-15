param([Parameter(Mandatory=$true)][string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
if (-not [System.IO.Path]::IsPathRooted($OutputDirectory) -or (Test-Path $OutputDirectory)) { throw 'Choose an absolute NEW output directory.' }
if ($env:OS -ne 'Windows_NT') { throw 'This script records native Windows evidence only.' }
$root = (& git rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Run inside the Metteur repository.' }
Set-Location $root
if (& git status --porcelain) { throw 'Preserve existing changes before checking a frozen candidate.' }
New-Item -ItemType Directory $OutputDirectory | Out-Null
& git rev-parse HEAD | Out-File "$OutputDirectory/source-head.txt"
$PSVersionTable | Out-File "$OutputDirectory/platform.txt"
$env:CARGO_BUILD_JOBS = '1'
$cases = @(
 @('lock','--lib','workspace::lock::tests'),
 @('storage','--test','nx_storage','additive_migration_cas_replay_restart_and_rollback'),
 @('recovery','--test','recovery_admission','unreadable_chat_checkpoint_stops_the_turn_before_tool_effects')
)
foreach ($case in $cases) {
 $name=$case[0]; $arguments=$case[1..($case.Length-1)]
 & cargo test --locked -p metteur-daemon @arguments -- --nocapture *> "$OutputDirectory/$name.log"
 if ($LASTEXITCODE -ne 0) { throw "$name failed; preserve logs and stop." }
}
'Native Windows targeted cases passed; not a full platform suite.' | Out-File "$OutputDirectory/RESULT.txt"
