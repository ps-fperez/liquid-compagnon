# Crée la clé de signature Android de Liquid Compagnon et l'enregistre dans les secrets GitHub du dépôt.
# Le mot de passe est saisi au clavier : il n'est ni affiché, ni écrit sur disque en clair.
#
# IMPORTANT : sauvegarde le fichier de clé créé (hors du PC). Sans lui, une nouvelle version de
# l'application ne pourra plus être installée par-dessus l'ancienne.
param(
    [string]$Repo = 'ps-fperez/liquid-compagnon'
)
$ErrorActionPreference = 'Stop'

$dir = Join-Path $env:USERPROFILE '.liquid-compagnon'
$keystore = Join-Path $dir 'release.jks'
$alias = 'liquid-compagnon'

if (Test-Path $keystore) {
    throw "Une clé existe déjà : $keystore. La remplacer empêcherait les mises à jour de l'application installée."
}

function Read-Secret([string]$prompt) {
    $secure = Read-Host $prompt -AsSecureString
    $bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
    try { [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr) }
    finally { [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr) }
}

$password = Read-Secret 'Mot de passe de la clé (12 caractères minimum)'
if ($password.Length -lt 12) { throw '12 caractères minimum.' }
if ($password -ne (Read-Secret 'Confirme le mot de passe')) { throw 'Les mots de passe ne correspondent pas.' }

$keytool = if ($env:JAVA_HOME) { Join-Path $env:JAVA_HOME 'bin\keytool.exe' } else { 'keytool' }
New-Item -ItemType Directory -Force $dir | Out-Null

$env:LC_KEYSTORE_PASSWORD = $password
try {
    & $keytool -genkeypair -keystore $keystore -storetype PKCS12 -alias $alias `
        -keyalg RSA -keysize 4096 -validity 10000 -dname 'CN=Liquid Compagnon' `
        -storepass:env LC_KEYSTORE_PASSWORD -keypass:env LC_KEYSTORE_PASSWORD
    if ($LASTEXITCODE -ne 0) { throw 'keytool a échoué.' }
}
finally {
    Remove-Item Env:LC_KEYSTORE_PASSWORD -ErrorAction SilentlyContinue
}

$keystoreB64 = [Convert]::ToBase64String([IO.File]::ReadAllBytes($keystore))
gh secret set ANDROID_KEYSTORE_BASE64 --repo $Repo --body $keystoreB64
gh secret set ANDROID_KEYSTORE_PASSWORD --repo $Repo --body $password
gh secret set ANDROID_KEY_ALIAS --repo $Repo --body $alias
$password = $null

Write-Host "Clé créée : $keystore" -ForegroundColor Green
Write-Host 'Secrets GitHub enregistrés. Sauvegarde ce fichier hors du PC (cloud perso, clé USB).' -ForegroundColor Yellow
