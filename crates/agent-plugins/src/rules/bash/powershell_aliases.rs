//! Ported from `src/rules/bash/powershell-aliases.ts`; only the core uses it now, so it has no wrapper.

use indexmap::IndexMap;
use std::sync::LazyLock;

/// permissions.md ("PowerShell"): "Common aliases are canonicalized before matching ... so
/// `PowerShell(Get-ChildItem *)` matches `gci`, `ls`, and `dir` as well." The docs name those three;
/// the rest are PowerShell's own built-in aliases (`Get-Alias` on Windows PowerShell 5.1 and
/// PowerShell 7), so a deny rule on a cmdlet is not passed by its short name (spec/judgments.md).
/// Keys are lower case because matching ignores case.
const CMDLET_ALIASES: &[(&str, &[&str])] = &[
    ("Get-ChildItem", &["gci", "ls", "dir"]),
    ("Remove-Item", &["rm", "del", "erase", "ri", "rd", "rmdir"]),
    ("Copy-Item", &["cp", "copy", "cpi"]),
    ("Move-Item", &["mv", "move", "mi"]),
    ("Rename-Item", &["ren", "rni"]),
    ("Get-Content", &["cat", "gc", "type"]),
    ("Set-Content", &["sc"]),
    ("Add-Content", &["ac"]),
    ("Clear-Content", &["clc"]),
    ("New-Item", &["ni"]),
    ("Set-Item", &["si"]),
    ("Clear-Item", &["cli"]),
    ("Get-Item", &["gi"]),
    ("Invoke-Item", &["ii"]),
    ("Set-ItemProperty", &["sp"]),
    ("Remove-ItemProperty", &["rp"]),
    ("Copy-ItemProperty", &["cpp"]),
    ("Set-Location", &["cd", "chdir", "sl"]),
    ("Get-Location", &["pwd", "gl"]),
    ("Push-Location", &["pushd"]),
    ("Pop-Location", &["popd"]),
    ("Invoke-Expression", &["iex"]),
    ("Invoke-Command", &["icm"]),
    ("Invoke-WebRequest", &["iwr", "curl", "wget"]),
    ("Invoke-RestMethod", &["irm"]),
    ("Invoke-History", &["r", "ihy"]),
    ("Start-Process", &["start", "saps"]),
    ("Stop-Process", &["kill", "spps"]),
    ("Get-Process", &["ps", "gps"]),
    ("Start-Job", &["sajb"]),
    ("Start-Service", &["sasv"]),
    ("Stop-Service", &["spsv"]),
    ("Set-Variable", &["set", "sv"]),
    ("Remove-Variable", &["rv"]),
    ("Set-Alias", &["sal"]),
    ("New-Alias", &["nal"]),
    ("New-PSDrive", &["ndr", "mount"]),
    ("Remove-PSDrive", &["rdr"]),
    ("Import-Module", &["ipmo"]),
    ("Write-Output", &["echo", "write"]),
    ("Select-String", &["sls"]),
    ("Tee-Object", &["tee"]),
    ("Start-Sleep", &["sleep"]),
];

/// Each alias and the cmdlet it stands for, in the table's order.
pub static POWERSHELL_ALIASES: LazyLock<IndexMap<&'static str, &'static str>> =
    LazyLock::new(|| {
        CMDLET_ALIASES
            .iter()
            .flat_map(|(cmdlet, aliases)| aliases.iter().map(move |alias| (*alias, *cmdlet)))
            .collect()
    });
