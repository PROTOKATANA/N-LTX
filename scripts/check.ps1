#!/usr/bin/env pwsh
# Puerta de calidad: se ejecuta antes de cada commit. Comprueba todos los lenguajes del repositorio.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Set-Location (git rev-parse --show-toplevel)

function Assert-Herramienta {
    param([string] $Nombre)
    if (-not (Get-Command $Nombre -ErrorAction SilentlyContinue)) {
        throw "Falta la herramienta '$Nombre'"
    }
}

function Write-Seccion {
    param([string] $Titulo)
    Write-Output ''
    Write-Output "== $Titulo =="
}

# Ejecuta un programa externo y falla si devuelve un código distinto de 0.
function Invoke-Paso {
    param([string] $Programa, [string[]] $Argumentos)
    & $Programa @Argumentos
    if ($LASTEXITCODE -ne 0) {
        throw "Falló: $Programa $($Argumentos -join ' ')"
    }
}

Write-Seccion 'Rust'
Assert-Herramienta 'cargo'
Assert-Herramienta 'cargo-audit'
Invoke-Paso 'cargo' @('fmt', '--all', '--', '--check')
Invoke-Paso 'cargo' @('clippy', '--all-targets', '--all-features', '--', '-D', 'warnings')
Invoke-Paso 'cargo' @('test', '--all-features')
Invoke-Paso 'cargo' @('build', '--release')
Invoke-Paso 'cargo' @('audit')

Write-Seccion 'Dart/Flutter'
Assert-Herramienta 'flutter'
Assert-Herramienta 'dart'
Push-Location 'ejemplo'
try {
    # antes del formato: dart format lee analysis_options.yaml, que necesita los paquetes resueltos
    Invoke-Paso 'flutter' @('pub', 'get')
    Invoke-Paso 'dart' @('format', '--output=none', '--set-exit-if-changed', 'lib')
    Invoke-Paso 'flutter' @('analyze', '--fatal-infos')
}
finally {
    Pop-Location
}

Write-Seccion 'Secretos'
Assert-Herramienta 'gitleaks'
Invoke-Paso 'gitleaks' @('git', '--redact', '--no-banner', '.')

Write-Output ''
Write-Output 'Puerta de calidad superada.'
