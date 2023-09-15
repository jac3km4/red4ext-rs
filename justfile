set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]
set dotenv-load

DEFAULT_GAME_DIR := join("C:\\", "Program Files (x86)", "Steam", "steamapps", "common", "Cyberpunk 2077")

game_dir := env_var_or_default("GAME_DIR", DEFAULT_GAME_DIR)

mod_name           := "example"
bin_name           := "example.dll"
log_name           := "example.log"

red4ext_repo_dir   := join(".", "target")
red4ext_game_dir   := join(game_dir, "red4ext", "plugins")

redscript_repo_dir := join(".", "example", "reds")
redscript_game_dir := join(game_dir, "r6", "scripts")

# list all commands
_default:
  @just --list --unsorted
  @echo "⚠️  on Windows, paths defined in .env must be double-escaped:"
  @echo 'e.g. GAME_DIR="C:\\\\path\\\\to\\\\your\\\\game\\\\folder"'

alias r := hot-reload

# copy .reds files to game folder (can be hot-reloaded in-game)
hot-reload:
 @if (Test-Path '{{ join(redscript_game_dir, mod_name) }}') { \
    Write-Host "Folder {{ join(redscript_game_dir, mod_name) }} already exist"; \
 } else { \
    New-Item '{{ join(redscript_game_dir, mod_name) }}' -ItemType Directory; \
    Write-Host "Created folder at {{ join(redscript_game_dir, mod_name) }}"; \
 }
 cp -Recurse -Force '{{redscript_repo_dir}}' '{{ join(redscript_game_dir, mod_name) }}';

alias i := install

# copy all files to game folder (before launching the game)
install target='release':
 @if (-NOT('{{target}}' -EQ 'debug') -AND -NOT('{{target}}' -EQ 'release')) { \
   Write-Host "target can only be 'debug' or 'release' (default to 'release')"; exit 1; \
 }
 @if ('{{target}}' -EQ 'debug') { cargo build; } else { cargo build --release; }
 @if (Test-Path '{{ join(red4ext_game_dir, mod_name) }}') { \
   Write-Host "Folder {{ join(red4ext_game_dir, mod_name) }} already exist"; \
 } else { \
   New-Item '{{ join(red4ext_game_dir, mod_name) }}' -ItemType Directory; \
 }
 cp -Force '{{ join(red4ext_repo_dir, target, bin_name) }}' '{{ join(red4ext_game_dir, mod_name, bin_name) }}';
 @just hot-reload

dev: (install 'debug')

# remove all files from game folder
uninstall:
 @if (Test-Path '{{ join(red4ext_game_dir, mod_name) }}') { \
    Remove-Item -Recurse -Force '{{ join(red4ext_game_dir, mod_name) }}'; \
 }
 @if (Test-Path '{{ join(redscript_game_dir, mod_name) }}') { \
    Remove-Item -Recurse -Force '{{ join(redscript_game_dir, mod_name) }}'; \
 }

# display red4ext logs
logs:
 @if (Test-Path '{{ join(game_dir, "red4ext", "logs", "red4ext.log") }}') { \
    Write-Host ">>> red4ext.log"; \
    type '{{ join(game_dir, "red4ext", "logs", "red4ext.log") }}' \
 } else { \
    Write-Host ">>> red4ext.log: missing file" \
 }
 @Write-Host "";
 @if (Test-Path '{{ join(game_dir, "red4ext", "logs", log_name) }}') { \
    Write-Host ">>> {{log_name}}"; \
    type '{{ join(game_dir, "red4ext", "logs", log_name) }}' \
 } else { \
    Write-Host ">>> {{log_name}}: missing file" \
 }
