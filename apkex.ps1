if ($MyInvocation.MyCommand.CommandType -eq "ExternalScript")
{ 
   $SCRIPT_DIR = Split-Path -Parent -Path $MyInvocation.MyCommand.Definition 
}
else
{ 
   $SCRIPT_DIR = Split-Path -Parent -Path ([Environment]::GetCommandLineArgs()[0]) 
   if (!$SCRIPT_DIR){ $SCRIPT_DIR = "." } 
}

# echo SCRIPT_DIR
Write-Host "Script directory: $SCRIPT_DIR"

$GLOBAL_PYTHON = Get-Command python3.exe -ErrorAction SilentlyContinue
if ($null -eq $GLOBAL_PYTHON) {
    $GLOBAL_PYTHON = Get-Command python.exe -ErrorAction SilentlyContinue
    if ($null -eq $GLOBAL_PYTHON) {
        Write-Host "Python is not installed"
        exit 1
    } else {
        $PYTHON_VERSION = & $GLOBAL_PYTHON --version
        if (-not $PYTHON_VERSION.Contains("3.")) {
            Write-Host "Python 3 is not installed"
            exit 1
        }
    }
} else {
    $PYTHON_VERSION = & $GLOBAL_PYTHON --version
    if (-not $PYTHON_VERSION.Contains("3.")) {
        Write-Host "Python 3 is not installed"
        exit 1
    }
}

Write-Host "Using global python: $($GLOBAL_PYTHON.Source) $PYTHON_VERSION"

$VENV_DIR = "$SCRIPT_DIR\venv"
$ACTIVATE = "$VENV_DIR\Scripts\Activate.ps1"

if (-not (Test-Path $VENV_DIR)) {
    # Take action if $DIR does not exist. #
    New-Item -ItemType Directory -Path $VENV_DIR | Out-Null
    & $GLOBAL_PYTHON -m venv $VENV_DIR
}
else {
    Write-Host "Virtual environment folder already exists"
}

. $ACTIVATE
pip install -r $SCRIPT_DIR\requirements.txt

python $SCRIPT_DIR"\apkex.py" $args