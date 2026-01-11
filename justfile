set shell := ["powershell.exe", "-ExecutionPolicy", "Bypass", "-Command"]

style:
    @Write-Host "start style the rust code"
    cd src-tauri; cargo clippy --fix --allow-dirty; cargo fmt --all
    @Write-Host "style finished"