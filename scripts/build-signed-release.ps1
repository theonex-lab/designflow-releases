param([switch]$Bootstrap)

# Run with PowerShell 7. This builds locally and does not publish to GitHub.
$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    $keys = Join-Path $env:LOCALAPPDATA 'DesignFlowSigning'
    $private = Join-Path $keys 'update-private.pem'
    $public = Join-Path $keys 'update-public.der'
    if (!(Test-Path -LiteralPath $private) -or !(Test-Path -LiteralPath $public)) {
        throw 'As chaves de assinatura do DesignFlow precisam estar em DesignFlowSigning.'
    }
    $rsa = [Security.Cryptography.RSA]::Create()
    try {
        $rsa.ImportFromPem([IO.File]::ReadAllText($private))
        if ([Convert]::ToBase64String($rsa.ExportRSAPublicKey()) -cne [Convert]::ToBase64String([IO.File]::ReadAllBytes($public))) {
            throw 'A chave pública não corresponde à chave privada.'
        }
    } finally { $rsa.Dispose() }
    $versionLine = Select-String -LiteralPath 'Cargo.toml' -Pattern '^version = "([0-9]+\.[0-9]+\.[0-9]+)"$' | Select-Object -First 1
    if (!$versionLine) { throw 'Versão do DesignFlow inválida em Cargo.toml.' }
    $version = $versionLine.Matches[0].Groups[1].Value
    $previous = $env:DESIGNFLOW_UPDATE_PUBLIC_KEY
    try {
        $env:DESIGNFLOW_UPDATE_PUBLIC_KEY = $public
        cargo build -p designflow --bin designflow --locked
        if ($LASTEXITCODE -ne 0) { throw 'Falha ao compilar o DesignFlow.' }
    } finally { $env:DESIGNFLOW_UPDATE_PUBLIC_KEY = $previous }
    if ($Bootstrap) {
        & "$PSScriptRoot/build-setup.ps1"
    } else {
        $output = "target/distribution/release-$version"
        $temp = New-Item -ItemType Directory -Path (Join-Path ([IO.Path]::GetTempPath()) ('designflow-build-' + [guid]::NewGuid()))
        $exe = Join-Path $temp.FullName 'DesignFlow.exe'
        try {
            $objcopy = Join-Path (rustc --print sysroot) 'lib/rustlib/x86_64-pc-windows-gnu/bin/rust-objcopy.exe'
            & $objcopy --strip-debug --subsystem windows target/debug/designflow.exe $exe
            if ($LASTEXITCODE -ne 0) { throw 'Falha ao preparar o executável da atualização.' }
            & "$PSScriptRoot/sign-update.ps1" -Version $version -Executable $exe -PrivateKey $private -OutputDirectory $output
            Get-ChildItem -LiteralPath $output -File
        } finally {
            Remove-Item -LiteralPath $exe -ErrorAction SilentlyContinue
            Remove-Item -LiteralPath $temp.FullName -ErrorAction SilentlyContinue
        }
    }
} finally { Pop-Location }
